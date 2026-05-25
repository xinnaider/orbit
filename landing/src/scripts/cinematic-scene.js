import * as THREE from 'three';

const PALETTE = [0x7cff9e, 0x4da3ff, 0xf2c96f, 0xf1ece3];

function hasWebGL() {
  try {
    const canvas = document.createElement('canvas');
    return Boolean(
      window.WebGLRenderingContext &&
        (canvas.getContext('webgl') || canvas.getContext('experimental-webgl'))
    );
  } catch {
    return false;
  }
}

function createParticleField(isMobile) {
  const count = isMobile ? 720 : 1680;
  const geometry = new THREE.BufferGeometry();
  const positions = new Float32Array(count * 3);
  const base = new Float32Array(count * 3);
  const colorValues = new Float32Array(count * 3);
  const phase = new Float32Array(count);
  const speed = new Float32Array(count);
  const color = new THREE.Color();

  for (let i = 0; i < count; i += 1) {
    const spreadX = isMobile ? 18 : 34;
    const spreadY = isMobile ? 22 : 24;
    const x = (Math.random() - 0.5) * spreadX;
    const y = (Math.random() - 0.45) * spreadY;
    const z = -Math.random() * 58 + 10;
    const paletteColor = PALETTE[Math.floor(Math.random() * PALETTE.length)];

    base[i * 3] = x;
    base[i * 3 + 1] = y;
    base[i * 3 + 2] = z;
    positions[i * 3] = x;
    positions[i * 3 + 1] = y;
    positions[i * 3 + 2] = z;
    phase[i] = Math.random() * Math.PI * 2;
    speed[i] = 0.35 + Math.random() * 1.2;

    color.setHex(paletteColor);
    color.multiplyScalar(0.55 + Math.random() * 0.45);
    colorValues[i * 3] = color.r;
    colorValues[i * 3 + 1] = color.g;
    colorValues[i * 3 + 2] = color.b;
  }

  geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
  geometry.setAttribute('color', new THREE.BufferAttribute(colorValues, 3));

  const material = new THREE.PointsMaterial({
    blending: THREE.AdditiveBlending,
    depthWrite: false,
    opacity: 0.86,
    size: isMobile ? 0.044 : 0.058,
    sizeAttenuation: true,
    transparent: true,
    vertexColors: true,
  });

  return {
    base,
    count,
    geometry,
    material,
    phase,
    points: new THREE.Points(geometry, material),
    positions,
    speed,
  };
}

function createSignalLines(isMobile) {
  const group = new THREE.Group();
  const lineCount = isMobile ? 9 : 18;

  for (let i = 0; i < lineCount; i += 1) {
    const geometry = new THREE.BufferGeometry();
    const vertices = [];
    const width = isMobile ? 9 : 18;
    const y = -7 + Math.random() * 16;
    const z = -34 - Math.random() * 20;
    const tilt = (Math.random() - 0.5) * 0.8;

    for (let p = 0; p < 42; p += 1) {
      const progress = p / 41;
      vertices.push(
        -width + progress * width * 2,
        y + Math.sin(progress * Math.PI * 2 + i) * 0.18 + progress * tilt,
        z + Math.sin(progress * Math.PI + i * 0.3) * 1.2
      );
    }

    geometry.setAttribute('position', new THREE.Float32BufferAttribute(vertices, 3));
    const material = new THREE.LineBasicMaterial({
      blending: THREE.AdditiveBlending,
      color: i % 3 === 0 ? 0x4da3ff : i % 3 === 1 ? 0x7cff9e : 0xf2c96f,
      depthWrite: false,
      opacity: 0.08,
      transparent: true,
    });

    const line = new THREE.Line(geometry, material);
    line.userData.speed = 0.08 + Math.random() * 0.16;
    line.userData.offset = Math.random() * Math.PI * 2;
    group.add(line);
  }

  return group;
}

function createHorizon() {
  const grid = new THREE.GridHelper(32, 28, 0x7cff9e, 0xf1ece3);
  grid.position.set(0, -8, -22);
  grid.rotation.x = Math.PI * 0.05;
  grid.material.opacity = 0.08;
  grid.material.transparent = true;
  grid.material.depthWrite = false;
  return grid;
}

function createOrbitLine(radiusX, radiusY, opacity) {
  const points = [];
  for (let i = 0; i <= 192; i += 1) {
    const angle = (i / 192) * Math.PI * 2;
    points.push(new THREE.Vector3(Math.cos(angle) * radiusX, Math.sin(angle) * radiusY, 0));
  }

  const geometry = new THREE.BufferGeometry().setFromPoints(points);
  const material = new THREE.LineBasicMaterial({
    blending: THREE.AdditiveBlending,
    color: 0x7cff9e,
    depthWrite: false,
    opacity,
    transparent: true,
  });
  material.userData.baseOpacity = opacity;

  return new THREE.Line(geometry, material);
}

