import { Euler, Vector3 } from 'three'

export const motionStep = 1 / 120

const linearResponse = 1 - Math.exp(-motionStep / 1.9)
const angularResponse = 1 - Math.exp(-motionStep / 2.6)

function lattice(x: number, y: number, seed: number) {
  let n = Math.imul(x, 0x9e3779b1) ^ Math.imul(y, 0x85ebca77) ^ seed
  n = Math.imul(n ^ (n >>> 16), 0x7feb352d)
  n = Math.imul(n ^ (n >>> 15), 0x846ca68b)
  return ((n ^ (n >>> 16)) >>> 0) / 0x80000000 - 1
}

function fade(t: number) {
  return t * t * t * (t * (6 * t - 15) + 10)
}

function noise(x: number, y: number, seed: number) {
  const i = Math.floor(x)
  const j = Math.floor(y)
  const u = fade(x - i)
  const v = fade(y - j)
  const a = lattice(i, j, seed)
  const b = lattice(i, j + 1, seed)
  const lower = a + (lattice(i + 1, j, seed) - a) * u
  const upper = b + (lattice(i + 1, j + 1, seed) - b) * u
  return lower + (upper - lower) * v
}

function returnVelocity(offset: number, extent: number, strength: number) {
  const edge = Math.max(0, (Math.abs(offset) / extent - 0.52) / 0.48)
  return -Math.sign(offset) * strength * edge * edge
}

export function createBubbleMotion(index: number, yaw: number) {
  const angles = new Euler(0, yaw, 0, 'YXZ')
  return {
    yaw,
    seed: Math.imul(index + 1, 0x9e3779b1),
    outwardExtent: 1.42,
    offset: new Vector3(),
    velocity: new Vector3(),
    angles,
    angularVelocity: new Vector3(),
  }
}

type BubbleMotion = ReturnType<typeof createBubbleMotion>

export function advanceBubble(
  bubble: BubbleMotion, anchor: Vector3, time: number, drift: number, depthExtent: number, lateralScale: number,
) {
  const { offset, velocity, angles, angularVelocity, yaw, seed } = bubble
  const x = (anchor.x + offset.x * lateralScale) * 0.52
  const y = (anchor.y + offset.y * lateralScale) * 0.63
  const z = (anchor.z + offset.z) * 0.47

  // Each component samples the plane perpendicular to its own axis.
  // The unbounded field has zero divergence; sliding those planes varies it over time.
  const currentX = 0.225 * noise(y + 0.055 * time, z - 0.028 * time, 0x5b9cca4f)
  const currentY = 0.146 * noise(z + 0.041 * time, x + 0.029 * time, 0x37a92761) + 0.020
  const currentZ = 0.091 * noise(x - 0.033 * time, y + 0.046 * time, 0x446f98ab)

  const horizontalExtent = offset.x * anchor.x > 0 ? Math.min(1.42, bubble.outwardExtent) : 1.42
  velocity.x += (currentX * drift + returnVelocity(offset.x, horizontalExtent, 0.35) - velocity.x) * linearResponse
  velocity.y += (currentY * drift + returnVelocity(offset.y, 0.62, 0.28) - velocity.y) * linearResponse
  velocity.z += (currentZ * drift + returnVelocity(offset.z, depthExtent, 0.18) - velocity.z) * linearResponse
  offset.addScaledVector(velocity, motionStep)

  // Independent torque samples avoid locking the tumbling to the direction of travel.
  const pitch = 0.075 * noise(z + time * 0.064, x - time * 0.031, seed ^ 0x142f93cf)
  const turn = 0.128 * noise(x + time * 0.046, y + time * 0.029, seed ^ 0x593df159)
  const roll = 0.068 * noise(y - time * 0.048, z + time * 0.039, seed ^ 0x7e4f36ab)
  angularVelocity.x += (pitch * drift + returnVelocity(angles.x, 0.42, 0.15) - angularVelocity.x) * angularResponse
  angularVelocity.y += (turn * drift + returnVelocity(angles.y - yaw, 1.16, 0.19) - angularVelocity.y) * angularResponse
  angularVelocity.z += (roll * drift + returnVelocity(angles.z, 0.36, 0.14) - angularVelocity.z) * angularResponse
  angles.x += angularVelocity.x * motionStep
  angles.y += angularVelocity.y * motionStep
  angles.z += angularVelocity.z * motionStep
}
