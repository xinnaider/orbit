# Orbit Landing Page — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a standalone Astro static landing page for Orbit inside `landing/` in the monorepo, with orbit animations, Terminal Cosmos aesthetic, and GitHub + Download CTAs.

**Architecture:** Astro SSG project in `landing/` with four components (Nav, Hero, Features, Footer), one global CSS file with design tokens and animations, and a GitHub Actions deploy step. No JS framework — CSS-only animations.

**Tech Stack:** Astro 4, CSS animations, SVG, GitHub Pages deploy via `peaceiris/actions-gh-pages`.

---

## File Map

```
CREATE:
  landing/package.json
  landing/astro.config.mjs
  landing/public/favicon.svg
  landing/src/styles/global.css
  landing/src/components/Nav.astro
  landing/src/components/Hero.astro
  landing/src/components/Features.astro
  landing/src/components/Footer.astro
  landing/src/pages/index.astro

MODIFY:
  .github/workflows/build.yml   — add landing deploy job
```

---

## Task 1: Astro project scaffold

**Files:**
- Create: `landing/package.json`
- Create: `landing/astro.config.mjs`
- Create: `landing/public/favicon.svg`

- [ ] **Step 1: Create `landing/package.json`**

```json
{
  "name": "orbit-landing",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "astro dev",
    "build": "astro build",
    "preview": "astro preview"
  },
  "dependencies": {
    "astro": "^4.0.0"
  }
}
```

- [ ] **Step 2: Create `landing/astro.config.mjs`**

```js
import { defineConfig } from 'astro/config';

export default defineConfig({
  output: 'static',
  site: 'https://xinnaider.github.io/orbit',
  base: '/orbit',
});
```

- [ ] **Step 3: Create `landing/public/favicon.svg`**

```svg
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
  <rect width="24" height="24" fill="#080808"/>
  <ellipse cx="12" cy="12" rx="10.2" ry="4.2" stroke="#00d47e" stroke-width="1.4"
    fill="none" transform="rotate(-38 12 12)"/>
  <circle cx="12" cy="12" r="2.6" fill="#00d47e"/>
  <circle cx="20.4" cy="7.6" r="1.3" fill="#00d47e" opacity="0.75"/>
</svg>
```

- [ ] **Step 4: Install Astro**

```bash
cd landing && npm install
```

Expected: `node_modules/` created, no errors.

- [ ] **Step 5: Verify dev server starts**

```bash
npm run dev
```

Expected: `http://localhost:4321` opens (empty page is fine — no src/pages yet).

- [ ] **Step 6: Commit**

```bash
git add landing/
git commit -m "feat(landing): scaffold Astro project"
```

---

## Task 2: Global CSS — design tokens and animations

**Files:**
- Create: `landing/src/styles/global.css`

- [ ] **Step 1: Create `landing/src/styles/global.css`**

```css
@import url('https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@300;400;500;600&display=swap');

/* ── Tokens ── */
:root {
  --bg:     #080808;
  --bg1:    #0e0e0e;
  --bd:     rgba(255, 255, 255, 0.04);
  --ac:     #00d47e;
  --ac-dim: rgba(0, 212, 126, 0.10);
  --t0:     #f0f0f0;
  --t1:     #c0c0c0;
  --t2:     #585858;
  --t3:     #303030;
  --font:   'JetBrains Mono', 'Cascadia Code', 'Fira Code', monospace;
}

/* ── Reset ── */
*, *::before, *::after { margin: 0; padding: 0; box-sizing: border-box; }
html, body { height: 100%; overflow-x: hidden; }
body {
  background: var(--bg);
  color: var(--t0);
  font-family: var(--font);
  font-size: 13px;
  line-height: 1.5;
  -webkit-font-smoothing: antialiased;
}
a { color: inherit; text-decoration: none; }

/* ── Scrollbar ── */
::-webkit-scrollbar { width: 4px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.08); border-radius: 2px; }

/* ── Orbit animations ── */
@keyframes spin-cw  { to { transform: rotate(360deg);  } }
@keyframes spin-ccw { to { transform: rotate(-360deg); } }
@keyframes twinkle  {
  0%, 100% { opacity: 0; }
  50%       { opacity: var(--peak, 0.5); }
}

.ring-outer  { animation: spin-cw  12s linear infinite; transform-origin: 50% 50%; }
.ring-inner  { animation: spin-ccw  8s linear infinite; transform-origin: 50% 50%; }
.satellite   { animation: spin-cw  12s linear infinite; transform-origin: 50% 50%; }

.star {
  position: absolute;
  width: 2px; height: 2px;
  border-radius: 50%;
  background: #fff;
  animation: twinkle var(--d, 4s) ease-in-out infinite;
  animation-delay: var(--delay, 0s);
  opacity: 0;
}
```

