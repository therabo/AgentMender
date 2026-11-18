import type { ClaudeOwnership, Kind, Provider, State } from './api'

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

export const claudeOwnership: Record<ClaudeOwnership, string> = {
  exclusive_verified: 'Esclusività verificata',
  shared_verified: 'Condiviso · protetto',
  unknown: 'Proprietà non verificata',
  global: 'Globale · protetto',
  user_owned_output: 'Dati utente · protetti',
  in_use: 'In uso · protetto',
}

export const claudeCategories: Record<string, string> = {
  transcript: 'Transcript', variant: 'Varianti transcript', subagent: 'Subagent', tool_results: 'Risultati dei tool',
  mcp_tasks: 'Task MCP', checkpoint: 'Checkpoint e file history', images: 'Immagini', uploads: 'Upload',
  scratchpad: 'Scratchpad', desktop_record: 'Registrazioni Desktop', sidecar: 'Sidecar', session_state: 'Stato locale',
  shared_data: 'Dati condivisi', unknown: 'Artefatti sconosciuti', project_artifacts: 'Archivi di progetto',
  desktop_code: 'Registro Code Desktop', desktop_shared: 'Dati Desktop condivisi', runtime: 'Runtime',
  global_cache: 'Cache globali', shared_logs: 'Log condivisi', protected_backup: 'Backup protetti',
  global_state: 'Stato globale', worktrees: 'Worktree', user_output: 'Output utente', cowork: 'Cowork',
  temporary: 'Dati temporanei', unclassified: 'Non classificati',
}

export const claudeWriter: Record<string, string> = {
  claude_process_observed: 'Processo Claude osservato · esclusione non verificata',
  no_named_process_observed: 'Nessun processo omonimo osservato · esclusione non verificata',
  unavailable: 'Rilevamento non disponibile · esclusione non verificata',
}
