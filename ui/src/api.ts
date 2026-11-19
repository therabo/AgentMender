export type Provider = 'codex' | 'claude'
type Coverage = 'partial' | 'unavailable' | 'not_detected'
export type State = 'active' | 'archived' | 'unknown'
export type Kind = 'main' | 'subagent' | 'fork' | 'unknown'

export type ProviderInfo = {
  id: Provider
  presence: 'installed' | 'state_only' | 'not_detected'
  coverage: Coverage
  inventory_incomplete: boolean
  projects_incomplete: boolean
  registered_projects: number
  project_conversations: number
  recent_conversations: number
  coverage_description: string
  total: number
  removal_available: boolean
  removal_reason: string | null
  claude_storage?: ClaudeDiscovery
}

type SourceAccess = 'available' | 'missing' | 'unavailable'

type NativePath = string | { encoding: string; hex: string }

export type ClaudeDiscovery = {
  roots: {
    path: NativePath
    source: 'default' | 'environment' | 'explicit' | 'additional'
    readable: boolean
    projects_access: SourceAccess
    namespaces: number
    same_filesystem_root: boolean
    present: boolean
    transcript_candidates: number
    enumeration_incomplete: boolean
    project_override: string | null
  }[]
  installations: { path: NativePath; source: string; version: string | null; confirmed: boolean }[]
  subagents: number
  forks: number
  sidechains: number
  desktop_metadata_roots: string[]
  reconciliation: {
    transcripts: {
      main: number
      desktop_main: number
      cli_main: number
      mixed_main: number
      unknown_surface_main: number
      archived_main: number
      not_archived_main: number
      unknown_archive_state_main: number
      subagents: number
      forks: number
      sidechains: number
      unclassified: number
      partial: number
      exclusions: Record<string, number>
      partial_reasons: Record<string, number>
    }
    desktop: {
      roots: { path: NativePath; access: SourceAccess; incomplete: boolean; registrations: number }[]
      registrations: number
      associated: number
      without_transcript: number
      unresolved: number
      without_cli_uuid: number
      unreadable_records: number
      incomplete: boolean
    }
    projects_incomplete: boolean
    out_of_scope: string[]
  }
}

type ClaudeSession = {
  metadata_complete: boolean
  modified_at: number | null
  surface: 'cli' | 'desktop' | 'mixed' | null
  title_source: 'custom' | 'generated' | 'first_request' | 'fallback'
  versions: string[]
  desktop_record: boolean
  same_uuid_elsewhere: number
}

export type ClaudeOwnership = 'exclusive_verified' | 'shared_verified' | 'unknown' | 'global' | 'user_owned_output' | 'in_use'

export type ClaudeMeasurement = {
  logical_bytes: number
  allocated_bytes: number | null
  files: number
  directories: number
  links: number
  hardlink_paths: number | null
  last_modified: number | null
  complete: boolean
}

type ClaudeArtifact = {
  category: string
  root_kind: string
  root: NativePath
  relative_path: NativePath
  identity: string | null
  ownership: ClaudeOwnership
  owner: string | null
  evidence: string
  format: string
  status: 'protected' | 'unverified' | 'read_only'
  bytes: number | null
  access: SourceAccess
  size: ClaudeMeasurement
  scope: 'standard' | 'deep' | 'preserve'
}

export type ClaudeStorage = {
  transcript_files: number
  logical_bytes: number
  subagents: number
  forks: number
  sidecar_candidates: number
  checkpoint_present: boolean
  history_matches: number | null
  history_observation: 'not_observed' | 'unverified' | 'uuid_only' | 'partial_uuid_only'
  process_registry: 'present_unverified' | 'blocked' | 'not_observed'
  writer_observation: string
  observed: ClaudeArtifact[]
  diagnostic_complete: boolean
  engine_versions: string[]
  fork_references: string[]
  categories: {
    key: string
    files: number
    logical_bytes: number
    allocated_bytes: number | null
    state: 'exclusive_verified' | 'unverified' | 'protected' | 'not_observed'
  }[]

}

