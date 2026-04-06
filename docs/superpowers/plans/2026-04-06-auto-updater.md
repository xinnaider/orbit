# Auto-Updater com Notificação — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implementar auto-updater no Orbit com notificação visual ao usuário, usando `tauri-plugin-updater` e `latest.json` hospedado no próprio GitHub Releases.

**Architecture:** O Rust backend expõe dois comandos IPC (`check_update`, `install_update`). O frontend verifica updates silenciosamente ao iniciar e exibe um banner persistente quando há versão nova disponível. O CI gera e faz upload do `latest.json` junto com cada release.

**Tech Stack:** `tauri-plugin-updater 2`, `@tauri-apps/plugin-updater`, GitHub Releases para hospedar `latest.json`, PowerShell no CI para gerar o manifesto.

**Por que GitHub Releases e não a landing page:**
- Já existe pipeline gerando releases automáticas
- Zero infra extra — só adicionar o arquivo ao upload existente
- URL previsível: `https://github.com/xinnaider/orbit/releases/latest/download/latest.json`
- A landing page exigiria um deploy pipeline adicional para um arquivo JSON

---

## Arquivos criados / modificados

| Arquivo | Ação | Responsabilidade |
|---------|------|-----------------|
| `front/Cargo.toml` | Modificar | Adicionar `tauri-plugin-updater = "2"` |
| `front/src/lib.rs` | Modificar | Registrar plugin + novos comandos IPC |
| `front/src/ipc/updater.rs` | Criar | Comandos `check_update` e `install_update` |
| `front/src/ipc/mod.rs` | Modificar | Exportar módulo `updater` |
| `front/tauri.conf.json` | Modificar | Adicionar bloco `plugins.updater` com pubkey e endpoint |
| `api/lib/types.ts` | Modificar | Adicionar tipo `UpdateInfo` |
| `api/lib/tauri.ts` | Modificar | Adicionar `checkUpdate()` e `installUpdate()` |
| `api/components/UpdateBanner.svelte` | Criar | Banner de notificação com botão de instalar |
| `api/App.svelte` | Modificar | Verificar update ao montar, exibir `UpdateBanner` |
| `.github/workflows/build.yml` | Modificar | Gerar e fazer upload de `latest.json` na release |

---

## Pré-requisito: Chave pública para verificação de assinaturas

O `tauri-plugin-updater` precisa da **chave pública** correspondente ao `TAURI_SIGNING_PRIVATE_KEY` já configurado no CI.

- [ ] **Verificar se você já tem a chave pública**

Se não tiver, execute localmente:
```bash
npx tauri signer generate -w tauri.key
```
Isso gera `tauri.key` (privada) e `tauri.key.pub` (pública). O conteúdo do `.pub` vai em `tauri.conf.json`. O conteúdo da privada vai no secret `TAURI_SIGNING_PRIVATE_KEY` do GitHub (substitua se já existe).

Se o CI já assina com sucesso e você tem a privada, extraia a pública rodando:
```bash
# A pública é a segunda linha do output de tauri signer generate
# Ou peça ao tauri-cli para mostrar a chave pública:
npx tauri signer sign --help
```

O valor que entra em `tauri.conf.json` é uma string base64 que começa com `dW50cnVzdGVkIGNvbW1lbnQ6...` — é seguro commitar.

---

## Task 1: Adicionar dependência Rust e registrar plugin

**Files:**
- Modify: `front/Cargo.toml`
- Modify: `front/src/lib.rs`
- Create: `front/src/ipc/updater.rs`
- Modify: `front/src/ipc/mod.rs`

- [ ] **Step 1: Adicionar `tauri-plugin-updater` ao Cargo.toml**

Em `front/Cargo.toml`, adicionar na seção `[dependencies]`:
```toml
tauri-plugin-updater = "2"
```

- [ ] **Step 2: Criar `front/src/ipc/updater.rs` com os dois comandos**

```rust
use tauri::{AppHandle, command};
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub body: String,
    pub current_version: String,
}

/// Verifica se há uma versão mais nova disponível.
/// Retorna None se estiver na versão mais recente.
#[command]
pub async fn check_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    let updater = app
        .updater_builder()
        .build()
        .map_err(|e| e.to_string())?;

    match updater.check().await {
        Ok(Some(update)) => Ok(Some(UpdateInfo {
            version: update.version.clone(),
            body: update.body.clone().unwrap_or_default(),
            current_version: update.current_version.to_string(),
        })),
        Ok(None) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Baixa e instala a atualização disponível, depois reinicia o app.
#[command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let updater = app
        .updater_builder()
        .build()
        .map_err(|e| e.to_string())?;

    let update = updater
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Nenhuma atualização disponível".to_string())?;

    update
        .download_and_install(|_downloaded, _total| {}, || {})
        .await
        .map_err(|e| e.to_string())?;

    app.restart();
}
```

