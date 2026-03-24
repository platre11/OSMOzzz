import * as THREE from "three";
import { GLTFLoader } from "three/addons/loaders/GLTFLoader.js";
import { DRACOLoader } from "three/addons/loaders/DRACOLoader.js";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";

export function initShield(container) {
  const scene = new THREE.Scene();

  const camera = new THREE.PerspectiveCamera(
    45,
    container.clientWidth / container.clientHeight,
    0.1,
    100,
  );
  camera.position.set(0, 0, 5);

  const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
  renderer.setSize(container.clientWidth, container.clientHeight);
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 3;
  container.appendChild(renderer.domElement);

  scene.add(new THREE.HemisphereLight(0xffffff, 0x444466, 4));
  const dir = new THREE.DirectionalLight(0xffffff, 10);
  dir.position.set(5, 5, 5);
  scene.add(dir);
  const fill = new THREE.DirectionalLight(0xaabbff, 6);
  fill.position.set(-5, 2, -3);
  scene.add(fill);
  const front = new THREE.DirectionalLight(0xffffff, 8);
  front.position.set(0, 0, 10);
  scene.add(front);

  const controls = new OrbitControls(camera, renderer.domElement);
  controls.enableZoom = false;
  controls.enablePan = false;
  controls.enableRotate = false;
  controls.autoRotate = true;
  controls.autoRotateSpeed = 1.5;

  const draco = new DRACOLoader();
  draco.setDecoderPath(
    "https://www.gstatic.com/draco/versioned/decoders/1.5.7/",
  );

  // ── State ─────────────────────────────────────────────────────────────────
  let hexGroupRef   = null; // Hexagons_Group mesh
  let hexCenterRef  = null; // Hex_center mesh
  let hexAnimating  = false;
  let hexStartTime  = null;

  let shieldMeshes    = []; // Shield mesh(es) to fade in after RimGlow
  let shieldFading    = false;
  let shieldStartTime = null;
  const SHIELD_FADE_DURATION = 800; // ms

  let popPhase     = 0;
  let popStartTime = null;
  const popStartPos   = new THREE.Vector3();
  const popStartQuat  = new THREE.Quaternion();
  const popStartScale = new THREE.Vector3();
  const popTargetPos  = new THREE.Vector3();
  const popTargetQuat = new THREE.Quaternion();

  // Billboard transition (phase 2 → 3)
  let billboardStartTime = null;
  const billboardStartQuat = new THREE.Quaternion();
  const BILLBOARD_DURATION = 500;

  const easeOutCubic   = (t) => 1 - Math.pow(1 - t, 3);
  const easeInOutCubic = (t) =>
    t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;

  // ── Load GLB ───────────────────────────────────────────────────────────────
  const gltfLoader = new GLTFLoader();
  gltfLoader.setDRACOLoader(draco);
  gltfLoader.load(
    "/assets/shield_web.glb",
    (gltf) => {
      const model = gltf.scene;
      const box   = new THREE.Box3().setFromObject(model);
      model.position.sub(box.getCenter(new THREE.Vector3()));
      const size  = box.getSize(new THREE.Vector3());
      model.scale.setScalar(2.5 / Math.max(size.x, size.y, size.z));

      model.traverse((child) => {
        if (child.isMesh && child.material) {
          child.material.roughness = Math.min(child.material.roughness + 0.4, 1);
          child.material.metalness = Math.max(child.material.metalness - 0.3, 0);
          child.material.needsUpdate = true;

          // Hide Shield mesh initially — will fade in after RimGlow appears
          if (child.name === "Shield") {
            child.material = child.material.clone();
            child.material.transparent = true;
            child.material.opacity = 0;
            shieldMeshes.push(child);
          }
        }
      });

      scene.add(model);

      // Fade Shield in after 700ms
      setTimeout(() => {
        shieldFading = true;
        shieldStartTime = performance.now();
      }, 700);

      // ── Hexagons_Group — emerge from scale 0 ────────────────────────────
      hexGroupRef = model.getObjectByName("Hexagons_Group");
      if (hexGroupRef) hexGroupRef.scale.set(0, 0, 0);

      // ── Hex_center — scale 0 like the others, rises after ──────────────
      hexCenterRef = model.getObjectByName("Hex_center");
      if (hexCenterRef) hexCenterRef.scale.set(0, 0, 0);

      // Phase 1 — hexagons emerge after shield fully visible
      // RimGlow: 0ms, Shield fade: 700ms → 1500ms, Hexagons: 1400ms
      setTimeout(() => {
        hexAnimating = true;
        hexStartTime = performance.now();
      }, 1400);

      // Phase 2 — Hex_center rises up from its default position, detached from model
      // 1400 (hexagons start) + 400 (hexagons anim) + 400 (pause) = 2200ms
      setTimeout(() => {
        if (!hexCenterRef || popPhase !== 0) return;

        // Capture current world transform (after emerge animation + rotation)
        hexCenterRef.updateWorldMatrix(true, false);
        hexCenterRef.getWorldPosition(popStartPos);
        hexCenterRef.getWorldQuaternion(popStartQuat);
        hexCenterRef.getWorldScale(popStartScale);

        // Detach from model → reparent to scene at current position
        hexCenterRef.removeFromParent();
        hexCenterRef.position.copy(popStartPos);
        hexCenterRef.quaternion.copy(popStartQuat);
        hexCenterRef.scale.copy(popStartScale);
        scene.add(hexCenterRef);

        // Target is always above start position — never descends
        popTargetPos.set(0, popStartPos.y + 1.2, 0);

        popStartTime = performance.now();
        popPhase = 1;
      }, 2200);
    },
    undefined,
    (err) => console.error("GLB load error:", err),
  );

  const onResize = () => {
    camera.aspect = container.clientWidth / container.clientHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(container.clientWidth, container.clientHeight);
  };
  window.addEventListener("resize", onResize);

  // ── Scroll tilt ────────────────────────────────────────────────────────────
  let scrollTiltTarget  = 0;
  let scrollTiltCurrent = 0;
  const MAX_TILT = Math.PI / 6; // 30°

  const onScroll = () => {
    const t = Math.min(window.scrollY / 400, 1);
    scrollTiltTarget = t * MAX_TILT;
  };
  window.addEventListener("scroll", onScroll);

  // ── Render loop ────────────────────────────────────────────────────────────
  let animId;
  const animate = () => {
    animId = requestAnimationFrame(animate);

    // Shield fade-in after RimGlow
    if (shieldFading && shieldMeshes.length > 0) {
      const elapsed = performance.now() - shieldStartTime;
      const t = Math.min(elapsed / SHIELD_FADE_DURATION, 1);
      const opacity = easeOutCubic(t);
      for (const m of shieldMeshes) m.material.opacity = opacity;
      if (t >= 1) shieldFading = false;
    }

    // Phase 1 — Hexagons_Group + Hex_center scale from 0 → 1 together
    if (hexAnimating) {
      const elapsed = performance.now() - hexStartTime;
      const t       = Math.min(elapsed / 400, 1);
      const s       = easeOutCubic(t);
      if (hexGroupRef)  hexGroupRef.scale.setScalar(s);
      if (hexCenterRef && popPhase === 0) hexCenterRef.scale.setScalar(s);
      if (t >= 1) hexAnimating = false;
    }

    // Phase 2 — Hex_center floats up, always faces camera (billboard)
    if (popPhase === 1 && hexCenterRef) {
      const elapsed = performance.now() - popStartTime;
      const t       = Math.min(elapsed / 1000, 1);
      hexCenterRef.position.lerpVectors(popStartPos, popTargetPos, easeInOutCubic(t));
      if (t >= 1) popPhase = 2;
    }

    // Phase 2 — start smooth billboard transition once rising is done
    if (popPhase === 2 && hexCenterRef) {
      billboardStartQuat.copy(hexCenterRef.quaternion);
      billboardStartTime = performance.now();
      popPhase = 3;
    }

    // Phase 3 — slerp towards camera-facing quat over BILLBOARD_DURATION
    if (popPhase === 3 && hexCenterRef) {
      const camDir = new THREE.Vector3()
        .subVectors(camera.position, hexCenterRef.position)
        .normalize();
      const targetQuat = new THREE.Quaternion().setFromUnitVectors(
        new THREE.Vector3(0, 1, 0),
        camDir,
      );
      const elapsed = performance.now() - billboardStartTime;
      const t = Math.min(elapsed / BILLBOARD_DURATION, 1);
      hexCenterRef.quaternion.slerpQuaternions(billboardStartQuat, targetQuat, easeInOutCubic(t));
      if (t >= 1) popPhase = 4;
    }

    // Phase 4 — continuous billboard (instant, transition done)
    if (popPhase === 4 && hexCenterRef) {
      const camDir = new THREE.Vector3()
        .subVectors(camera.position, hexCenterRef.position)
        .normalize();
      hexCenterRef.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), camDir);
    }

    // Scroll tilt — smooth lerp toward target
    scrollTiltCurrent += (scrollTiltTarget - scrollTiltCurrent) * 0.05;

    controls.update();

    // Élève la caméra sans changer le rayon → toujours face à l'origine
    const radius  = 5;
    const newY    = Math.sin(scrollTiltCurrent) * radius;
    const xzDist  = Math.cos(scrollTiltCurrent) * radius;
    const azimuth = Math.atan2(camera.position.x, camera.position.z);
    camera.position.set(Math.sin(azimuth) * xzDist, newY, Math.cos(azimuth) * xzDist);
    camera.lookAt(0, 0, 0);

    renderer.render(scene, camera);
  };
  animate();

  return () => {
    cancelAnimationFrame(animId);
    window.removeEventListener("resize", onResize);
    window.removeEventListener("scroll", onScroll);
    controls.dispose();
    renderer.dispose();
    if (container.contains(renderer.domElement))
      container.removeChild(renderer.domElement);
  };
}
