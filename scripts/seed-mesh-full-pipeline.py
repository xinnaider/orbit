"""Wipe ALL existing Mesh data and seed a complete 5-agent pipeline with skills + notes per agent.

Pipeline: Investigador -> Planejador -> Implementador -> Testador -> Revisor
Each agent gets 2 skills (read-only context) + 1 note (editable context) wired in.
"""

import os
import sqlite3
import sys

DB = os.path.join(os.environ["APPDATA"], "com.josefernando.orbit", "agent-dashboard.db")

FLOOR_NAME = "Pipeline completa (5 agentes)"
GRAPH_NAME = "Pipeline com skills + notas"

# 5 agents, top-down by execution order. (skills, note_seed) attached to each.
AGENTS = [
    {
        "name": "Investigador",
        "skills": ["read-codebase", "diagnose"],
        "note_name": "Investigador — escopo",
        "note_seed": "## Escopo da investigação\n\n- Foco no monorepo Next.js + NestJS.\n- Ignorar `node_modules`, `dist`, `.next`.\n- Anotar aqui os caminhos suspeitos enquanto investiga.\n",
        "pre_prompt": """Voce e o INVESTIGADOR, primeiro agente do pipeline.

CONTEXTO: Monorepo Next.js + NestJS. A estrutura comum e:
- `<projeto>-back/` (NestJS + TypeScript) com controllers, services, use-cases, DTOs em `src/<modulo>/dto/`
- `<projeto>-front/` (Next.js + TypeScript + React 19) com componentes em `src/components/<modulo>/`, forms via react-hook-form + zod, clientes HTTP em `src/services/`

PROCEDIMENTO:
1. Voce recebe uma TASK do usuario (na proxima mensagem).
2. Use as skills conectadas (`read-codebase`, `diagnose`) para varrer o codigo.
3. Mapeie o fluxo completo: UI -> servico front -> API back -> use-case/service -> repositorio -> banco/store.
4. Se for BUG: formule hipotese citando trechos de codigo como evidencia.
5. Se for FEATURE: liste o que existe e o que falta.
6. Consulte a nota anexa para o escopo da investigacao.

ENTREGA (relatorio objetivo, direto):
- Arquivos envolvidos (caminhos relativos ao workspace)
- Fluxo de execucao resumido
- Para bug: causa provavel + evidencia (trecho de codigo ou linha)
- Para feature: gaps identificados
- Perguntas abertas se faltar contexto

RESTRICOES:
- NAO altere arquivos.
- NAO commit.
- Use so Read/Grep/Glob/Bash read-only (ex: git log, cat, ls).""",
        "use_worktree": 0,
    },
    {
        "name": "Planejador",
        "skills": ["plan", "decide"],
        "note_name": "Planejador — convencoes",
        "note_seed": "## Convencoes do projeto\n\n- Backend antes do front (DTO -> service -> controller -> client front).\n- Migrations explicitas, nunca `synchronize: true`.\n- Atualize esta nota com decisoes que valem para os proximos planos.\n",
        "pre_prompt": """Voce e o PLANEJADOR, segundo agente do pipeline.

CONTEXTO: O INVESTIGADOR mapeou o problema/feature. O relatorio dele chega na proxima mensagem.

PROCEDIMENTO:
1. Leia o relatorio do investigador com atencao.
2. Use as skills conectadas (`plan`, `decide`) para estruturar o plano.
3. Consulte a nota anexa para convencoes que devem ser respeitadas.
4. Transforme em um plano executavel.
5. Pense em ordem logica: backend geralmente antes de front.

ENTREGA (plano estruturado):
- Lista ordenada de arquivos a modificar/criar (caminhos relativos)
- O que mudar em cada arquivo (trechos "antes -> depois" quando viavel)
- Dependencias entre mudancas
- Como validar: testes existentes + testes a criar + comandos exatos
- Riscos conhecidos (breaking changes, migracoes, etc.)

RESTRICOES:
- NAO altere codigo.
- NAO escreva codigo final — so plano.
- Seja especifico; plano vago nao ajuda o implementador.""",
        "use_worktree": 0,
    },
    {
        "name": "Implementador",
        "skills": ["implement", "clean-code"],
        "note_name": "Implementador — checklist",
        "note_seed": "## Checklist de implementacao\n\n- [ ] Arquivos do plano alterados\n- [ ] Imports atualizados\n- [ ] Sem `any` novos\n- [ ] Testes locais rodaram\n- [ ] Sem console.log esquecido\n",
        "pre_prompt": """Voce e o IMPLEMENTADOR, terceiro agente do pipeline.

CONTEXTO: O PLANEJADOR te entrega o plano na proxima mensagem.

PROCEDIMENTO:
1. Use as skills conectadas (`implement`, `clean-code`) para escrever o codigo.
2. Consulte a nota anexa para garantir o checklist.
3. Execute as mudancas na ordem descrita no plano (Edit/Write).
4. Apos cada grupo logico de mudancas, rode os testes relevantes:
   - Backend: `cd <projeto>-back && npm run test:unit` (ou `test`)
   - Frontend: `cd <projeto>-front && npm test`
5. Se teste quebrar: diagnostique, corrija, rode de novo.
6. Se o plano nao cobrir um edge case, use seu julgamento e REPORTE o desvio.
7. Nao prosseguir pro proximo arquivo ate o atual passar nos testes relevantes.

ENTREGA (resumo final):
- Arquivos alterados (com caminhos)
- Status dos testes: quais passaram, quais quebraram e como foram corrigidos
- Desvios do plano (se houver)
- Proximos passos sugeridos (migrations, docs, etc.)

RESTRICOES:
- IMPLEMENTE DE VERDADE — nao apenas descreva. Execute Edit/Write.
- NAO faca `git commit` / `git push`.
- Se precisar de comando destrutivo em DB/infra, PARE e reporte.
- Mantenha o coverage alto.""",
        "use_worktree": 1,
    },
    {
        "name": "Testador",
        "skills": ["write-tests", "check-tests"],
        "note_name": "Testador — politica",
        "note_seed": "## Politica de testes\n\n- Unit tests sao obrigatorios para use-cases e services.\n- E2E so quando o fluxo cruza camadas (front -> back -> banco).\n- Sem mock de DB nos testes de integracao.\n- Falha em teste = bloqueia merge.\n",
        "pre_prompt": """Voce e o TESTADOR, quarto agente do pipeline.

CONTEXTO: O IMPLEMENTADOR alterou o codigo. O resumo dele chega na proxima mensagem.

PROCEDIMENTO:
1. Leia o resumo de mudancas do implementador.
2. Use as skills conectadas (`write-tests`, `check-tests`) para validar a cobertura.
3. Consulte a nota anexa para politica de testes do projeto.
4. Para cada arquivo alterado:
   a. Verifique se ja existe teste cobrindo o caminho mudado.
   b. Se faltar: ESCREVA o teste (unit ou integracao conforme apropriado).
   c. Rode os testes relevantes do modulo.
5. Garanta cobertura para casos de borda mencionados no plano.

ENTREGA:
- Lista de testes criados/alterados (com caminho)
- Resultado: passaram, falharam (com motivo)
- Cobertura aproximada do que foi tocado
- Casos nao cobertos (se houver) e justificativa

RESTRICOES:
- Use Edit/Write para criar/ajustar testes.
- NAO altere codigo de producao — so testes.
- Se um teste falha por bug real (nao por ele estar errado), REPORTE com evidencia.""",
        "use_worktree": 1,
    },
    {
        "name": "Revisor",
        "skills": ["review-code", "check-bugs"],
        "note_name": "Revisor — criterios",
        "note_seed": "## Criterios de review\n\n- Bug real ou risco concreto? (nao 'preferencia de estilo')\n- Quebra de contrato API/DTO?\n- Estado/efeito colateral nao previsto?\n- Performance regressao mensuravel?\n- Falha de seguranca (input validation, autenticacao, secrets)?\n",
        "pre_prompt": """Voce e o REVISOR, ultimo agente do pipeline.

CONTEXTO: O TESTADOR validou a cobertura. O resumo dele chega na proxima mensagem.

PROCEDIMENTO:
1. Leia o resumo do testador (e tambem o output do implementador, se necessario).
2. Use as skills conectadas (`review-code`, `check-bugs`) para fazer um review serio.
3. Consulte a nota anexa para os criterios de review.
4. Releia os arquivos alterados (Read).
5. Aponte problemas REAIS — nao preferencias estilisticas.
6. Categorize: BLOCKER, MAJOR, MINOR, NITPICK.

ENTREGA (relatorio final do pipeline):
- Resumo executivo (1 paragrafo)
- Issues por categoria, com arquivo:linha + sugestao concreta
- Veredito: APROVADO / APROVADO COM RESSALVAS / BLOQUEADO
- Proximos passos antes de merge

RESTRICOES:
- NAO altere codigo. So aponte.
- Seja especifico; reviews vagos nao ajudam.
- Se precisar de mais contexto, pergunte; nao chute.""",
        "use_worktree": 0,
    },
]