function createHeroOrbit() {
  const group = new THREE.Group();
  group.name = 'hero-orbit-3d';

  const ringA = createOrbitLine(2.72, 0.78, 0.38);
  ringA.rotation.set(
    THREE.MathUtils.degToRad(68),
    THREE.MathUtils.degToRad(-28),
    THREE.MathUtils.degToRad(-18)
  );

  const ringB = createOrbitLine(2.18, 0.6, 0.2);
  ringB.rotation.set(
    THREE.MathUtils.degToRad(52),
    THREE.MathUtils.degToRad(38),
    THREE.MathUtils.degToRad(30)
  );

  const ringC = createOrbitLine(1.58, 0.44, 0.13);
  ringC.rotation.set(
    THREE.MathUtils.degToRad(76),
    THREE.MathUtils.degToRad(12),
    THREE.MathUtils.degToRad(72)
  );

  const coreMaterial = new THREE.MeshStandardMaterial({
    blending: THREE.AdditiveBlending,
    color: 0x7cff9e,
    emissive: 0x7cff9e,
    emissiveIntensity: 0.86,
    opacity: 0.48,
    roughness: 0.58,
    transparent: true,
  });
  coreMaterial.userData.baseOpacity = 0.48;
  const core = new THREE.Mesh(new THREE.SphereGeometry(0.42, 40, 28), coreMaterial);

  const haloMaterial = new THREE.MeshBasicMaterial({
    blending: THREE.AdditiveBlending,
    color: 0x7cff9e,
    depthWrite: false,
    opacity: 0.07,
    transparent: true,
  });
  haloMaterial.userData.baseOpacity = 0.07;
  const halo = new THREE.Mesh(new THREE.SphereGeometry(0.82, 40, 28), haloMaterial);

  const satellitePlane = new THREE.Group();
  satellitePlane.rotation.copy(ringA.rotation);
  const satellitePivot = new THREE.Group();
  const satelliteMaterial = new THREE.MeshStandardMaterial({
    blending: THREE.AdditiveBlending,
    color: 0x7cff9e,
    emissive: 0x7cff9e,
    emissiveIntensity: 1.2,
    opacity: 0.7,
    transparent: true,
  });
  satelliteMaterial.userData.baseOpacity = 0.7;
  const satellite = new THREE.Mesh(new THREE.SphereGeometry(0.095, 24, 16), satelliteMaterial);
  satellite.position.set(2.72, 0, 0);
  satellitePivot.add(satellite);
  satellitePlane.add(satellitePivot);

  group.add(ringA, ringB, ringC, halo, core, satellitePlane);
  group.userData.satellitePivot = satellitePivot;
  group.userData.baseRotation = {
    x: THREE.MathUtils.degToRad(-14),
    y: THREE.MathUtils.degToRad(-42),
    z: THREE.MathUtils.degToRad(8),
  };

  return group;
}

function layoutHeroOrbit(heroOrbit) {
  const isMobile = window.innerWidth < 760;
  if (isMobile) {
    heroOrbit.position.set(4.65, 3, -12.8);
    heroOrbit.scale.setScalar(2.35);
    heroOrbit.userData.opacityMultiplier = 0.7;
    return;
  }

  heroOrbit.position.set(5.55, -0.15, -11.8);
  heroOrbit.scale.setScalar(4.05);
  heroOrbit.userData.opacityMultiplier = 1;
}

function setObjectOpacity(root, opacityMultiplier) {
  root.traverse((object) => {
    const materials = object.material
      ? Array.isArray(object.material)
        ? object.material
        : [object.material]
      : [];

    materials.forEach((material) => {
      if (typeof material.opacity === 'number') {
        material.opacity = (material.userData.baseOpacity ?? material.opacity) * opacityMultiplier;
      }
    });
  });
}

function disposeObject(root) {
  root.traverse((object) => {
    object.geometry?.dispose?.();
    const materials = object.material
      ? Array.isArray(object.material)
        ? object.material
        : [object.material]
      : [];
    materials.forEach((material) => material.dispose?.());
  });
}

