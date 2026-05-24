import { initCinematicScene } from './cinematic-scene.js';
import { initInstallControls } from './install.js';
import { initLenis } from './lenis.js';
import { initMotion } from './motion.js';

function hideLoader() {
  const loader = document.getElementById('loader');
  if (!loader) return;

  loader.classList.add('is-hidden');
  setTimeout(() => loader.remove(), 420);
}

function initAnchorFallback(lenis, snapToSectionId) {
  document.querySelectorAll('a[href^="#"]').forEach((anchor) => {
    anchor.addEventListener('click', (event) => {
      const href = anchor.getAttribute('href');
      if (!href || href.length <= 1) return;

      const target = document.querySelector(href);
      if (!target) return;

      event.preventDefault();
      if (snapToSectionId?.(href.slice(1))) return;

      if (lenis) {
        lenis.scrollTo(target);
        return;
      }

      target.scrollIntoView({ behavior: 'smooth', block: 'start' });
    });
  });
}

function initSectionSnap(lenis) {
  if (!lenis || window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;

  const sections = [...document.querySelectorAll('[data-snap-section]')];
  if (sections.length < 2) return;

  let activeIndex = 0;
  let isSnapping = false;
  let releaseTimer;
  let settleTimer;

  function nearestSectionIndex() {
    const scrollTop = window.scrollY;
    return sections.reduce((nearest, section, index) => {
      const currentDistance = Math.abs(section.offsetTop - scrollTop);
      const nearestDistance = Math.abs(sections[nearest].offsetTop - scrollTop);

      return currentDistance < nearestDistance ? index : nearest;
    }, 0);
  }

  function shouldSkipSettleSnap() {
    const lastSection = sections[sections.length - 1];
    if (!lastSection) return false;

    return window.scrollY > lastSection.offsetTop + 24;
  }

  function snapTo(index) {
    const target = sections[index];
    if (!target) return false;

    window.clearTimeout(releaseTimer);
    activeIndex = index;
    isSnapping = true;
    lenis.scrollTo(target, {
      duration: 0.98,
      force: true,
      lock: true,
      onComplete: () => {
        isSnapping = false;
      },
    });

    releaseTimer = window.setTimeout(() => {
      isSnapping = false;
    }, 1250);
    return true;
  }

  function snapToSectionId(id) {
    const index = sections.findIndex((section) => section.id === id);
    if (index < 0) return false;

    return snapTo(index);
  }

  function scheduleNearestSnap() {
    if (isSnapping) return;
    if (shouldSkipSettleSnap()) return;

    window.clearTimeout(settleTimer);
    settleTimer = window.setTimeout(() => {
      if (!isSnapping && !shouldSkipSettleSnap()) snapTo(nearestSectionIndex());
    }, 160);
  }

  document.addEventListener(
    'wheel',
    (event) => {
      if (event.ctrlKey || Math.abs(event.deltaY) < 12) return;
      if (event.target?.closest?.('[data-lenis-prevent]')) return;

      if (isSnapping) {
        event.preventDefault();
        return;
      }

      const direction = event.deltaY > 0 ? 1 : -1;
      activeIndex = nearestSectionIndex();
      const nextIndex = Math.max(0, Math.min(sections.length - 1, activeIndex + direction));

      if (nextIndex === activeIndex) return;

      event.preventDefault();
      snapTo(nextIndex);
    },
    { capture: true, passive: false }
  );

  document.addEventListener('touchend', scheduleNearestSnap, { passive: true });
  lenis.on('scroll', scheduleNearestSnap);

  return snapToSectionId;
}

export function initExperience() {
  if (window.__orbitLandingReady) return;
  window.__orbitLandingReady = true;

  const start = () => {
    const lenis = initLenis();
    const scene = initCinematicScene('scene-root');
    const snapToSectionId = initSectionSnap(lenis);

    window.__lenis = lenis;
    initAnchorFallback(lenis, snapToSectionId);
    initInstallControls();
    initMotion({ lenis, scene });
    hideLoader();
  };

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', start, { once: true });
  } else {
    start();
  }
}
