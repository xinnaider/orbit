# Lessons Learned

Padrões de erro identificados durante o desenvolvimento. Leia ao início de cada sessão.

---

## GIF / Puppeteer

**Regra:** O parâmetro `delay` do gifenc está em **milissegundos** (não centissegundos como o spec GIF). Use `Math.round(1000 / FPS)`.
**Por quê:** Passando `100 / FPS` (centissegundos), o GIF ficava ~10× mais rápido que o esperado.
**Quando aplicar:** Sempre que usar gifenc para gerar GIFs animados.

---

**Regra:** Usar `.flatten({ background: '#ffffff' }).ensureAlpha()` no sharp antes de passar para `quantize`, e passar `{ clearAlphaColor: 255 }` ao quantize.
**Por quê:** `clearAlphaColor` padrão é `0` (preto) — pixels transparentes viravam preto, corrompendo frames com fundo escuro/transparente.
**Quando aplicar:** Ao converter screenshots para GIF com gifenc.

---

**Regra:** Usar `page.waitForSelector('.item')` antes da primeira captura, não apenas `sleep()`.
**Por quê:** `sleep` fixo não garante que o conteúdo renderizou — o primeiro frame saía branco.
**Quando aplicar:** Sempre que capturar screenshots com Puppeteer após navegação.

---

**Regra:** Após rodar o gerador de GIF, verificar se a porta 1420 ficou em TIME_WAIT antes de rodar novamente.
**Por quê:** Tentativas consecutivas falham com `ERR_CONNECTION_REFUSED` ou navigation timeout porque o Vite não consegue bindar a porta.
**Quando aplicar:** Ao rodar `npm run demo:gif` mais de uma vez seguida.

---

## Git / Versionamento

**Regra:** Ao fazer `git stash` + `git pull` + `git stash pop`, verificar se arquivos staged antes do stash foram incluídos no commit seguinte.
**Por quê:** A deleção do screenshot foi incluída no commit de bump de versão sem intenção, pois estava staged antes do stash.
**Quando aplicar:** Sempre que usar stash para contornar conflitos de pull.

---

## Execucao de Planos

**Regra:** Ao executar um plano com multiplos blocos, verificar no codigo que cada bloco visivel foi conectado ao fluxo real da UI antes de declarar concluido.
**Por quê:** Implementar o Git panel sem conectar o bloco de tabs/header deixou os ajustes visuais aprovados invisiveis no app.
**Quando aplicar:** Sempre que um plano incluir componentes novos e wiring em stores/containers, especialmente planos com etapas sobrepostas.
