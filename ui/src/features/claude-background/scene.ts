import {
  AnimationMixer, Box3, DirectionalLight, Group, HemisphereLight, MathUtils, Mesh,
  NeutralToneMapping, PerspectiveCamera, Scene, SkinnedMesh, SphereGeometry, Vector3, WebGLRenderer,
  type BufferGeometry, type Material, type Object3D, type Skeleton,
} from 'three'
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js'
import { clone } from 'three/addons/utils/SkeletonUtils.js'
import { advanceBubble, createBubbleMotion, motionStep } from './bubbleMotion'
import { glass } from './glass'

const motion = '(prefers-reduced-motion: reduce)'
const field = {
  profiles: {
    desktop: { bubbles: 10, mascots: 5, dpr: 1.5, pixels: 2_100_000, fps: 60 },
    compact: { bubbles: 6, mascots: 2, dpr: 1.2, pixels: 1_050_000, fps: 30 },
    phone: { bubbles: 5, mascots: 1, dpr: 1.1, pixels: 700_000, fps: 30 },
  },
  depth: {
    near: { strength: 1, drift: 1, depthExtent: 0.42 },
    middle: { strength: 0.76, drift: 0.76, depthExtent: 0.34 },
    far: { strength: 0.38, drift: 0.50, depthExtent: 0.27 },
  },
  placements: [
    { x: -0.66, y: 0.80, z: 2.6, radius: 0.77, depth: 'near', mascot: true, yaw: 0.28 },
    { x: 0.78, y: 0.81, z: -1.2, radius: 0.73, depth: 'middle', mascot: true, yaw: -0.34 },
    { x: 0.85, y: -0.75, z: 2.0, radius: 1.08, depth: 'near', mascot: true, yaw: -0.52 },
    { x: -0.87, y: -0.56, z: -5.3, radius: 1.13, depth: 'far' },
    { x: 0.92, y: 0.32, z: -3.7, radius: 0.51, depth: 'middle' },
    { x: -0.94, y: 0.27, z: -6.6, radius: 0.53, depth: 'far' },
    { x: 0.36, y: 0.83, z: -4.9, radius: 0.63, depth: 'middle', mascot: true, yaw: -0.19 },
    { x: -0.58, y: 0.55, z: -9.2, radius: 0.77, depth: 'far' },
    { x: 0.78, y: -0.31, z: -5.5, radius: 0.76, depth: 'middle' },
    { x: -0.95, y: -0.86, z: -6.8, radius: 0.56, depth: 'far', mascot: true, yaw: 0.42 },
  ],
} as const

export type ClaudeScene = {
  start: () => void
  pause: () => void
  dispose: () => void
}

