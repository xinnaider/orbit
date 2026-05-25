# Scroll to bottom button — ORB-6

## Objetivo

O botão flutuante **"↓ scroll to bottom"** deve aparecer no **canto inferior direito** da área do chat, alinhado ao padding horizontal do feed — não à esquerda junto à coluna da timeline.

## Comportamento esperado

- Quando o usuário rola o feed para cima (`atBottom === false`), o botão aparece sobre o conteúdo do chat.
- Posição: `position: absolute`, `bottom` ~14px (10px em viewport ≤768px), **`right`** alinhado ao padding do timeline (38px desktop, 18px mobile) — espelhando o padding direito de `.timeline` em `Feed.svelte`.
- Clique: chama `Feed.scrollToBottom()` e oculta o botão quando o feed volta ao fim.
- O botão não deve sobrepor a coluna de nós da timeline à esquerda nem o input na base do painel.

## Casos de borda

- **Split panes / compact density**: botão permanece ancorado ao `.feed-wrap` do painel ativo (não vaza para o painel vizinho).
- **Feed vazio ou só mensagens pendentes**: botão não é renderizado (`{#if !atBottom}` dentro de feed com conteúdo).
- **Redimensionamento / mobile**: `right` reduz para 18px no breakpoint ≤768px, consistente com o padding do timeline em mobile.
- **Múltiplas sessões**: trocar de sessão recria o `Feed` via `{#key session.id}`; estado `atBottom` reinicia corretamente.

## Critérios de aceitação

1. Com feed longo, após scroll manual para cima, o botão aparece **à direita** do painel central.
2. O botão **não** aparece alinhado à coluna de timeline (esquerda).
3. Clicar no botão leva o scroll ao fim e o botão some.
4. Em layout mobile (≤768px), o botão continua à direita com offset 18px.
5. Em split view com dois painéis, cada painel mostra seu botão apenas no canto inferior direito **daquele** painel.

## Pontos de teste (manual)

| # | Passo | Resultado esperado |
|---|--------|-------------------|
| 1 | Abrir sessão com histórico longo | Feed carrega no fim; botão oculto |
| 2 | Rolar feed para cima | Botão visível no canto **inferior direito** |
| 3 | Inspecionar posição vs timeline | Botão não sobrepõe os nós da timeline à esquerda |
| 4 | Clicar "↓ scroll to bottom" | Scroll no fim; botão desaparece |
| 5 | Reduzir janela ≤768px, repetir 2–4 | Botão ainda à direita, ~18px da borda |
| 6 | Abrir split com dois chats, rolar só um | Botão só no painel rolado, canto direito desse painel |
| 7 | Trocar de sessão na sidebar | Sem botão fantasma; comportamento reinicia na nova sessão |

## Implementação

- Arquivo: `ui/components/CentralPanel.svelte` — classe `.scroll-btn`: usar `right` em vez de `left`.
