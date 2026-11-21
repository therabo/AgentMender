export type Provider = 'codex' | 'claude'
type Coverage = 'available' | 'unavailable' | 'not_detected'
export type State = 'active' | 'archived' | 'unknown'
export type Kind = 'main' | 'subagent' | 'fork' | 'unknown'

export type ProviderInfo = {
  id: Provider
  coverage: Coverage
  archive_error: string | null
  total: number
  sessions_observed_only: boolean
  project_count: number
  projects_observed_only: boolean
  unassigned_sessions: number
  observed_transcript_bytes: number | null
}

type NativePath = string | { encoding: string; hex: string }

export type Session = {
  key: string
  native_id: string
  title: string | null
  state: State
  kind: Kind
  last_active_at: number | null
  transcript_bytes: number | null
  protected: boolean
  removal_reason: string | null
}

export type Page = { provider: ProviderInfo; total: number; rows: Session[] }
export type Detail = {
  session: Session
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

export type Project = {
  key: string
  name: string
  provider: Provider
  roots: string[]
  main_conversations: number
  removal_available: boolean
}

export type ProjectPage = { provider: ProviderInfo; total: number; rows: Project[] }