def main():
    if not os.path.exists(DB):
        print(f"DB nao encontrado: {DB}", file=sys.stderr)
        sys.exit(1)

    conn = sqlite3.connect(DB)
    conn.execute("PRAGMA foreign_keys=ON")
    cur = conn.cursor()

    # 1. Wipe ALL existing mesh data. `runs` and `run_sessions` reference
    #    graphs/graph_nodes without ON DELETE CASCADE, so clear them first
    #    before the cascading floor delete.
    cur.execute("DELETE FROM run_sessions")
    cur.execute("DELETE FROM runs")
    cur.execute("DELETE FROM mesh_note_contents")
    cur.execute("SELECT id, name FROM floors")
    rows = cur.fetchall()
    for fid, fname in rows:
        print(f"removendo floor #{fid} ({fname}) e tudo dentro...")
        cur.execute("DELETE FROM floors WHERE id = ?", (fid,))

    # 2. Create fresh floor
    cur.execute("INSERT INTO floors (name) VALUES (?)", (FLOOR_NAME,))
    floor_id = cur.lastrowid
    print(f"\nfloor #{floor_id}: {FLOOR_NAME}")

    # 3. Create agent templates
    agent_template_ids = []
    for a in AGENTS:
        cur.execute(
            "INSERT INTO agent_templates (floor_id, name, pre_prompt, provider, use_worktree) "
            "VALUES (?, ?, ?, 'claude-code', ?)",
            (floor_id, a["name"], a["pre_prompt"], a["use_worktree"]),
        )
        tid = cur.lastrowid
        agent_template_ids.append(tid)
        print(f"  agent template #{tid}: {a['name']}")

    # 4. Create skill templates (one per unique skill slug used)
    all_skills = sorted({s for a in AGENTS for s in a["skills"]})
    skill_template_ids = {}
    for slug in all_skills:
        cur.execute(
            "INSERT INTO agent_templates (floor_id, name, pre_prompt, provider, use_worktree) "
            "VALUES (?, ?, ?, 'skill', 0)",
            (floor_id, slug, slug),  # pre_prompt holds the slug for skill templates
        )
        skill_template_ids[slug] = cur.lastrowid
        print(f"  skill template #{cur.lastrowid}: {slug}")

    # 5. Create the system note template (lazy-created by add_note normally)
    cur.execute(
        "INSERT INTO agent_templates (floor_id, name, pre_prompt, provider, use_worktree) "
        "VALUES (?, '__note__', '', 'note', 0)",
        (floor_id,),
    )
    note_template_id = cur.lastrowid
    print(f"  note template #{note_template_id}: __note__")

    # 6. Create the graph
    cur.execute(
        "INSERT INTO graphs (floor_id, name, provider) VALUES (?, ?, 'claude-code')",
        (floor_id, GRAPH_NAME),
    )
    graph_id = cur.lastrowid
    print(f"\ngraph #{graph_id}: {GRAPH_NAME}")

    # 7. Layout: 5 columns @ x=100,420,740,1060,1380; agent y=400; skills above; notes below
    COL_W = 320
    AGENT_Y = 400
    SKILL_Y_TOP = 40
    SKILL_Y_GAP = 110
    NOTE_Y = 720

    agent_node_ids = []
    for i, a in enumerate(AGENTS):
        x = 100 + i * COL_W
        # Agent node
        cur.execute(
            "INSERT INTO graph_nodes (graph_id, template_id, display_name, x, y) "
            "VALUES (?, ?, ?, ?, ?)",
            (graph_id, agent_template_ids[i], a["name"], x, AGENT_Y),
        )
        anid = cur.lastrowid
        agent_node_ids.append(anid)
        print(f"  agent node #{anid} '{a['name']}' @ ({x},{AGENT_Y})")

        # Skill nodes (above the agent)
        for j, slug in enumerate(a["skills"]):
            sx = x + (j * 140) - 70  # spread two skills horizontally
            sy = SKILL_Y_TOP + (j * SKILL_Y_GAP)
            display = f"{slug} [{a['name']}]"
            cur.execute(
                "INSERT INTO graph_nodes (graph_id, template_id, display_name, x, y) "
                "VALUES (?, ?, ?, ?, ?)",
                (graph_id, skill_template_ids[slug], display, sx, sy),
            )
            snid = cur.lastrowid
            print(f"    skill node #{snid} '{slug}' -> {a['name']}")
            cur.execute(
                "INSERT INTO graph_edges (graph_id, from_node_id, to_node_id) VALUES (?, ?, ?)",
                (graph_id, snid, anid),
            )

        # Note node (below the agent)
        note_display = a["note_name"]
        cur.execute(
            "INSERT INTO graph_nodes (graph_id, template_id, display_name, x, y) "
            "VALUES (?, ?, ?, ?, ?)",
            (graph_id, note_template_id, note_display, x, NOTE_Y),
        )
        nnid = cur.lastrowid
        cur.execute(
            "INSERT INTO mesh_note_contents (node_id, name, content) VALUES (?, ?, ?)",
            (nnid, a["note_name"], a["note_seed"]),
        )
        print(f"    note node #{nnid} '{a['note_name']}' -> {a['name']}")
        cur.execute(
            "INSERT INTO graph_edges (graph_id, from_node_id, to_node_id) VALUES (?, ?, ?)",
            (graph_id, nnid, anid),
        )

    # 8. Pipeline edges agent[i] -> agent[i+1]
    for i in range(len(agent_node_ids) - 1):
        cur.execute(
            "INSERT INTO graph_edges (graph_id, from_node_id, to_node_id) VALUES (?, ?, ?)",
            (graph_id, agent_node_ids[i], agent_node_ids[i + 1]),
        )
        print(f"  pipeline edge: {AGENTS[i]['name']} -> {AGENTS[i + 1]['name']}")

    # 9. Set entry node = first agent
    cur.execute(
        "UPDATE graphs SET entry_node_id = ? WHERE id = ?",
        (agent_node_ids[0], graph_id),
    )

    conn.commit()
    conn.close()

    print(
        "\n"
        f"Pronto. Recarregue o app (Ctrl+R) e abra Mesh -> '{FLOOR_NAME}' -> '{GRAPH_NAME}'.\n"
        "Aponte o cwd pra raiz do projeto antes de Start pipeline.\n"
        "Se 'nao implementa' continuar acontecendo, cole o output do Implementador "
        "que eu ajusto o pre_prompt."
    )


if __name__ == "__main__":
    main()
