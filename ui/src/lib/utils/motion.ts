export function motionDuration(token: string) {
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return 0
  return Number.parseFloat(getComputedStyle(document.documentElement).getPropertyValue(token)) || 0
}
