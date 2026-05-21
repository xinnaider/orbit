# Code Review: Pipeline e Config de Testes

**Reviewer:** AI Code Review Agent
**Date:** 2026-05-20
**Files Reviewed:** 6 (build.yml, e2e.yml, landing.yml, vite.config.js, vitest.config.js, vitest.components.config.ts, playwright.config.ts)
**Changes:** New E2E pipeline + config adjustments for test discovery

---

## Scope

| Dimensão       | Score | Notas |
|----------------|-------|-------|
| Architecture   | 🟡    | CI fragmentado em 3 workflows, sem shared config; 3 vitest configs diferentes |
| Correctness    | 🟢    | Pipelines funcionais, sem bugs evidentes |
| Security       | 🟡    | SSH password hardcoded no workflow (landing.yml), falta `actions/checkout` persist-credentials |
| Performance    | 🟢    | Cache npm e Rust ok, mas Rust cache não é reaproveitado entre jobs |
| Testing        | 🔴    | Lint job não roda testes — PR pode quebrar `npm test` sem ser detectado |
| Maintainability| 🟡    | Duplicação de paths-ignore entre workflows, 3 configs vitest com overlap |

---

## Required Changes 🔴

### [`.github/workflows/build.yml:78-79`] — Testes ausentes no lint job

**Issue:** O job `lint` roda rustfmt, clippy, eslint, svelte-check mas NÃO roda `npm test` e `npm run test:components`. Um PR pode passar no lint mas quebrar testes unitários/componentes.

**Severity:** 🔴 blocking
**Suggestion:** Adicionar steps de teste no job `lint`:

```yaml
      - name: Run unit tests
        run: npm test

      - name: Run E2E tests (smoke)
        run: npx playwright test --project=chromium e2e/
```

Ou criar job separado `test` que rode em paralelo com `lint`.

### [`.github/workflows/build.yml:78-79`] — svelte-check lento e frágil sem cache

**Issue:** `npx svelte-kit sync` roda em toda execução entre linters, sem cache do `.svelte-kit/`. `cargo check --release` na linha 67 faz o mesmo — compila tudo em release mode antes do lint, o que adiciona ~3-5min.

**Severity:** 🟡 important
**Suggestion:** Separar `cargo check --release` em step condicional (só antes de build) ou usar `--profile=check` se disponível.

---

## Suggestions 🟡

### [`.github/workflows/build.yml:31-79`] — Rust cache não é compartilhado entre jobs

**Issue:** `lint` e `build-windows` rodam `swatinem/rust-cache` separadamente com `shared-key` diferentes (`rust-windows` no lint vs `rust-windows` no build — ok, estão iguais). Mas `cache-on-failure: true` com `cargo check --release` no lint e `cargo build --release` no build não compartilha artefatos de compilação entre jobs porque cada job tem seu diretório de trabalho independente.

**Severity:** 🟡 important
**Suggestion:** Usar `actions/cache` manual para compartilhar `tauri/target` entre jobs, ou garantir que o build nunca precise recompilar o que o lint já compilou. Na prática, como são jobs diferentes em runners diferentes, o cache do Rust salva na action cache e o build seguinte restaura — funciona, mas é lento.

### [`.github/workflows/e2e.yml:40-43`] — npm ci e playwright install sequenciais

**Issue:** `npm ci` seguido de `npx playwright install chromium` — total ~2min. Podem ser paralelizados ou otimizados com cache do Playwright.

**Severity:** 🟢 nit
**Suggestion:** Adicionar cache do Playwright browsers:

```yaml
      - name: Cache Playwright browsers
        uses: actions/cache@v4
        id: playwright-cache
        with:
          path: ~/AppData/Local/ms-playwright
          key: playwright-${{ hashFiles('package-lock.json') }}-chromium

      - run: npx playwright install chromium
        if: steps.playwright-cache.outputs.cache-hit != 'true'
```

### [`.github/workflows/landing.yml:77-86`] — SSH com senha em vez de chave

**Issue:** `appleboy/ssh-action` com `password: ${{ secrets.SERVER_PASSWORD }}` — funcional, mas menos seguro que SSH key. Também `docker compose pull` sem `docker compose build` — se a imagem não tiver tag `latest` nova, deploy não muda nada.

**Severity:** 🟡 important
**Suggestion:** Migrar para SSH key. Adicionar `docker compose up -d --build` para garantir que o container use a imagem mais recente.

### [`.github/workflows/build.yml:8-25`] — paths-ignore duplicado entre push e pull_request

**Issue:** A lista de `paths-ignore` é idêntica em `push` e `pull_request`. Qualquer mudança precisa ser feita em dois lugares.

**Severity:** 🟢 nit
**Suggestion:** Usar YAML anchors:

```yaml
on:
  push:
    branches: [master, dev]
    tags: ['v*']
    paths-ignore: &IGNORE
      - '**.md'
      - '.github/ISSUE_TEMPLATE/**'
      - 'docs/**'
  pull_request:
    branches: [master, dev]
    paths-ignore: *IGNORE
```

### [`vite.config.js` + `vitest.config.js`] — Duas configs de teste

**Issue:** Temos 3 arquivos de config de teste: `vite.config.js` (unit, ambiente node), `vitest.config.js` (unit, ambiente jsdom), `vitest.components.config.ts` (componentes, happy-dom). O `vitest.config.js` duplica 95% do `vite.config.js` só pra mudar environment e VITE_MOCK. Quando alguém mexe no server config do vite (porta, host), precisa lembrar de atualizar os 2 ou 3 arquivos.

**Severity:** 🟡 important
**Suggestion:** Unificar: usar `vite.config.js` como base, e os configs específicos sobrescreverem só o necessário.

```js
// vitest.config.js
import { defineConfig, mergeConfig } from 'vitest/config';
import viteConfig from './vite.config';

export default defineConfig(mergeConfig(viteConfig, {
  test: { environment: 'jsdom' },
  define: { 'import.meta.env.VITE_MOCK': '"true"' },
}));
```

Isso exige que `vite.config.js` exporte um objeto síncrono (não async function).

### [`playwright.config.ts:11`] — `fullyParallel: true` com 1 worker no CI

**Issue:** `fullyParallel: true` junto com `workers: isCI ? 1 : undefined` — `fullyParallel` só faz efeito com `workers > 1`. No CI só 1 worker, então `fullyParallel` é inócuo. Não quebra, mas é enganoso.

**Severity:** 🟢 nit
**Suggestion:** Fazer `fullyParallel` também condicional:

```ts
fullyParallel: isCI ? false : true,
```

---

## Questions ❓

- **build.yml linha 78-79:** Por que `svelte-check` precisa de `npx svelte-kit sync` antes? Isso poderia ser um step separado e cacheado?
- **landing.yml:** Tem intenção de adicionar health check pós-deploy? Se o container sobe com erro, o workflow reporta sucesso.
- **e2e.yml:** O timeout de 10min é para o job inteiro ou por step? Na linha 47, `timeout-minutes` está no `name: Run E2E smoke tests`, não no step. Na prática, o timeout do job inteiro é 6h (default do GH). Melhor colocar timeout no job: `timeout-minutes: 15`.

---

## Verdict

**🔄 Changes Requested**

Testes precisam rodar no CI — o lint job do `build.yml` é o lugar natural. Sem isso, PRs podem quebrar `npm test` sem ninguém perceber até o merge.
