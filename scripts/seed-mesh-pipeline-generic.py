"""Replace existing Mesh pipeline with generic role-based templates for Next.js + NestJS stacks.

Runs UPDATE on existing rows when possible (no duplicate floors), otherwise seeds fresh.
"""

import os
import sqlite3
import sys

DB = os.path.join(os.environ["APPDATA"], "com.josefernando.orbit", "agent-dashboard.db")

FLOOR_NAME = "Pipeline (3 agentes)"
GRAPH_NAME = "Pipeline generico"

TEMPLATES = [
    {
        "name": "Investigador",
        "pre_prompt": """Voce e o INVESTIGADOR.

CONTEXTO: Voce trabalha num monorepo Next.js + NestJS. A estrutura comum e:
- `<projeto>-back/` (NestJS + TypeScript) com controllers, services, use-cases, DTOs em `src/<modulo>/dto/`, repositorios (Firebase, Postgres, etc.)
- `<projeto>-front/` (Next.js + TypeScript + React 19) com componentes em `src/components/<modulo>/`, forms via react-hook-form + zod, clientes HTTP em `src/services/`

PROCEDIMENTO:
1. Voce recebe uma TASK do usuario (na proxima mensagem).
2. Explore o codigo relevante em front E back (Glob, Grep, Read livremente).
3. Mapeie o fluxo completo: UI -> servico front -> API back -> use-case/service -> repositorio -> banco/store.
4. Se for BUG: formule hipotese citando trechos de codigo como evidencia.
5. Se for FEATURE: liste o que existe e o que falta.

ENTREGA (relatorio objetivo, direto, sem enrolacao):
- Arquivos envolvidos (caminhos relativos ao workspace)
- Fluxo de execucao resumido
- Para bug: causa provavel + evidencia (trecho de codigo ou linha)
- Para feature: gaps identificados
- Perguntas abertas se faltar contexto

RESTRICOES:
- NAO altere arquivos.
- NAO commit.
- Use so Read/Grep/Glob/Bash read-only (ex: git log, cat, ls).""",
        "provider": "claude-code",
        "use_worktree": 0,
    },
    {
        "name": "Planejador",
        "pre_prompt": """Voce e o PLANEJADOR.

CONTEXTO: Voce trabalha num monorepo Next.js + NestJS. Um INVESTIGADOR mapeou o problema/feature e te entrega o relatorio na proxima mensagem.

PROCEDIMENTO:
1. Leia o relatorio do investigador com atencao.
2. Transforme em um plano executavel de correcao/implementacao.
3. Se precisar, abra arquivos para confirmar antes de planejar (Read livremente).
4. Pense em ordem logica: backend geralmente antes de front (DTO -> service -> controller -> front).

ENTREGA (plano estruturado):
- Lista ordenada de arquivos a modificar/criar (caminhos relativos)
- O que mudar em cada arquivo (trechos "antes -> depois" quando viavel, ou descricao precisa)
- Dependencias entre mudancas
- Como validar: quais testes ja existem, quais criar, comandos exatos (`cd <projeto>-back && npm run test:unit`, etc.)
- Riscos conhecidos (breaking changes, migracoes de DB, etc.)

RESTRICOES:
- NAO altere codigo.
- NAO escreva codigo final — so plano.
- Seja especifico; plano vago nao ajuda o implementador.""",
        "provider": "claude-code",
        "use_worktree": 0,
    },
    {
        "name": "Implementador",
        "pre_prompt": """Voce e o IMPLEMENTADOR.

CONTEXTO: Voce trabalha num monorepo Next.js + NestJS. Um PLANEJADOR te entrega o plano na proxima mensagem.

PROCEDIMENTO:
1. Execute as mudancas na ordem descrita no plano (Edit/Write).
2. Apos cada grupo logico de mudancas, rode os testes relevantes:
   - Backend: `cd <projeto>-back && npm run test:unit` (ou `test` se nao tiver split)
   - Frontend: `cd <projeto>-front && npm test`
3. Se teste quebrar: diagnostique, corrija, rode de novo.
4. Se o plano nao cobrir um edge case, use seu julgamento e REPORTE o desvio.
5. Nao prosseguir pro proximo arquivo ate o atual passar nos testes relevantes.

ENTREGA (resumo final):
- Arquivos alterados (com caminhos)
- Status dos testes: quais passaram, quais quebraram e como foram corrigidos
- Desvios do plano (se houver)
- Proximos passos sugeridos (migrations, docs, etc.)

RESTRICOES:
- NAO faca `git commit` / `git push` — deixa o usuario revisar e commitar depois.
- Se precisar de comando destrutivo em DB/infra, PARE e reporte.
- Mantenha o coverage alto (o projeto costuma ter thresholds no jest.config).""",
        "provider": "claude-code",
        "use_worktree": 1,
    },
]