export async function createScene(host: HTMLElement, bytes: ArrayBuffer, signal: AbortSignal, onLost: () => void): Promise<ClaudeScene> {
  // The pinned GLB is self-contained. No decoders or texture requests are needed.
  const model = await new GLTFLoader().parseAsync(bytes, '')
  signal.throwIfAborted()

  const canvas = document.createElement('canvas')
  const context = canvas.getContext('webgl2', { alpha: true, antialias: true, powerPreference: 'low-power' })
  if (!context) throw new Error('WebGL2 unavailable')
  const gl = context

  const geometries = new Set<BufferGeometry>()
  const materials = new Set<Material>()
  const skeletons = new Set<Skeleton>()
  function collect(root: Object3D) {
    root.traverse(object => {
      if (!(object instanceof Mesh)) return
      geometries.add(object.geometry)
      for (const material of Array.isArray(object.material) ? object.material : [object.material]) materials.add(material)
      if (object instanceof SkinnedMesh) skeletons.add(object.skeleton)
    })
  }
  collect(model.scene)

  let renderer: WebGLRenderer | undefined
  const scene = new Scene()
  const camera = new PerspectiveCamera(40, 1, 0.1, 48)
  camera.position.z = 14
  const reducedMotion = window.matchMedia(motion)
  const coarse = window.matchMedia('(pointer: coarse)')
  const mixers = new Map<Group, { mixer: AnimationMixer; avatar: Object3D }>()
  let listeners: AbortController | undefined
  let frame = 0
  let lastFrame = 0
  let motionTicks = 0
  let stepRemainder = 0
  let width = 0
  let height = 0
  let lateralScale = 1
  let profile: (typeof field.profiles)[keyof typeof field.profiles] = field.profiles.desktop
  let disposed = false
  const resize = new ResizeObserver(() => size())

  function pause() {
    listeners?.abort()
    listeners = undefined
    resize.disconnect()
    stopFrame()
  }

  function dispose() {
    if (disposed) return
    disposed = true
    pause()
    canvas.removeEventListener('webglcontextlost', onLost)
    for (const { mixer, avatar } of mixers.values()) {
      mixer.stopAllAction()
      mixer.uncacheRoot(avatar)
    }
    scene.clear()
    for (const skeleton of skeletons) skeleton.dispose()
    for (const geometry of geometries) geometry.dispose()
    for (const material of materials) material.dispose()
    renderer?.dispose()
    if (!gl.isContextLost()) gl.getExtension('WEBGL_lose_context')?.loseContext()
    canvas.remove()
  }

  const bubbles: {
    group: Group
    anchor: Vector3
    motion: ReturnType<typeof createBubbleMotion>
    mascot?: Object3D
    index: number
  }[] = []

  try {
    renderer = new WebGLRenderer({ canvas, context: gl, alpha: true })
    renderer.setClearColor(0, 0)
    renderer.toneMapping = NeutralToneMapping
    renderer.toneMappingExposure = 0.91

    scene.add(new HemisphereLight('#ffe0ef', '#61406c', 1.91))
    const key = new DirectionalLight('#fff0dd', 2.75)
    key.position.set(-4, 8, 6)
    const rim = new DirectionalLight('#f9a5d4', 2.10)
    rim.position.set(5, 1, -5)
    const fill = new DirectionalLight('#c8b5ff', 0.75)
    fill.position.set(4, -2, 5)
    scene.add(key, rim, fill)

    const idle = model.animations.find(clip => clip.name === 'Clawd / Idle')
    if (!idle) throw new Error('Clawd Idle clip missing')
    const box = new Box3()
    const envelope = new Box3()
    const sampler = new AnimationMixer(model.scene)
    sampler.clipAction(idle).play()
    const steps = Math.ceil(idle.duration * 24)
    for (let i = 0; i <= steps; i++) {
      sampler.setTime(i / steps * idle.duration)
      envelope.union(box.setFromObject(model.scene, true))
    }
    sampler.stopAllAction()
    sampler.uncacheRoot(model.scene)
    const center = envelope.getCenter(new Vector3())
    // Half of the envelope's diagonal bounds every sampled vertex at any yaw.
    const fit = 1.56 / envelope.getSize(new Vector3()).length()
    if (!Number.isFinite(fit) || fit <= 0) throw new Error('Invalid Clawd bounds')

    const sphere = new SphereGeometry(1, 40, 28)
    geometries.add(sphere)
    const shells = {
      near: glass(field.depth.near.strength),
      middle: glass(field.depth.middle.strength),
      far: glass(field.depth.far.strength),
    }
    for (const material of Object.values(shells)) materials.add(material)

    field.placements.forEach((placement, index) => {
      const group = new Group()
      const shell = new Mesh(sphere, shells[placement.depth])
      group.add(shell)
      const bubble: (typeof bubbles)[number] = {
        group, anchor: new Vector3(), motion: createBubbleMotion(index, 'yaw' in placement ? placement.yaw : 0), index,
      }
      if ('mascot' in placement) {
        const avatar = clone(model.scene)
        collect(avatar)
        avatar.scale.setScalar(fit)
        avatar.position.copy(center).multiplyScalar(-fit)
        avatar.traverse(object => {
          if (object instanceof SkinnedMesh) object.frustumCulled = false
        })
        const mixer = new AnimationMixer(avatar)
        mixer.clipAction(idle).play()
        mixer.setTime(idle.duration * (0.12 + index * 0.29))
        mixers.set(group, { mixer, avatar })
        group.add(avatar)
        bubble.mascot = avatar
      }
      scene.add(group)
      bubbles.push(bubble)
    })
    canvas.addEventListener('webglcontextlost', onLost)
    host.append(canvas)
  } catch (error) {
    dispose()
    throw error
  }

  function size() {
    if (!renderer || !listeners) return
    const nextWidth = host.clientWidth
    const nextHeight = host.clientHeight
    if (!nextWidth || !nextHeight || nextWidth === width && nextHeight === height) return
    width = nextWidth
    height = nextHeight
    const phone = width < 540
    const compact = width < 760 || coarse.matches || navigator.hardwareConcurrency <= 4
    profile = phone ? field.profiles.phone : compact ? field.profiles.compact : field.profiles.desktop
    camera.aspect = width / height
    camera.updateProjectionMatrix()
    const dpr = Math.min(devicePixelRatio, profile.dpr, Math.sqrt(profile.pixels / (width * height)))
    renderer.setPixelRatio(dpr)
    renderer.setSize(width, height, false)

    const viewportScale = Math.min(1, width / 950)
    lateralScale = viewportScale * (phone ? 0.55 : 1)
    const halfFov = Math.tan(MathUtils.degToRad(camera.fov / 2))
    let mascots = 0
    for (const { group, anchor, motion, mascot, index } of bubbles) {
      const placement = field.placements[index]
      group.visible = index < profile.bubbles
      if (mascot) mascot.visible = group.visible && mascots++ < profile.mascots
      const halfHeight = (camera.position.z - placement.z) * halfFov
      const x = phone && index === 0 ? 0.83 : phone && index === 1 ? -0.98 : placement.x
      const y = phone && index === 1 ? -0.38 : compact && index < 2 ? 0.85 : placement.y
      const halfWidth = halfHeight * camera.aspect
      anchor.set(x * halfWidth, y * halfHeight, placement.z)
      // Allocate less room outward for spheres intentionally placed at the edge.
      motion.outwardExtent = Math.max(0.26, (1 - Math.abs(x)) * halfWidth / lateralScale)
      group.scale.setScalar(placement.radius * viewportScale)
    }
    if (!document.hidden) draw(0)
  }

  function draw(dt: number) {
    stepRemainder += dt
    while (stepRemainder >= motionStep) {
      stepRemainder -= motionStep
      const time = ++motionTicks * motionStep
      for (const { anchor, motion, index } of bubbles) {
        const depth = field.depth[field.placements[index].depth]
        advanceBubble(motion, anchor, time, depth.drift, depth.depthExtent, lateralScale)
      }
    }
    for (const { group, mascot, anchor, motion, index } of bubbles) {
      if (!group.visible) continue
      group.position.set(
        anchor.x + motion.offset.x * lateralScale,
        anchor.y + motion.offset.y * lateralScale,
        anchor.z + motion.offset.z,
      )
      group.quaternion.setFromEuler(motion.angles)
      if (mascot?.visible && dt) mixers.get(group)?.mixer.update(dt * (0.82 + index * 0.07))
    }
    if (width && height) renderer?.render(scene, camera)
  }

  function stopFrame() {
    if (frame) cancelAnimationFrame(frame)
    frame = 0
    lastFrame = 0
  }

  function tick(now: number) {
    frame = requestAnimationFrame(tick)
    const step = now - lastFrame
    if (lastFrame && step < 1000 / profile.fps - 1) return
    draw(lastFrame ? Math.min(step / 1000, 0.06) : 0)
    lastFrame = now
  }

  function sync() {
    stopFrame()
    if (document.hidden || !listeners) return
    draw(0)
    if (!reducedMotion.matches) frame = requestAnimationFrame(tick)
  }

  function start() {
    if (disposed || listeners) return
    listeners = new AbortController()
    const signal = listeners.signal
    document.addEventListener('visibilitychange', sync, { signal })
    reducedMotion.addEventListener('change', sync, { signal })
    resize.observe(host)
    size()
    sync()
  }

  return { start, pause, dispose }
}
