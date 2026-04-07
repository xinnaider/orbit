# Split Panes — Design Spec

**Data:** 06/04/2026  
**Branch:** `feat/split-panes`  
**Status:** Aprovado

---

## Objetivo

Permitir que o Orbit exiba até 4 sessões do Claude Code simultaneamente em painéis divididos, com drag-and-drop direcional da sidebar para criar as divisões.

---

## Layout do grid

Quatro slots fixos dispostos em 2×2:

```
┌────────┬────────┐
│   TL   │   TR   │
├────────┼────────┤
│   BL   │   BR   │
└────────┴────────┘
```

- Estado inicial: apenas `TL` visível (ocupa todo o espaço central — comportamento idêntico ao atual)
- Painéis são abertos incrementalmente por drag — nunca exibem mais de 4 ao mesmo tempo
- Quando um painel é fechado, o grid se reajusta dando o espaço de volta aos adjacentes

---

## Modelo de dados

**Novo store:** `api/lib/stores/layout.ts`

```typescript
type PaneId = 'tl' | 'tr' | 'bl' | 'br';

interface SplitLayout {
  panes: Record<PaneId, number | null>; // sessionId | null (vazio/oculto)
  visible: PaneId[];                    // slots atualmente abertos
  focused: PaneId;                      // MetaPanel e selectedSessionId seguem este
}

const defaultLayout: SplitLayout = {
  panes: { tl: null, tr: null, bl: null, br: null },
  visible: ['tl'],
  focused: 'tl',
};
```

`selectedSessionId` (store atual) vira um **derived** de `splitLayout`:

```typescript
// api/lib/stores/sessions.ts
import { derived } from 'svelte/store';
import { splitLayout } from './layout';

export const selectedSessionId = derived(
  splitLayout,
  ($l) => $l.panes[$l.focused] ?? null
);
```

Isso preserva a interface existente — Sidebar, MetaPanel e todos os event handlers em `App.svelte` continuam lendo `$selectedSessionId` sem modificação.

---

## Componentes

### Novos

| Componente | Arquivo | Responsabilidade |
|-----------|---------|-----------------|
| `PaneGrid` | `api/components/PaneGrid.svelte` | Renderiza o grid CSS; gerencia drag-over global; orquestra os slots |
| `Pane` | `api/components/Pane.svelte` | Um slot do grid; detecta zonas de drop; renderiza `CentralPanel` ou placeholder `+` |

### Modificados

| Componente | Mudança |
|-----------|---------|
| `App.svelte` | Substitui `<CentralPanel session={selected}>` por `<PaneGrid>`; passa `session` do pane focado para `<MetaPanel>` |
| `Sidebar.svelte` | Adiciona `draggable="true"` em cada item de sessão; emite `sessionId` no `dataTransfer` |

### Sem mudança

`CentralPanel`, `Feed`, `InputBar`, `MetaPanel`, `RightPanel` — nenhum arquivo tocado internamente.

---

## Drag & Drop

### Sidebar → Pane

Cada item de sessão na sidebar recebe `draggable="true"`. No evento `dragstart`:

```javascript
e.dataTransfer.setData('text/plain', String(session.id));
e.dataTransfer.effectAllowed = 'move';
```

### Detecção de zona no `Pane`

O painel divide sua área em 5 zonas calculadas no `dragover` pelo cursor relativo ao bounding rect:

```
┌──────────[top 20%]──────────┐
│                             │
[left 20%]  [center 60%]  [right 20%]
│                             │
└─────────[bottom 20%]────────┘
```

| Zona | Ação no `drop` |
|------|----------------|
| Centro | Substitui `sessionId` do slot atual (sem criar novo painel) |
| Direita | Abre slot adjacente à direita (`TL→TR`, `BL→BR`) |
| Esquerda | Abre slot adjacente à esquerda (`TR→TL`, `BR→BL`) |
| Baixo | Abre slot adjacente abaixo (`TL→BL`, `TR→BR`) |
| Cima | Abre slot adjacente acima (`BL→TL`, `BR→TR`) |

