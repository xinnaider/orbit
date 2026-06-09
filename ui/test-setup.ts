// Component-test setup.
//
// Svelte 5 transitions drive animation through the Web Animations API
// (element.animate). happy-dom does not implement it, so any element carrying a
// transition directive throws "element.animate is not a function". Provide a
// minimal no-op Animation so transitions resolve instantly in tests (the motion
// itself is exercised in the browser, not here).
if (typeof Element !== 'undefined' && !Element.prototype.animate) {
  Element.prototype.animate = function animate() {
    return {
      cancel() {},
      finish() {},
      play() {},
      pause() {},
      reverse() {},
      addEventListener() {},
      removeEventListener() {},
      onfinish: null,
      oncancel: null,
      currentTime: 0,
      playState: 'finished',
      finished: Promise.resolve(),
    } as unknown as Animation;
  };
}
