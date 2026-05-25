import { gsap } from 'gsap';
import { ScrollTrigger } from 'gsap/ScrollTrigger';

gsap.registerPlugin(ScrollTrigger);

function revealImmediately() {
  document.querySelectorAll('[data-reveal], [data-text-scrub]').forEach((element) => {
    element.style.opacity = '1';
    element.style.transform = 'none';
    element.style.filter = 'none';
  });
}

export function initMotion({ lenis, scene } = {}) {
  const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  if (reducedMotion) {
    revealImmediately();
    return () => {};
  }

  const cleanups = [];

  if (lenis) {
    const raf = (time) => lenis.raf(time * 1000);
    lenis.on('scroll', ScrollTrigger.update);
    gsap.ticker.add(raf);
    gsap.ticker.lagSmoothing(0);
    cleanups.push(() => gsap.ticker.remove(raf));
  }

  gsap.set('[data-reveal]', {
    filter: 'blur(10px)',
    opacity: 0,
    y: 32,
  });

  gsap.utils.toArray('[data-reveal]').forEach((element) => {
    const tween = gsap.to(element, {
      duration: 1.1,
      ease: 'power3.out',
      filter: 'blur(0px)',
      opacity: 1,
      scrollTrigger: {
        end: 'top 54%',
        once: true,
        start: 'top 86%',
        trigger: element,
      },
      y: 0,
    });

    cleanups.push(() => tween.kill());
  });

  gsap.utils.toArray('[data-text-scrub]').forEach((element) => {
    const tween = gsap.fromTo(
      element,
      {
        filter: 'blur(12px)',
        opacity: 0.16,
        y: 48,
      },
      {
        ease: 'none',
        filter: 'blur(0px)',
        opacity: 1,
        scrollTrigger: {
          end: 'top 42%',
          scrub: 0.8,
          start: 'top 92%',
          trigger: element,
        },
        y: 0,
      }
    );

    cleanups.push(() => tween.kill());
  });

  const heroTimeline = gsap.timeline({ defaults: { ease: 'power3.out' } });
  heroTimeline
    .from('.hero-kicker', { duration: 0.8, opacity: 0, y: 20 }, 0.1)
    .from('.hero-title', { duration: 1.1, opacity: 0, y: 44 }, 0.18)
    .from('.hero-copy', { duration: 0.9, opacity: 0, y: 24 }, 0.36)
    .from('.hero-actions', { duration: 0.8, opacity: 0, y: 22 }, 0.54)
    .from('.hero-proof', { duration: 0.8, opacity: 0, y: 18 }, 0.7);
  cleanups.push(() => heroTimeline.kill());

  const sceneTrigger = ScrollTrigger.create({
    end: 'bottom bottom',
    onUpdate: (self) => scene?.setScrollProgress(self.progress),
    start: 'top top',
    trigger: document.body,
  });
  cleanups.push(() => sceneTrigger.kill());

  const imageRefresh = () => ScrollTrigger.refresh();
  window.addEventListener('load', imageRefresh, { once: true });
  cleanups.push(() => window.removeEventListener('load', imageRefresh));

  ScrollTrigger.refresh();

  return () => {
    cleanups.forEach((cleanup) => cleanup());
    ScrollTrigger.getAll().forEach((trigger) => trigger.kill());
  };
}
