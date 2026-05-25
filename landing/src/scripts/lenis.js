import Lenis from 'lenis';
import 'lenis/dist/lenis.css';

export function initLenis() {
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
    document.documentElement.classList.add('motion-reduced');
    return null;
  }

  if (window.matchMedia('(max-width: 760px), (pointer: coarse)').matches) {
    document.documentElement.classList.add('native-scroll');
    return null;
  }

  const lenis = new Lenis({
    anchors: { offset: -84 },
    autoRaf: false,
    duration: 1.15,
    easing: (t) => 1 - Math.pow(1 - t, 3),
    gestureOrientation: 'vertical',
    smoothWheel: true,
    syncTouch: false,
    wheelMultiplier: 0.86,
  });

  document.documentElement.classList.add('lenis-ready');
  return lenis;
}