def main():
    if not os.path.exists(DB):
        print(f"DB nao encontrado: {DB}", file=sys.stderr)
        sys.exit(1)

    conn = sqlite3.connect(DB)
    conn.execute("PRAGMA foreign_keys=ON")
    cur = conn.cursor()

    # Remove any existing Mesh seed floors by name (generic OR old educoins one)
    old_names = ("Educoins Bugfix", FLOOR_NAME)
    for nm in old_names:
        cur.execute("SELECT id FROM floors WHERE name = ?", (nm,))
        for (fid,) in cur.fetchall():
            print(f"removendo floor #{fid} ({nm}) e dependencias…")
            cur.execute("DELETE FROM floors WHERE id = ?", (fid,))

    # Create fresh floor
    cur.execute("INSERT INTO floors (name) VALUES (?)", (FLOOR_NAME,))
    floor_id = cur.lastrowid
    print(f"floor #{floor_id}: {FLOOR_NAME}")

    template_ids = []
    for t in TEMPLATES:
        cur.execute(
            "INSERT INTO agent_templates (floor_id, name, pre_prompt, provider, use_worktree) "
            "VALUES (?, ?, ?, ?, ?)",
            (floor_id, t["name"], t["pre_prompt"], t["provider"], t["use_worktree"]),
        )
        tid = cur.lastrowid
        template_ids.append(tid)
        print(f"  template #{tid}: {t['name']}")

    cur.execute(
        "INSERT INTO graphs (floor_id, name, provider) VALUES (?, ?, ?)",
        (floor_id, GRAPH_NAME, "claude-code"),
    )
    graph_id = cur.lastrowid
    print(f"graph #{graph_id}: {GRAPH_NAME}")

    positions = [(80, 200), (500, 200), (920, 200)]
    display_names = ["Investigador", "Planejador", "Implementador"]
    node_ids = []
    for tid, (x, y), dname in zip(template_ids, positions, display_names):
        cur.execute(
            "INSERT INTO graph_nodes (graph_id, template_id, display_name, x, y) "
            "VALUES (?, ?, ?, ?, ?)",
            (graph_id, tid, dname, x, y),
        )
        node_ids.append(cur.lastrowid)
        print(f"  node #{cur.lastrowid}: {dname}")

    for from_id, to_id in [(node_ids[0], node_ids[1]), (node_ids[1], node_ids[2])]:
        cur.execute(
            "INSERT INTO graph_edges (graph_id, from_node_id, to_node_id) VALUES (?, ?, ?)",
            (graph_id, from_id, to_id),
        )
        print(f"  edge {from_id} -> {to_id}")

    cur.execute("UPDATE graphs SET entry_node_id = ? WHERE id = ?", (node_ids[0], graph_id))

    conn.commit()
    conn.close()

    print()
    print("Seed generico concluido.")
    print("No app: Mesh -> 'Pipeline (3 agentes)' -> 'Pipeline generico'.")
    print("Cola a task concreta (ex: 'edicao de usuario do educoins nao troca a senha')")
    print("e clica Start pipeline. Lembre de apontar o cwd pra pasta raiz do projeto desejado.")


if __name__ == "__main__":
    main()
