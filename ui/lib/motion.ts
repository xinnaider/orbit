/**
 * Reduced-motion-aware wrappers around Svelte's built-in transitions.
 *
 * Motion in Orbit is subtle and tactile: short slides/fades that give feedback
 * on state changes (panels opening, the sidebar collapsing, a message landing),
 * never decorative. Every transition here collapses to an instant (duration 0)
 * cut when the user prefers reduced motion, so the directive can be used
 * directly without each call site re-checking the media query.
 *
 * Usage:  <div transition:fly={{ x: -16 }}>  /  in:fade  /  out:slide
 */
import { fly as flyBase, fade as fadeBase, slide as slideBase } from 'svelte/transition';
import { cubicOut } from 'svelte/easing';

type FlyParams = Parameters<typeof flyBase>[1];
type FadeParams = Parameters<typeof fadeBase>[1];
type SlideParams = Parameters<typeof slideBase>[1];

/** True when the OS asks for reduced motion. Safe in non-browser/test envs. */
export function prefersReducedMotion(): boolean {
  return (
    typeof window !== 'undefined' &&
    typeof window.matchMedia === 'function' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches
  );
}

/** Default tactile timing: quick, eased, never lingering. */
const DUR = 200;

export function fly(node: Element, params: FlyParams = {}) {
  return flyBase(node, {
    duration: prefersReducedMotion() ? 0 : DUR,
    easing: cubicOut,
    ...params,
    ...(prefersReducedMotion() ? { duration: 0 } : {}),
  });
}

export function fade(node: Element, params: FadeParams = {}) {
  return fadeBase(node, {
    duration: prefersReducedMotion() ? 0 : DUR,
    ...params,
    ...(prefersReducedMotion() ? { duration: 0 } : {}),
  });
}

export function slide(node: Element, params: SlideParams = {}) {
  return slideBase(node, {
    duration: prefersReducedMotion() ? 0 : DUR,
    easing: cubicOut,
    ...params,
    ...(prefersReducedMotion() ? { duration: 0 } : {}),
  });
}
