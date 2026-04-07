# Changelog

---

## Abril 2026

### 07/04 · Novo — Painéis divididos (split panes)
O Orbit agora permite visualizar até 4 sessões do Claude Code simultaneamente.
Arraste qualquer sessão da barra lateral para a borda de um painel para abrir
uma divisão lado a lado. Até 4 painéis em grade 2×2. Clique em um painel para
focá-lo — o MetaPanel acompanha o painel em foco.

### 06/04 · Novo — Apelido e worktree ao criar sessão
Ao criar uma nova sessão, agora é possível dar um apelido personalizado para identificá-la facilmente. Se o campo for deixado em branco, o app sugere automaticamente um nome baseado em codinomes de dispositivos Android combinados com o nome do projeto.

Também foi adicionada a opção de criar a sessão dentro de um **git worktree** isolado. Quando ativada, o Claude trabalha em um branch separado (criado automaticamente como `orbit/<nome-da-sessão>`), mantendo o branch principal intacto durante o trabalho.

### 06/04 · Melhoria — Execução de comandos sem interrupção
Comandos do terminal agora executam automaticamente, sem pedir confirmação a cada passo. O fluxo de trabalho do agente ficou mais fluido e sem pausas desnecessárias.

### 06/04 · Melhoria — Output em tempo real
Durante a execução de comandos longos, o resultado aparece progressivamente na tela — sem precisar esperar o comando terminar para ver o que está acontecendo.

### 06/04 · Novo — Aviso de limite de uso da API
Quando o limite de uso da API do Claude é atingido, o app exibe uma mensagem clara na tela em vez de simplesmente parar de responder. O aviso some automaticamente após 30 segundos.

### 06/04 · Novo — Atualização automática
O app verifica automaticamente se há uma versão nova disponível logo ao abrir.
Quando houver, um aviso aparece na parte inferior da tela com um botão para instalar
e reiniciar — sem precisar baixar nada manualmente.

### 06/04 · Ajuste — Indicador de sessão parada
Sessões encerradas agora exibem uma etiqueta "stopped" no painel lateral, tornando mais fácil identificar o estado de cada sessão.

---
