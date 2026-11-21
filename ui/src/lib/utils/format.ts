import type { Kind, Provider, State } from '../api/contracts'

export const providerLabel: Record<Provider, string> = { codex: 'Codex', claude: 'Claude Code' }
export const stateLabel: Record<State, string> = {
  active: 'Active',
  archived: 'Archived',
  unknown: 'Unknown',
}
export const kindLabel: Record<Kind, string> = {
  main: 'Main conversation',
  subagent: 'Subagent',
  fork: 'Fork',
  unknown: 'Unknown',
}

const calendar = new Intl.DateTimeFormat('en-US', {
  dateStyle: 'medium', timeStyle: 'short', timeZone: 'UTC',
})
const integer = new Intl.NumberFormat('en-US')
const decimal = new Intl.NumberFormat('en-US', { maximumFractionDigits: 1 })

export function dateLabel(seconds: number | null): string {
  const epoch = seconds === null ? NaN : seconds * 1000
  return Number.isNaN(new Date(epoch).getTime()) ? 'Unavailable' : calendar.format(epoch)
}

export function sizeLabel(bytes: number | null): string {
  if (bytes === null) return 'Unavailable'
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB']
  const scale = bytes > 0 ? Math.min(Math.floor(Math.log2(bytes) / 10), units.length - 1) : 0
  return `${decimal.format(bytes / 1024 ** scale)} ${units[scale]}`
}

export const numberLabel = (count: number) => integer.format(count)

export function pathLabel(path: string | { encoding: string; hex: string } | undefined) {
  return typeof path === 'string' ? path : 'Percorso non rappresentabile'
}

const relativeDate = new Intl.RelativeTimeFormat('en', { numeric: 'auto' })

export function activityLabel(seconds: number | null): string {
  if (seconds === null) return 'Unavailable'
  const elapsed = Math.round(Date.now() / 1000) - seconds
  if (elapsed < 0 || elapsed >= 28 * 86400) return dateLabel(seconds)
  if (elapsed < 60) return 'Just now'
  if (elapsed < 3600) return relativeDate.format(-Math.floor(elapsed / 60), 'minute')
  if (elapsed < 86400) return relativeDate.format(-Math.floor(elapsed / 3600), 'hour')
  return relativeDate.format(-Math.floor(elapsed / 86400), 'day')
}