Highlight visual durante `dragover`: borda colorida na direção detectada (`2px solid var(--ac)`).

**Quando não há slot adjacente disponível** (ex: arrastar para direita com TR já aberto), a zona é ignorada — o painel inteiro trata como zona centro.

**Máximo 4 painéis:** se `visible.length === 4`, bordas mostram cursor `not-allowed` e drop nas bordas é ignorado.

---

## Regras de abertura de slots

| Ação | Slot aberto |
|------|------------|
| Drop na borda direita de TL | TR |
| Drop na borda inferior de TL | BL |
| Drop na borda direita de BL | BR |
| Drop na borda inferior de TR | BR |
| Drop na borda esquerda de TR | TL (se fechado) |
| Drop na borda superior de BL | TL (se fechado) |
| Drop na borda superior de BR | TR ou BL (o que estiver aberto) |
| Drop na borda esquerda de BR | BL ou TR (o que estiver aberto) |

---

## Foco e MetaPanel

- Clicar em qualquer lugar dentro de um `Pane` → `splitLayout.focused = paneId`
- `MetaPanel` recebe `session = $sessions.find(s => s.id === $selectedSessionId)` — sem mudança na sua implementação interna
- O painel em foco recebe uma borda sutil destacada (`1px solid var(--bd2)`)

---

## Fechar um painel

Cada `Pane` tem um botão `×` no header (aparece apenas quando `visible.length > 1`). Ao fechar:

1. Remove o `paneId` de `visible`
2. Mantém o `sessionId` no store (sessão continua rodando em background)
3. Se o painel fechado era o `focused`, foca no primeiro de `visible`
4. Grid CSS se reajusta automaticamente — sem animação na v1

---

## Renderização do grid (CSS)

`PaneGrid` usa CSS Grid com `grid-template-columns` e `grid-template-rows` calculados a partir de `visible`:

| `visible` | Colunas | Linhas |
|-----------|---------|--------|
| `['tl']` | `1fr` | `1fr` |
| `['tl','tr']` | `1fr 1fr` | `1fr` |
| `['tl','bl']` | `1fr` | `1fr 1fr` |
| `['tl','tr','bl','br']` | `1fr 1fr` | `1fr 1fr` |

Divisores entre painéis são `<PaneDivider>` estáticos (sem redimensionamento na v1 — splits sempre 50/50).

---

## O que NÃO muda

- Toda a camada Rust (sem alteração em nenhum arquivo `.rs`)
- Eventos Tauri (`session:output`, `session:state`, etc.)
- `CentralPanel`, `Feed`, `InputBar`, `MetaPanel` internamente
- Lógica de sessões no `sessions.ts` (exceto `selectedSessionId` virar derived)

---

## Fora de escopo (v1)

- Redimensionamento de painéis por arrastar o divisor (painéis sempre 50/50)
- Persistência do layout entre reinicializações do app
- Drag entre painéis (arrastar sessão de um painel para outro)
- Indicador de qual painel está em foco na sidebar

---

## Arquivos criados/modificados

| Arquivo | Ação |
|---------|------|
| `api/lib/stores/layout.ts` | **Criar** — `splitLayout` store e ações |
| `api/components/PaneGrid.svelte` | **Criar** — grid container |
| `api/components/Pane.svelte` | **Criar** — slot com drag detection |
| `api/App.svelte` | **Modificar** — substituir `CentralPanel` por `PaneGrid` |
| `api/components/Sidebar.svelte` | **Modificar** — adicionar `draggable` nos itens |
| `api/lib/stores/sessions.ts` | **Modificar** — `selectedSessionId` vira derived |
| `api/lib/stores/sessions.test.ts` | **Modificar** — adaptar testes ao derived |
