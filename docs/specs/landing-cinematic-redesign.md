# Landing Cinematic Redesign

## Objective

Rebuild the Orbit landing page as a professional Astro landing experience that communicates multi-agent orchestration quickly and uses cinematic motion without becoming a marketing-only splash page.

## Expected Behavior

- The page keeps the existing product content: Orbit, multi-provider AI agent sessions, real-time feed, persistent history, token/cost tracking, MCP orchestration, and install paths.
- A full-bleed Three.js background renders moving particles and subtle depth across the page.
- Lenis provides smooth scrolling for wheel and anchor navigation.
- Wheel scrolling snaps between page anchors with a smooth animated transition and a single active section state.
- Snapped sections align to the viewport start and are vertically centered, so the previous section does not remain visible after a completed scroll.
- GSAP and ScrollTrigger drive smooth text reveals and scroll-based transitions.
- The visual language follows Orbit's main dark theme tokens: compact type, muted panels, restrained borders, and green/status accents.
- The first viewport presents the Orbit brand and value proposition, while showing a hint of the next product section.
- Motion respects `prefers-reduced-motion` by disabling continuous animation and revealing content immediately.

## Edge Cases

- If WebGL fails, the page still renders readable content on the static background.
- If Lenis fails to initialize, native scrolling remains usable.
- Install command controls fall back gracefully when clipboard access is unavailable.
- The install operating-system dropdown opens above the CTA so it does not push attention below the hero action row.
- Product metrics below the demo are intentionally omitted to keep the product example as the section focus.
- The navigation uses separated glass controls instead of one continuous navigation bar background.
- The hero includes an animated orbit mark that echoes the Orbit logo.
- The MCP workflow visualization uses cards only, without connector branches.
- Mobile layouts avoid text overlap and keep the 3D scene behind content rather than beside it.

## Acceptance Criteria

- `landing` builds successfully with Astro.
- The page imports and uses `three`, `gsap`, and `lenis`.
- The Three.js canvas is fixed, full-bleed, nonblank, and animated.
- Scroll-triggered text transitions are visible on desktop and mobile.
- Navigation anchors use smooth scrolling.
- No dead legacy star/orbit scripts remain wired into the page.