- [ ] **Step 2: Commit**

```bash
git add landing/src/styles/global.css
git commit -m "feat(landing): add global CSS tokens and animations"
```

---

## Task 3: Nav component

**Files:**
- Create: `landing/src/components/Nav.astro`

- [ ] **Step 1: Create `landing/src/components/Nav.astro`**

```astro
---
// No props — static nav
---

<nav>
  <a class="logo" href="/">
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
      <ellipse cx="12" cy="12" rx="10.2" ry="4.2"
        stroke="#00d47e" stroke-width="1.4"
        transform="rotate(-38 12 12)"/>
      <circle cx="12" cy="12" r="2.6" fill="#00d47e"/>
      <circle cx="20.4" cy="7.6" r="1.3" fill="#00d47e" opacity="0.75"/>
    </svg>
    orbit
  </a>
  <div class="links">
    <a href="https://github.com/xinnaider/orbit#readme" class="hide-mobile">docs</a>
    <a href="https://github.com/xinnaider/orbit/releases" class="hide-mobile">releases</a>
    <a href="https://github.com/xinnaider/orbit" class="gh-btn">★ GitHub</a>
  </div>
</nav>

<style>
  nav {
    position: fixed; top: 0; left: 0; right: 0; z-index: 100;
    padding: 14px 48px;
    display: flex; align-items: center; justify-content: space-between;
    border-bottom: 1px solid var(--bd);
    background: rgba(8, 8, 8, 0.85);
    backdrop-filter: blur(12px);
  }
  .logo {
    display: flex; align-items: center; gap: 9px;
    font-weight: 600; font-size: 14px; letter-spacing: 0.1em;
    color: var(--t0);
  }
  .links { display: flex; align-items: center; gap: 20px; }
  .links a {
    font-size: 11px; letter-spacing: 0.08em; color: var(--t2);
    transition: color 0.15s;
  }
  .links a:hover { color: var(--t0); }
  .gh-btn {
    display: flex; align-items: center; gap: 6px;
    background: var(--ac-dim);
    border: 1px solid rgba(0, 212, 126, 0.3);
    color: var(--ac) !important;
    border-radius: 4px; padding: 5px 12px;
    transition: background 0.15s !important;
  }
  .gh-btn:hover { background: rgba(0, 212, 126, 0.18) !important; }

  @media (max-width: 768px) {
    nav { padding: 12px 20px; }
    .hide-mobile { display: none; }
  }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add landing/src/components/Nav.astro
git commit -m "feat(landing): add Nav component"
```

---

## Task 4: Hero component

**Files:**
- Create: `landing/src/components/Hero.astro`

- [ ] **Step 1: Create `landing/src/components/Hero.astro`**

