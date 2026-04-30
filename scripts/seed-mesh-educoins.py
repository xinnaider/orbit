"""Seed the Mesh DB with an educoins bugfix pipeline.

Idempotent: running twice creates duplicates (since rows have auto-id).
Intended for quick end-to-end testing of the pipeline runner.
"""

import os
import sqlite3
import sys

DB = os.path.join(os.environ["APPDATA"], "com.josefernando.orbit", "agent-dashboard.db")

FLOOR_NAME = "Educoins Bugfix"
GRAPH_NAME = "Pipeline Fix Senha"

TEMPLATES = [
    {
        "name": "Investigador",
        "pre_prompt": (
            "Voce e o INVESTIGADOR. Tarefa: descobrir por que a edicao de usuario no "
            "projeto educoins NAO esta trocando a senha. "
            "Explore educoins-back (NestJS) e educoins-front (Next.js): procure controllers, "
            "services e use-cases de users; endpoints PATCH/PUT /users; validacoes; DTOs; "
            "e o fluxo do form de edicao no front. Use Read, Grep, Glob livremente. "
            "Entregue um relatorio com: (1) arquivos envolvidos com caminhos absolutos, "
            "(2) fluxo completo front -> back -> banco, (3) hipotese do bug com trechos de codigo "
            "que comprovam. NAO altere nenhum arquivo. Seja objetivo."
        ),
        "provider": "claude-code",
        "use_worktree": 0,
    },
    {
        "name": "Planejador",
        "pre_prompt": (
            "Voce e o PLANEJADOR. Um INVESTIGADOR fez o diagnostico de um bug na troca de senha "
            "durante a edicao de usuario do educoins. Voce vai receber o relatorio dele "
            "como proxima mensagem. "
            "Responda com um plano de correcao: arquivos a modificar (caminhos absolutos), "
            "o que alterar em cada um (idealmente citando linhas ou funcoes), ordem das mudancas, "
            "e como validar (quais testes existem e se precisa criar novos). "
            "NAO altere codigo. Seja especifico."
        ),
        "provider": "claude-code",
        "use_worktree": 0,
    },
    {
        "name": "Implementador",
        "pre_prompt": (
            "Voce e o IMPLEMENTADOR. Um PLANEJADOR escreveu um plano de correcao para o bug "
            "de troca de senha na edicao de usuario do educoins. Voce vai receber o plano como "
            "proxima mensagem. "
            "Aplique as mudancas no codigo com Edit/Write. Depois de cada arquivo editado, "
            "rode os testes relevantes (`cd educoins-back && npm run test:unit` ou no front). "
            "Reporte o que foi alterado, quais testes passaram e se algum quebrou "
            "(nesse caso, corrija)."
        ),
        "provider": "claude-code",
        "use_worktree": 1,
    },
]


def main():
    if not os.path.exists(DB):
        print(f"DB nao encontrado: {DB}", file=sys.stderr)
        print("Abra o Orbit ao menos uma vez pra criar o banco.", file=sys.stderr)
        sys.exit(1)

    conn = sqlite3.connect(DB)
    conn.execute("PRAGMA foreign_keys=ON")
    cur = conn.cursor()

    # 1. Floor
    cur.execute("INSERT INTO floors (name) VALUES (?)", (FLOOR_NAME,))
    floor_id = cur.lastrowid
    print(f"floor #{floor_id}: {FLOOR_NAME}")

    # 2. Templates
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

    # 3. Graph
    cur.execute(
        "INSERT INTO graphs (floor_id, name, provider) VALUES (?, ?, ?)",
        (floor_id, GRAPH_NAME, "claude-code"),
    )
    graph_id = cur.lastrowid
    print(f"graph #{graph_id}: {GRAPH_NAME}")

    # 4. Nodes
    positions = [(80, 200), (500, 200), (920, 200)]
    display_names = ["A: Investigar", "B: Planejar", "C: Implementar"]
    node_ids = []
    for i, (tid, (x, y), dname) in enumerate(zip(template_ids, positions, display_names)):
        cur.execute(
            "INSERT INTO graph_nodes (graph_id, template_id, display_name, x, y) "
            "VALUES (?, ?, ?, ?, ?)",
            (graph_id, tid, dname, x, y),
        )
        node_ids.append(cur.lastrowid)
        print(f"  node #{cur.lastrowid}: {dname}")

    # 5. Edges A->B, B->C
    edges = [(node_ids[0], node_ids[1]), (node_ids[1], node_ids[2])]
    for from_id, to_id in edges:
        cur.execute(
            "INSERT INTO graph_edges (graph_id, from_node_id, to_node_id) VALUES (?, ?, ?)",
            (graph_id, from_id, to_id),
        )
        print(f"  edge {from_id} -> {to_id}")

    # 6. Set entry node
    cur.execute(
        "UPDATE graphs SET entry_node_id = ? WHERE id = ?",
        (node_ids[0], graph_id),
    )

    conn.commit()
    conn.close()

    print()
    print("Seed concluido. No app, vai em Mesh -> Educoins Bugfix -> 'Pipeline Fix Senha'.")
    print("Se o app ja estava aberto, troca de floor e volta (ou reabre).")


if __name__ == "__main__":
    main()