export function initCinematicScene(containerId) {
  const container = document.getElementById(containerId);
  if (!container || !hasWebGL()) return null;

  const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const isMobile = window.innerWidth < 760;
  const renderer = new THREE.WebGLRenderer({
    alpha: true,
    antialias: true,
    powerPreference: 'high-performance',
  });
  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(48, 1, 0.1, 120);
  const clock = new THREE.Clock();
  const pointer = { x: 0, y: 0 };
  const pointerTarget = { x: 0, y: 0 };
  const scroll = { progress: 0 };

  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.setClearColor(0x050505, 0);
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 1.8));
  renderer.domElement.className = 'scene-canvas';
  container.appendChild(renderer.domElement);

  scene.fog = new THREE.FogExp2(0x050505, 0.036);
  camera.position.set(0, isMobile ? 1.2 : 2.1, isMobile ? 22 : 18);

  const field = createParticleField(isMobile);
  const lines = createSignalLines(isMobile);
  const horizon = createHorizon();
  const heroOrbit = createHeroOrbit();
  const rig = new THREE.Group();

  rig.add(field.points, lines, horizon);
  scene.add(rig, heroOrbit);
  layoutHeroOrbit(heroOrbit);

  const ambient = new THREE.AmbientLight(0xf1ece3, 0.9);
  const heroLight = new THREE.PointLight(0x7cff9e, 2.2, 34);
  heroLight.position.set(4, 2.5, 4);
  scene.add(ambient, heroLight);

  function resize() {
    const width = window.innerWidth;
    const height = window.innerHeight;
    camera.aspect = width / height;
    camera.updateProjectionMatrix();
    renderer.setSize(width, height, false);
    layoutHeroOrbit(heroOrbit);
  }

  function updateParticles(time) {
    for (let i = 0; i < field.count; i += 1) {
      const index = i * 3;
      const wave = time * field.speed[i] + field.phase[i];
      const drift = (time * field.speed[i] * 2.2 + scroll.progress * 24) % 64;

      field.positions[index] = field.base[index] + Math.sin(wave * 0.8) * 0.34;
      field.positions[index + 1] = field.base[index + 1] + Math.cos(wave * 0.6) * 0.38;
      field.positions[index + 2] = ((field.base[index + 2] + drift + 64) % 70) - 56;
    }

    field.geometry.attributes.position.needsUpdate = true;
  }

  function render() {
    const time = clock.getElapsedTime();

    pointer.x += (pointerTarget.x - pointer.x) * 0.055;
    pointer.y += (pointerTarget.y - pointer.y) * 0.055;

    if (!reducedMotion) {
      updateParticles(time);
      lines.children.forEach((line, index) => {
        line.position.x = Math.sin(time * line.userData.speed + line.userData.offset) * 1.4;
        line.material.opacity = 0.06 + Math.sin(time * 0.9 + index) * 0.028;
      });
      rig.rotation.y = pointer.x * 0.035 + scroll.progress * 0.14;
      rig.rotation.x = -pointer.y * 0.025;
      rig.rotation.z = Math.sin(time * 0.08) * 0.015;
      horizon.position.z = -22 + scroll.progress * 11;

      const visibility = Math.max(0, 1 - scroll.progress * 5.8);
      heroOrbit.visible = visibility > 0.015;
      setObjectOpacity(heroOrbit, visibility * heroOrbit.userData.opacityMultiplier);
      heroOrbit.rotation.x =
        heroOrbit.userData.baseRotation.x + Math.sin(time * 0.2) * 0.045 + pointer.y * 0.08;
      heroOrbit.rotation.y =
        heroOrbit.userData.baseRotation.y +
        Math.sin(time * 0.16) * 0.075 +
        pointer.x * 0.16 +
        scroll.progress * 0.7;
      heroOrbit.rotation.z = heroOrbit.userData.baseRotation.z + Math.sin(time * 0.12) * 0.035;
      heroOrbit.userData.satellitePivot.rotation.z = time * 0.54;
    }

    camera.position.x = pointer.x * 0.45;
    camera.position.y = (isMobile ? 1.2 : 2.1) + pointer.y * 0.28 - scroll.progress * 1.2;
    camera.lookAt(0, -0.6 - scroll.progress * 2.8, -18);
    renderer.render(scene, camera);
  }

  function tick() {
    render();
  }

  function onPointerMove(event) {
    pointerTarget.x = (event.clientX / window.innerWidth - 0.5) * 2;
    pointerTarget.y = (event.clientY / window.innerHeight - 0.5) * 2;
  }

  resize();
  render();
  if (!reducedMotion) renderer.setAnimationLoop(tick);
  window.addEventListener('resize', resize);
  window.addEventListener('pointermove', onPointerMove, { passive: true });

  return {
    setScrollProgress(value) {
      scroll.progress = Math.max(0, Math.min(1, value));
    },
    destroy() {
      renderer.setAnimationLoop(null);
      window.removeEventListener('resize', resize);
      window.removeEventListener('pointermove', onPointerMove);
      field.geometry.dispose();
      field.material.dispose();
      lines.children.forEach((line) => {
        line.geometry.dispose();
        line.material.dispose();
      });
      disposeObject(heroOrbit);
      horizon.geometry.dispose();
      if (Array.isArray(horizon.material)) {
        horizon.material.forEach((material) => material.dispose());
      } else {
        horizon.material.dispose();
      }
      renderer.dispose();
      renderer.domElement.remove();
    },
  };
}
