export type Provider = 'codex' | 'claude'
export type Coverage = 'partial' | 'unavailable' | 'not_detected'
export type State = 'active' | 'archived' | 'unknown'
export type Kind = 'main' | 'subagent' | 'fork' | 'unknown'

export type ProviderInfo = {
  id: Provider
  presence: 'installed' | 'state_only' | 'not_detected'
  coverage: Coverage
  coverage_description: string
  total: number
  removal_available: boolean
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
  parent: string | null
  forked_from: string | null
  protected: boolean
  removal_reason: string | null
}

export type Page = { provider: ProviderInfo; total: number; rows: Session[] }
export type Detail = {
  session: Session
  version: string | null
  conversation_preview: boolean
  children: string[]
}
export type Preview = { id: string; confirmation: string; valid_seconds: number }
export type Operation = {
  native_id: string
  status: 'running' | 'changed' | 'command_succeeded' | 'outcome_unknown'
  message: string
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