```astro
---
// No props
---

<section class="hero">
  <!-- Stars -->
  <div class="stars" aria-hidden="true">
    <div class="star" style="top:12%;left:8%;--d:4s;--delay:.2s;--peak:.5"></div>
    <div class="star" style="top:22%;left:85%;--d:3.5s;--delay:1s;--peak:.4"></div>
    <div class="star" style="top:65%;left:91%;--d:5s;--delay:.5s;--peak:.3"></div>
    <div class="star" style="top:80%;left:6%;--d:4.5s;--delay:2s;--peak:.4"></div>
    <div class="star" style="top:45%;left:4%;--d:6s;--delay:.8s;--peak:.3"></div>
    <div class="star" style="top:30%;left:92%;--d:3s;--delay:1.5s;--peak:.5"></div>
    <div class="star" style="top:72%;left:78%;--d:4s;--delay:.3s;--peak:.35"></div>
    <div class="star" style="top:18%;left:48%;--d:5.5s;--delay:2.5s;--peak:.25"></div>
    <div class="star" style="top:88%;left:55%;--d:3.8s;--delay:1.2s;--peak:.4"></div>
    <div class="star" style="top:55%;left:72%;--d:4.2s;--delay:.6s;--peak:.3"></div>
  </div>

  <!-- Orbit SVG -->
  <div class="orbit-wrap" aria-hidden="true">
    <svg viewBox="0 0 300 300" fill="none" xmlns="http://www.w3.org/2000/svg">
      <defs>
        <filter id="glow">
          <feGaussianBlur stdDeviation="3" result="b"/>
          <feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge>
        </filter>
        <filter id="glow-sm">
          <feGaussianBlur stdDeviation="2" result="b"/>
          <feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge>
        </filter>
      </defs>

      <!-- Outer ring (clockwise) -->
      <g class="ring-outer">
        <ellipse cx="150" cy="150" rx="138" ry="53"
          stroke="#00d47e" stroke-width="1.2" opacity="0.35"
          transform="rotate(-38 150 150)"/>
      </g>

      <!-- Inner ring (counter-clockwise) -->
      <g class="ring-inner">
        <ellipse cx="150" cy="150" rx="88" ry="34"
          stroke="#00d47e" stroke-width="0.8" opacity="0.18"
          transform="rotate(-38 150 150)"/>
      </g>

      <!-- Satellite (follows outer ring) -->
      <g class="satellite">
        <circle cx="279" cy="117" r="4.5"
          fill="#00d47e" opacity="0.9" filter="url(#glow-sm)"/>
      </g>

      <!-- Central body -->
      <circle cx="150" cy="150" r="20" fill="#00d47e" opacity="0.10"/>
      <circle cx="150" cy="150" r="12" fill="#00d47e" opacity="0.15"/>
      <circle cx="150" cy="150" r="6"  fill="#00d47e" filter="url(#glow)"/>
    </svg>
  </div>

  <!-- Content -->
  <p class="eyebrow">● multi-agent dashboard</p>
  <h1>orbit</h1>
  <p class="subtitle">
    Manage multiple Claude Code sessions simultaneously.<br/>
    Real-time feed, persistent history, token tracking.
  </p>

  <div class="cta-group">
    <a href="https://github.com/xinnaider/orbit" class="btn-primary">
      <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
        <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38
          0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13
          -.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66
          .07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15
          -.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27
          .68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12
          .51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48
          0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8
          c0-4.42-3.58-8-8-8z"/>
      </svg>
      Star on GitHub
    </a>
    <a href="https://github.com/xinnaider/orbit/releases/latest" class="btn-secondary">
      ↓ Download for Windows
    </a>
    <span class="meta">Windows 10+ · MIT · free &amp; open source</span>
  </div>
</section>

<style>
  .hero {
    min-height: 100vh;
    display: flex; flex-direction: column;
    align-items: center; justify-content: center;
    position: relative; overflow: hidden;
    padding: 120px 24px 80px;
    text-align: center;
  }
  .stars { position: absolute; inset: 0; pointer-events: none; }

  .orbit-wrap {
    width: 300px; height: 300px;
    margin: 0 auto 44px; flex-shrink: 0;
  }
  .orbit-wrap svg { width: 100%; height: 100%; overflow: visible; }

  .eyebrow {
    font-size: 10px; letter-spacing: 0.25em;
    color: var(--ac); text-transform: uppercase;
    margin-bottom: 14px;
  }
  h1 {
    font-size: clamp(40px, 8vw, 72px);
    font-weight: 600; letter-spacing: 0.12em;
    color: var(--t0); line-height: 1.1; margin-bottom: 18px;
  }
  .subtitle {
    font-size: 13px; color: var(--t2);
    max-width: 420px; line-height: 1.8; margin-bottom: 36px;
  }

  .cta-group {
    display: flex; flex-direction: column;
    align-items: center; gap: 10px;
  }
  .btn-primary, .btn-secondary {
    display: inline-flex; align-items: center; gap: 8px;
    border-radius: 4px; font-family: var(--font);
    cursor: pointer; width: 220px; justify-content: center;
    transition: all 0.2s;
  }
  .btn-primary {
    background: var(--ac-dim);
    border: 1px solid rgba(0, 212, 126, 0.4);
    color: var(--ac); padding: 12px 28px; font-size: 13px;
    letter-spacing: 0.06em;
  }
  .btn-primary:hover {
    background: rgba(0, 212, 126, 0.18);
    border-color: rgba(0, 212, 126, 0.7);
    box-shadow: 0 0 24px rgba(0, 212, 126, 0.12);
  }
  .btn-secondary {
    background: rgba(255,255,255,0.03);
    border: 1px solid rgba(255,255,255,0.08);
    color: #888; padding: 11px 28px; font-size: 12px;
    letter-spacing: 0.06em;
  }
  .btn-secondary:hover {
    background: rgba(255,255,255,0.06);
    border-color: rgba(255,255,255,0.15);
    color: var(--t1);
  }
  .meta {
    margin-top: 4px;
    font-size: 10px; color: var(--t3); letter-spacing: 0.1em;
  }

  @media (max-width: 768px) {
    .orbit-wrap { width: 220px; height: 220px; margin-bottom: 32px; }
    .subtitle { font-size: 12px; }
    .btn-primary, .btn-secondary { width: 200px; }
  }
  @media (max-width: 480px) {
    h1 { letter-spacing: 0.06em; }
    .orbit-wrap { width: 180px; height: 180px; }
  }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add landing/src/components/Hero.astro
git commit -m "feat(landing): add Hero component with orbit animation and CTAs"
```

