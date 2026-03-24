import * as THREE from 'three'
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js'
import { DRACOLoader } from 'three/addons/loaders/DRACOLoader.js'
import { OrbitControls } from 'three/addons/controls/OrbitControls.js'

export function initShield(container) {
  const scene = new THREE.Scene()

  const camera = new THREE.PerspectiveCamera(45, container.clientWidth / container.clientHeight, 0.1, 100)
  camera.position.set(0, 0, 5)

  const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true })
  renderer.setSize(container.clientWidth, container.clientHeight)
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
  renderer.outputColorSpace = THREE.SRGBColorSpace
  renderer.toneMapping = THREE.ACESFilmicToneMapping
  renderer.toneMappingExposure = 3
  container.appendChild(renderer.domElement)

  scene.add(new THREE.HemisphereLight(0xffffff, 0x444466, 4))
  const dir = new THREE.DirectionalLight(0xffffff, 10)
  dir.position.set(5, 5, 5)
  scene.add(dir)
  const fill = new THREE.DirectionalLight(0xaabbff, 6)
  fill.position.set(-5, 2, -3)
  scene.add(fill)
  const front = new THREE.DirectionalLight(0xffffff, 8)
  front.position.set(0, 0, 10)
  scene.add(front)

  const controls = new OrbitControls(camera, renderer.domElement)
  controls.enableZoom = false
  controls.enablePan = false
  controls.autoRotate = true
  controls.autoRotateSpeed = 1.5

  const draco = new DRACOLoader()
  draco.setDecoderPath('https://www.gstatic.com/draco/versioned/decoders/1.5.7/')

  const gltfLoader = new GLTFLoader()
  gltfLoader.setDRACOLoader(draco)
  gltfLoader.load(
    '/assets/shield_web.glb',
    (gltf) => {
      const model = gltf.scene
      const box = new THREE.Box3().setFromObject(model)
      model.position.sub(box.getCenter(new THREE.Vector3()))
      const size = box.getSize(new THREE.Vector3())
      model.scale.setScalar(2.5 / Math.max(size.x, size.y, size.z))

      model.traverse((child) => {
        if (child.isMesh && child.material) {
          child.material.roughness = Math.min(child.material.roughness + 0.4, 1)
          child.material.metalness = Math.max(child.material.metalness - 0.3, 0)
          child.material.needsUpdate = true
        }
      })

      scene.add(model)
    },
    undefined,
    (err) => console.error('GLB load error:', err)
  )

  const onResize = () => {
    camera.aspect = container.clientWidth / container.clientHeight
    camera.updateProjectionMatrix()
    renderer.setSize(container.clientWidth, container.clientHeight)
  }
  window.addEventListener('resize', onResize)

  let animId
  const animate = () => {
    animId = requestAnimationFrame(animate)
    controls.update()
    renderer.render(scene, camera)
  }
  animate()

  return () => {
    cancelAnimationFrame(animId)
    window.removeEventListener('resize', onResize)
    controls.dispose()
    renderer.dispose()
    if (container.contains(renderer.domElement)) container.removeChild(renderer.domElement)
  }
}