export type ClaudeUsage = {
  roots: {
    path: NativePath
    kind: string
    evidence: string
    access: SourceAccess
    size: ClaudeMeasurement
    groups: {
      relative_path: NativePath
      category: string
      ownership: ClaudeOwnership
      size: ClaudeMeasurement
      formats: [string, number][]
      largest_files: { relative_path: NativePath; logical_bytes: number; allocated_bytes: number | null }[]
    }[]
  }[]
  size: ClaudeMeasurement
  complete: boolean
  entries_examined: number
  writer_observation: string
  removal_reason: string
  largest_sessions: {
    key: string
    title: string | null
    native_id: string
    namespace: string | null
    logical_bytes: number
    exclusive_logical_bytes: number
  }[]
  associated_transcript_bytes: number
  exclusive_transcript_bytes: number
  catalog_complete: boolean
}

export type Session = {
  key: string
  native_id: string
  title: string | null
  project: string | null
  workspace: string | null
  provider: Provider
  state: State
  kind: Kind
  namespace?: string
  archive_root?: NativePath
  claude?: ClaudeSession
  created_at: number | null
  last_active_at: number | null
  transcript_bytes: number | null
  protected: boolean
  removal_reason: string | null
}

export type Page = { provider: ProviderInfo; total: number; rows: Session[] }
export type Detail = {
  session: Session
  version: string | null
  conversation_preview: boolean
  claude_cleanup_supported?: boolean
}

export type CodexReview = {
  review: string
  plan: { trees: { id: string; descendants: { id: string; parent: string; archived: boolean; present: boolean; depth: number }[] }[] }
}
export type ClaudeReview = {
  review: string
  plan: {
    files: { root: NativePath; relative_path: NativePath; bytes: number }[]
    directories: { root: NativePath; relative_path: NativePath }[]
    logical_bytes: number
    history_matches: number | null
    notes: string[]
  }
}
export type DeletionResult = {
  status: 'deleted' | 'partial' | 'failed'
  message: string
  errors: string[]
}
export type Transcript = {
  messages: { role: 'user' | 'assistant'; text: string }[]
  next: string | null
  complete: boolean
}

export function errorText(failure: unknown, fallback = 'Il servizio locale non risponde.'): string {
  return failure instanceof Error ? failure.message : fallback
}

const authorizationKey = 'agentmender.authorization'

export async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
  const headers = new Headers(options.headers)
  const authorization = sessionStorage.getItem(authorizationKey)
  if (authorization) headers.set('Authorization', authorization)
  if (options.body) headers.set('Content-Type', 'application/json')
  const response = await fetch(path, { ...options, headers, cache: 'no-store', credentials: 'omit' })
  if (!response.ok) {
    const payload = await response.json().catch(() => ({}))
    throw new Error(payload.error ?? 'La richiesta non è valida o il servizio locale non risponde.')
  }
  if (response.status === 204) return undefined as T
  return response.json()
}

export async function authorize(): Promise<void> {
  const ticket = new URLSearchParams(window.location.hash.slice(1)).get('ticket')
  if (!ticket) {
    if (!sessionStorage.getItem(authorizationKey)) {
      throw new Error('Apri il link di avvio stampato da agentmender ui.')
    }
    return
  }
  history.replaceState(null, '', location.pathname + location.search)
  const session = await request<{ authorization: string }>('/api/authorize', {
    method: 'POST',
    body: JSON.stringify({ ticket }),
  })
  sessionStorage.setItem(authorizationKey, session.authorization)
}

export const sessionPath = (provider: Provider, key: string) =>
  `/api/${provider}/sessions/${encodeURIComponent(key)}`

export type Project = {
  key: string
  name: string
  provider: Provider
  workspace: string | null
  namespace: string | null
  archive_root?: NativePath
  roots: string[]
  registration: "native" | "desktop" | "native_and_desktop" | "namespace"
  main_conversations: number
  last_active_at: number | null
  transcript_bytes: number | null
  transcript_bytes_incomplete: boolean
  incomplete: boolean
  persistent_instance: boolean
  removal_available: boolean
  removal_reason: string | null
}

export type ProjectPage = { provider: ProviderInfo; total: number; rows: Project[] }
export const projectPath = (provider: Provider, key: string) =>
  `/api/${provider}/projects/${encodeURIComponent(key)}`