---

## Task 5: Features component

**Files:**
- Create: `landing/src/components/Features.astro`

- [ ] **Step 1: Create `landing/src/components/Features.astro`**

```astro
---
const features = [
  {
    icon: '⊙',
    title: 'Multi-session',
    desc: 'Run <mark>multiple Claude agents</mark> in parallel across different projects simultaneously.',
  },
  {
    icon: '▸',
    title: 'Real-time feed',
    desc: 'Streaming JSON displayed as structured entries — thinking blocks, tool calls, responses.',
  },
  {
    icon: '◈',
    title: 'Persistent history',
    desc: 'SQLite-backed sessions. Close the app, reopen — <mark>conversations resume</mark> automatically.',
  },
  {
    icon: '$',
    title: 'Cost tracking',
    desc: 'Per-session token usage and estimated cost in USD. Always know what each agent is spending.',
  },
  {
    icon: '/',
    title: 'Slash commands',
    desc: 'Autocomplete from installed <mark>Claude Code plugins</mark>. File picker with @ references.',
  },
  {
    icon: '○',
    title: 'Context menu',
    desc: 'Right-click sessions to rename, stop, or delete. No digging through menus.',
  },
];
---

<section class="features">
  <p class="label">what it does</p>
  <div class="grid">
    {features.map((f) => (
      <div class="cell">
        <span class="icon">{f.icon}</span>
        <div class="title">{f.title}</div>
        <div class="desc" set:html={f.desc} />
      </div>
    ))}
  </div>
</section>

<style>
  .features {
    padding: 100px 48px;
    max-width: 960px; margin: 0 auto;
    border-top: 1px solid var(--bd);
  }
  .label {
    font-size: 9px; letter-spacing: 0.3em; text-transform: uppercase;
    color: var(--t3); margin-bottom: 44px; text-align: center;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 1px;
    background: var(--bd);
    border: 1px solid var(--bd);
    border-radius: 6px; overflow: hidden;
  }
  .cell {
    background: var(--bg); padding: 26px 22px;
    transition: background 0.2s;
  }
  .cell:hover { background: var(--bg1); }
  .icon { font-size: 17px; margin-bottom: 11px; display: block; }
  .title { font-size: 12px; font-weight: 600; color: var(--t1); margin-bottom: 7px; }
  .desc { font-size: 11px; color: #444; line-height: 1.6; }
  .desc :global(mark) {
    background: none; color: var(--ac);
  }

  @media (max-width: 768px) {
    .features { padding: 60px 20px; }
    .grid { grid-template-columns: 1fr; }
  }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add landing/src/components/Features.astro
git commit -m "feat(landing): add Features component"
```

---

## Task 6: Footer component

**Files:**
- Create: `landing/src/components/Footer.astro`

- [ ] **Step 1: Create `landing/src/components/Footer.astro`**

```astro
---
// No props
---

<footer>
  <span class="copy">orbit · MIT license · © josefernando</span>
  <div class="links">
    <a href="https://github.com/xinnaider/orbit">GitHub</a>
    <a href="https://github.com/xinnaider/orbit/releases">Releases</a>
    <a href="https://github.com/xinnaider/orbit/issues">Issues</a>
  </div>
</footer>

<style>
  footer {
    display: flex; align-items: center; justify-content: space-between;
    max-width: 960px; margin: 0 auto;
    padding: 24px 48px;
    border-top: 1px solid var(--bd);
  }
  .copy { font-size: 10px; color: var(--t3); letter-spacing: 0.08em; }
  .links { display: flex; gap: 20px; }
  .links a {
    font-size: 10px; color: var(--t3); letter-spacing: 0.08em;
    transition: color 0.15s;
  }
  .links a:hover { color: var(--t2); }

  @media (max-width: 768px) {
    footer {
      flex-direction: column; gap: 12px;
      text-align: center; padding: 20px;
    }
  }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add landing/src/components/Footer.astro
git commit -m "feat(landing): add Footer component"
```

