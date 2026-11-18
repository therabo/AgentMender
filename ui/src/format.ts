import type { Kind, Provider, State } from './api'

export const providerLabel: Record<Provider, string> = { codex: 'Codex', claude: 'Claude Code' }
export const stateLabel: Record<State, string> = {
  active: 'Non archiviata',
  archived: 'Archiviata',
  unknown: 'Sconosciuto',
}
export const kindLabel: Record<Kind, string> = {
  main: 'Principale',
  subagent: 'Subagent',
  fork: 'Fork',
  unknown: 'Sconosciuto',
}

const calendar = new Intl.DateTimeFormat('it-IT', {
  dateStyle: 'medium', timeStyle: 'short', timeZone: 'UTC',
})
const integer = new Intl.NumberFormat('it-IT')
const decimal = new Intl.NumberFormat('it-IT', { maximumFractionDigits: 1 })

export function dateLabel(seconds: number | null): string {
  const epoch = seconds === null ? NaN : seconds * 1000
  return Number.isNaN(new Date(epoch).getTime()) ? 'Non disponibile' : calendar.format(epoch)
}

export function sizeLabel(bytes: number | null): string {
  if (bytes === null) return 'Non disponibile'
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB']
  const scale = bytes > 0 ? Math.min(Math.floor(Math.log2(bytes) / 10), units.length - 1) : 0
  return `${decimal.format(bytes / 1024 ** scale)} ${units[scale]}`
}

export const numberLabel = (count: number) => integer.format(count)

export function pathLabel(path: string | { encoding: string; hex: string } | undefined) {
  return typeof path === 'string' ? path : 'Percorso non rappresentabile'
}