- [ ] **Step 3: Exportar módulo em `front/src/ipc/mod.rs`**

Adicionar ao arquivo existente:
```rust
pub mod updater;
```

- [ ] **Step 4: Registrar plugin e comandos em `front/src/lib.rs`**

Adicionar `.plugin(tauri_plugin_updater::Builder::new().build())` e os dois comandos no `invoke_handler`:

```rust
// No início de run(), antes de .setup():
.plugin(tauri_plugin_updater::Builder::new().build())

// No invoke_handler, adicionar os dois comandos:
ipc::updater::check_update,
ipc::updater::install_update,
```

- [ ] **Step 5: Verificar compilação**

```bash
cargo check --manifest-path front/Cargo.toml
```
Esperado: `Finished` sem erros.

- [ ] **Step 6: Commit**

```bash
git add front/Cargo.toml front/Cargo.lock front/src/lib.rs front/src/ipc/updater.rs front/src/ipc/mod.rs
git commit -m "feat(updater): add tauri-plugin-updater, IPC commands check_update and install_update"
```

---

## Task 2: Configurar tauri.conf.json

**Files:**
- Modify: `front/tauri.conf.json`

- [ ] **Step 1: Adicionar bloco `plugins.updater` ao `tauri.conf.json`**

Substituir o conteúdo atual pelo abaixo, adicionando o bloco `plugins` após `bundle`:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Orbit",
  "version": "0.1.0",
  "identifier": "com.josefernando.orbit",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../build"
  },
  "app": {
    "windows": [
      {
        "title": "Orbit",
        "width": 1200,
        "height": 750,
        "minWidth": 900,
        "minHeight": 500
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  },
  "plugins": {
    "updater": {
      "pubkey": "COLE_AQUI_SUA_CHAVE_PUBLICA",
      "endpoints": [
        "https://github.com/xinnaider/orbit/releases/latest/download/latest.json"
      ]
    }
  }
}
```

Substitua `COLE_AQUI_SUA_CHAVE_PUBLICA` pelo conteúdo do seu arquivo `.pub` gerado no pré-requisito.

- [ ] **Step 2: Verificar compilação**

```bash
cargo check --manifest-path front/Cargo.toml
```

- [ ] **Step 3: Commit**

```bash
git add front/tauri.conf.json
git commit -m "feat(updater): configure updater endpoint and pubkey in tauri.conf.json"
```

---

## Task 3: Frontend — tipos e wrappers IPC

**Files:**
- Modify: `api/lib/types.ts`
- Modify: `api/lib/tauri.ts`

- [ ] **Step 1: Adicionar tipo `UpdateInfo` em `api/lib/types.ts`**

Adicionar no final do arquivo:
```typescript
export interface UpdateInfo {
  version: string;
  body: string;
  currentVersion: string;
}
```

- [ ] **Step 2: Adicionar funções em `api/lib/tauri.ts`**

Adicionar no final do arquivo:
```typescript
export async function checkUpdate(): Promise<UpdateInfo | null> {
  return await invoke<UpdateInfo | null>('check_update');
}

export async function installUpdate(): Promise<void> {
  await invoke('install_update');
}
```

- [ ] **Step 3: Verificar tipos**

```bash
npx svelte-check --tsconfig ./tsconfig.json
```
Esperado: 0 erros.

- [ ] **Step 4: Commit**

```bash
git add api/lib/types.ts api/lib/tauri.ts
git commit -m "feat(updater): add UpdateInfo type and IPC wrappers"
```

---

## Task 4: Componente UpdateBanner

**Files:**
- Create: `api/components/UpdateBanner.svelte`

- [ ] **Step 1: Criar `api/components/UpdateBanner.svelte`**

```svelte
<script lang="ts">
  import type { UpdateInfo } from '../lib/types';
  import { installUpdate } from '../lib/tauri';

  export let update: UpdateInfo;

  let installing = false;
  let error = '';

  async function install() {
    installing = true;
    error = '';
    try {
      await installUpdate();
      // app reinicia automaticamente após install
    } catch (e: any) {
      error = e?.message ?? String(e);
      installing = false;
    }
  }
</script>

<div class="update-banner">
  <div class="update-icon">↑</div>
  <div class="update-body">
    <div class="update-title">
      nova versão disponível — <span class="update-version">{update.version}</span>
    </div>
    {#if update.body}
      <div class="update-notes">{update.body}</div>
    {/if}
    {#if error}
      <div class="update-error">{error}</div>
    {/if}
  </div>
  <button class="update-btn" on:click={install} disabled={installing}>
    {installing ? 'instalando...' : 'atualizar agora'}
  </button>
</div>

<style>
  .update-banner {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    z-index: 200;
    display: flex;
    align-items: center;
    gap: 12px;
    background: rgba(0, 212, 126, 0.08);
    border-top: 1px solid rgba(0, 212, 126, 0.3);
    padding: 10px 16px;
    animation: slideUp 0.2s ease;
  }
  @keyframes slideUp {
    from { transform: translateY(100%); }
    to   { transform: translateY(0); }
  }
  .update-icon {
    font-size: 16px;
    color: var(--ac);
    flex-shrink: 0;
  }
  .update-body {
    flex: 1;
    min-width: 0;
  }
  .update-title {
    font-size: var(--sm);
    color: var(--t1);
    font-weight: 500;
  }
  .update-version {
    color: var(--ac);
  }
  .update-notes {
    font-size: var(--xs);
    color: var(--t2);
    margin-top: 2px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .update-error {
    font-size: var(--xs);
    color: var(--s-error);
    margin-top: 2px;
  }
  .update-btn {
    background: var(--ac-d);
    border: 1px solid var(--ac);
    border-radius: 3px;
    color: var(--ac);
    font-size: var(--xs);
    padding: 5px 14px;
    flex-shrink: 0;
    letter-spacing: 0.04em;
    transition: background 0.15s;
  }
  .update-btn:hover:not(:disabled) {
    background: rgba(0, 212, 126, 0.18);
  }
  .update-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
```

- [ ] **Step 2: Verificar tipos**

```bash
npx svelte-check --tsconfig ./tsconfig.json
```
Esperado: 0 erros.

- [ ] **Step 3: Commit**

```bash
git add api/components/UpdateBanner.svelte
git commit -m "feat(updater): add UpdateBanner component"
```

---

## Task 5: Integrar em App.svelte

**Files:**
- Modify: `api/App.svelte`

- [ ] **Step 1: Importar `UpdateBanner`, `checkUpdate` e `UpdateInfo` em `App.svelte`**

No bloco `<script>`, adicionar:
```typescript
import UpdateBanner from './components/UpdateBanner.svelte';
import { checkUpdate } from './lib/tauri';
import type { UpdateInfo } from './lib/types';

let availableUpdate: UpdateInfo | null = null;
```

- [ ] **Step 2: Verificar update no `onMount` com delay de 3s**

Dentro do `onMount`, ao final (após configurar os listeners):
```typescript
// Verifica update em background sem bloquear o startup
setTimeout(async () => {
  try {
    availableUpdate = await checkUpdate();
  } catch (_e) {
    // silencioso — falha de rede não deve afetar o uso do app
  }
}, 3000);
```

- [ ] **Step 3: Exibir `UpdateBanner` no template**

No template, adicionar antes do `<div class="layout">`:
```svelte
{#if availableUpdate}
  <UpdateBanner update={availableUpdate} />
{/if}
```

- [ ] **Step 4: Verificar lint e tipos**

```bash
npx svelte-check --tsconfig ./tsconfig.json
npx eslint api --max-warnings 0
```
Esperado: 0 erros, 0 warnings.

- [ ] **Step 5: Commit**

```bash
git add api/App.svelte
git commit -m "feat(updater): check for update on startup, show UpdateBanner when available"
```

---

## Task 6: CI — gerar e publicar `latest.json`

**Files:**
- Modify: `.github/workflows/build.yml`

O CI precisa, após o build, coletar o arquivo `.nsis.zip` e sua `.sig`, e gerar o `latest.json` com a assinatura embutida para upload na release.

- [ ] **Step 1: Adicionar step de geração do `latest.json` no workflow**

No job `build-windows`, após o step `Compute SHA-256 of installer` e antes de `Upload build artifacts`, adicionar:

```yaml
- name: Generate latest.json for updater
  shell: pwsh
  run: |
    $version = "${{ github.ref_name }}"
    if (-not $version.StartsWith("v")) { $version = "0.0.0-nightly" }
    $version = $version.TrimStart("v")

    # Localiza o .nsis.zip e sua .sig
    $zip = Get-ChildItem front/target/release/bundle/nsis/*.nsis.zip -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($null -eq $zip) {
      Write-Host "Nenhum .nsis.zip encontrado, pulando geração do latest.json"
      exit 0
    }

    $sigFile = "$($zip.FullName).sig"
    if (-not (Test-Path $sigFile)) {
      Write-Host "Arquivo .sig não encontrado: $sigFile"
      exit 0
    }

    $signature = Get-Content $sigFile -Raw
    $pubDate = (Get-Date -Format "yyyy-MM-ddTHH:mm:ssZ")

    # URL de download na release do GitHub
    $downloadUrl = "https://github.com/xinnaider/orbit/releases/download/v$version/$($zip.Name)"

    $latestJson = @{
      version  = $version
      notes    = "Veja o changelog completo em https://github.com/xinnaider/orbit/releases/tag/v$version"
      pub_date = $pubDate
      platforms = @{
        "windows-x86_64" = @{
          signature = $signature.Trim()
          url       = $downloadUrl
        }
      }
    } | ConvertTo-Json -Depth 5

    $latestJson | Out-File -FilePath latest.json -Encoding utf8NoBOM
    Write-Host "latest.json gerado:"
    Get-Content latest.json
```

- [ ] **Step 2: Incluir `latest.json` no upload de artifacts**

No step `Upload build artifacts`, adicionar `latest.json` à lista de `path`:
```yaml
path: |
  front/target/release/bundle/msi/*.msi
  front/target/release/bundle/nsis/*.exe
  front/target/release/bundle/nsis/*.nsis.zip
  front/target/release/orbit.exe
  latest.json
  sha256sum.txt
```

- [ ] **Step 3: Incluir `latest.json` e o `.nsis.zip` nos assets da release**

No step `Create Release`, adicionar ao `files`:
```yaml
files: |
  front/target/release/bundle/msi/*.msi
  front/target/release/bundle/nsis/*.exe
  front/target/release/bundle/nsis/*.nsis.zip
  latest.json
  sha256sum.txt
```

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/build.yml
git commit -m "feat(updater): generate and publish latest.json in CI release pipeline"
```

---

## Task 7: Abrir PR e testar

- [ ] **Step 1: Push da branch**

```bash
git push -u origin feat/auto-updater
```

- [ ] **Step 2: Abrir PR**

```bash
gh pr create \
  --title "feat: auto-updater com notificação" \
  --body "Implementa atualização automática com banner de notificação.

## O que muda
- Verifica nova versão silenciosamente 3s após abrir o app
- Exibe banner na parte inferior quando há atualização disponível
- Botão 'atualizar agora' baixa e instala — app reinicia automaticamente
- CI gera \`latest.json\` e publica junto com cada release

## Pré-requisito antes do merge
- Substituir \`COLE_AQUI_SUA_CHAVE_PUBLICA\` em \`tauri.conf.json\` pela chave pública real
- Garantir que \`TAURI_SIGNING_PRIVATE_KEY\` no GitHub Secrets está correto"
```

- [ ] **Step 3: Verificar que o CI passou (lint + build)**

Acompanhar em: `https://github.com/xinnaider/orbit/actions`

- [ ] **Step 4: Atualizar CHANGELOG.md antes do merge**

Em `CHANGELOG.md`, adicionar em Abril 2026:

```markdown
### DD/04 · Novo — Atualização automática
O app verifica automaticamente se há uma versão nova disponível logo ao abrir.
Quando houver, um aviso aparece na parte inferior da tela com um botão para instalar
e reiniciar — sem precisar baixar nada manualmente.
```

---

## Notas de teste manual

Após merge e release de uma versão nova (`v0.2.0`):

1. Instalar a versão anterior (`v0.1.0`) localmente
2. Abrir o app — após 3 segundos o banner deve aparecer
3. Clicar em "atualizar agora" — progresso e reinício automático
4. Verificar que a versão instalada é `v0.2.0`

Para testar sem uma release real: apontar temporariamente o endpoint para um servidor local com um `latest.json` com versão superior à atual.