---

## Task 7: index.astro — assemble the page

**Files:**
- Create: `landing/src/pages/index.astro`

- [ ] **Step 1: Create `landing/src/pages/index.astro`**

```astro
---
import Nav      from '../components/Nav.astro';
import Hero     from '../components/Hero.astro';
import Features from '../components/Features.astro';
import Footer   from '../components/Footer.astro';
import '../styles/global.css';
---

<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <meta name="description"
      content="Orbit — manage multiple Claude Code sessions simultaneously. Real-time feed, persistent history, cost tracking." />
    <meta property="og:title"       content="Orbit" />
    <meta property="og:description" content="Multi-session Claude Code agent dashboard." />
    <meta property="og:type"        content="website" />
    <meta name="theme-color"        content="#080808" />
    <link rel="icon" type="image/svg+xml" href="/orbit/favicon.svg" />
    <title>Orbit — Claude Agent Dashboard</title>
  </head>
  <body>
    <Nav />
    <main>
      <Hero />
      <Features />
    </main>
    <Footer />
  </body>
</html>
```

- [ ] **Step 2: Run dev and verify the full page**

```bash
cd landing && npm run dev
```

Open `http://localhost:4321/orbit` — expect:
- Fixed nav with logo + links
- Full-screen hero with orbit animation (3 rings rotating, satellite orbiting, stars twinkling)
- Two CTAs: Star on GitHub (green) + Download for Windows (muted)
- 6-cell feature grid below
- Footer at bottom

- [ ] **Step 3: Test mobile layout**

In browser DevTools → toggle device toolbar → 375px width. Expect:
- Nav shows only GitHub button
- Orbit shrinks to 220px
- Features stack to 1 column
- Footer stacks vertically

- [ ] **Step 4: Build and verify static output**

```bash
npm run build
```

Expected: `dist/` folder created with `index.html` and assets. No build errors.

- [ ] **Step 5: Commit**

```bash
git add landing/src/pages/index.astro
git commit -m "feat(landing): assemble full page — hero, features, footer"
```

---

## Task 8: GitHub Actions deploy to GitHub Pages

**Files:**
- Modify: `.github/workflows/build.yml`

- [ ] **Step 1: Add landing deploy job to `.github/workflows/build.yml`**

Add this job at the end of the file (after `build-windows`):

```yaml
  deploy-landing:
    name: Deploy Landing Page
    runs-on: ubuntu-latest
    needs: lint
    if: github.event_name == 'push' && github.ref == 'refs/heads/master'
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'
          cache-dependency-path: landing/package-lock.json

      - name: Install landing deps
        run: cd landing && npm ci

      - name: Build landing
        run: cd landing && npm run build

      - name: Deploy to GitHub Pages
        uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: landing/dist
          force_orphan: true
```

- [ ] **Step 2: Generate lockfile for landing**

```bash
cd landing && npm install
```

This creates `landing/package-lock.json` needed for `cache-dependency-path`.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/build.yml landing/package-lock.json
git commit -m "ci: deploy landing to GitHub Pages on master push"
```

- [ ] **Step 4: Enable GitHub Pages in repo settings**

Go to `https://github.com/xinnaider/orbit/settings/pages`:
- Source: **Deploy from a branch**
- Branch: **gh-pages** / **/ (root)**
- Click Save

After the next push to master, the landing will be live at `https://xinnaider.github.io/orbit`.

---

## Self-Review

**Spec coverage:**
- ✅ Astro SSG stack
- ✅ Terminal Cosmos aesthetic (tokens in global.css)
- ✅ Nav with fixed position, glass background, mobile-hide links
- ✅ Hero: orbit animation (outer/inner rings + satellite + stars + central body)
- ✅ Hero CTAs: GitHub (primary) + Download (secondary) + meta line
- ✅ Features: 6-cell grid, 3→1col on mobile, accent on phrases
- ✅ Footer: left/right, stacks on mobile
- ✅ Responsive breakpoints at 768px and 480px
- ✅ All CSS animations (twinkle, spin-cw, spin-ccw, hover transitions)
- ✅ GitHub Pages deploy via Actions

**No placeholders found.**

**Type consistency:** No shared types across components — each is self-contained Astro.
