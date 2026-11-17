export type Provider = 'codex' | 'claude'
export type Coverage = 'partial' | 'unavailable' | 'not_detected'
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
}
export type Preview = {
  id: string
  confirmation: string
  valid_seconds: number
  main_id: string
  main_title: string | null
  spawned_descendants: number
  total_sessions: number
}
export type Operation = {
  native_id: string
  status: 'preparing' | 'running' | 'partial' | 'refused' | 'uncertain' | 'changed' | 'verified' | 'native_uncertain' | 'verification_incomplete' | 'residual'
  message: string
  project?: { key: string; name: string; removed: Subtree[]; remaining: Subtree[]; uncertain: Subtree | null }
}
export type Transcript = {
  messages: { role: 'user' | 'assistant'; text: string }[]
  next: string | null
  complete: boolean
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
  if (response.status === 202 || response.status === 204) return undefined as T
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
export type Subtree = { main_id: string; total_sessions: number }
export type ProjectPreview = {
  id: string
  confirmation: string
  valid_seconds: number
  status: 'ready'
  provider: Provider
  project_key: string
  project_name: string
  roots: string[]
  main_conversations: number
  descendants: number
  total_sessions: number
  subtrees: Subtree[]
  scope: "sessions"
  excluded: string[]
  complete: boolean
}

export const projectPath = (provider: Provider, key: string) =>
  `/api/${provider}/projects/${encodeURIComponent(key)}`
