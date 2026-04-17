# Orbit Landing Page — Design Spec

## Goal

Create a standalone landing page for Orbit in a `landing/` folder within the monorepo. The page must grab attention, communicate what the product does in seconds, and convert visitors to GitHub stars + downloads.

## Decisions

| Decision | Choice |
|---|---|
| Stack | Astro (SSG, no JS framework overhead) |
| Aesthetic | Terminal Cosmos — `#080808` background, `#00d47e` accent, JetBrains Mono |
| Primary CTA | Star on GitHub → `github.com/xinnaider/orbit` |
| Secondary CTA | Download for Windows → GitHub Releases latest |
| Layout | 3 sections: Hero + Features + Footer |
| Deployment | Static output — GitHub Pages or any static host |

---

## Structure

```
landing/
├── src/
│   ├── pages/
│   │   └── index.astro        # Single page
│   ├── components/
│   │   ├── Nav.astro           # Fixed nav
│   │   ├── Hero.astro          # Orbit animation + CTAs
│   │   ├── Features.astro      # 6-item grid
│   │   └── Footer.astro        # Links + license
│   └── styles/
│       └── global.css          # CSS variables, resets, animations
├── public/
│   └── favicon.svg             # Orbit SVG icon
├── astro.config.mjs
└── package.json
```

---

## Design System

```css
--bg:      #080808
--bg1:     #0e0e0e
--bd:      rgba(255,255,255,0.04)
--ac:      #00d47e
--ac-dim:  rgba(0,212,126,0.10)
--t0:      #f0f0f0
--t1:      #c0c0c0
--t2:      #585858
--t3:      #303030
--font:    'JetBrains Mono', 'Cascadia Code', 'Fira Code', monospace
```

---

## Sections

### Nav (fixed, `position: fixed`)

- Left: Orbit SVG logo + wordmark "orbit"
- Right: "docs" link · "releases" link · "★ GitHub" button (accent bordered)
- Mobile: hide "docs" + "releases", keep only GitHub button
- Background: `#080808` + `backdrop-filter: blur(12px)` for glass effect

### Hero (full viewport height)

**Background elements:**
- ~10 star dots scattered, each with independent CSS `twinkle` animation (opacity 0 → peak → 0)
- No images, pure CSS + SVG

**Orbit animation (SVG, 300×300px):**
- Outer ellipse ring: slow clockwise rotation (12s)
- Inner ellipse ring: faster counter-clockwise (8s)
- Satellite dot: follows outer ring orbit (12s, same speed)
- Central body: 3 concentric circles with decreasing opacity + glow filter
- All transforms use `transform-origin: 50% 50%` on the SVG group

**Content (centered, below orbit):**
- Eyebrow: `● multi-agent dashboard` in accent green, uppercase, letter-spaced
- Title: `orbit` — large, monospace, `clamp(40px, 8vw, 72px)`
- Subtitle: 2-line description, muted `#585858`
- CTA group (column, centered):
  1. `★ Star on GitHub` — accent border + background, GitHub SVG icon
  2. `↓ Download for Windows` — muted secondary style
  3. Meta line: `Windows 10+ · MIT · free & open source` in `#303030`

### Features (below hero)

- Section label: `what it does` — small, ultra-spaced, barely visible
- **6-cell grid** (3×2 desktop, 1 column mobile):
  1. ⊙ Multi-session
  2. ▸ Real-time feed
  3. ◈ Persistent history
  4. $ Cost tracking
  5. / Slash commands
  6. ○ Context menu
- Grid separated by 1px `rgba(255,255,255,0.04)` lines (gap:1px on dark background)
- Each cell: hover brightens background slightly
- Accent `#00d47e` on key phrases within descriptions

### Footer

- Left: `orbit · MIT license · © josefernando`
- Right: GitHub · Releases · Issues links
- Mobile: stacks to column, centered

---

## Responsive Breakpoints

| Breakpoint | Changes |
|---|---|
| ≤ 768px | Nav links hidden (docs, releases). Orbit 300→220px. Features 3col→1col. Footer row→column. |
| ≤ 480px | Orbit 220→180px. Title letter-spacing reduced. |

---

## Animations

| Element | Animation | Duration |
|---|---|---|
| Stars (~10) | `twinkle` — opacity 0→peak→0 | 3–6s each, staggered delays |
| Outer ring | Clockwise rotation | 12s linear infinite |
| Inner ring | Counter-clockwise | 8s linear infinite |
| Satellite | Clockwise (same as outer) | 12s linear infinite |
| CTAs | Hover: background + border opacity + box-shadow | 0.2s transition |
| Nav links | Hover: color | 0.15s transition |
| Feature cells | Hover: background | 0.2s transition |

---

## Astro Config

- `output: 'static'` — pre-rendered HTML, no server
- `site: 'https://xinnaider.github.io/orbit'` (or custom domain)
- No JS framework — Astro components only, CSS animations handle interactivity

---

## GitHub Actions integration

Add a deploy step to `.github/workflows/build.yml` that runs `npm run build` in `landing/` and deploys to GitHub Pages on every push to `master`.

---

## Out of scope

- Blog / changelog
- i18n
- Dark/light toggle (always dark)
- Analytics (can be added later as a script tag)
