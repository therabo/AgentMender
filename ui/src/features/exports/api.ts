import { request } from '../../lib/api/client'

export type Protection =
  | 'provider_managed' | 'independent_backup' | 'user_output' | 'unknown_owner'
  | 'shared' | 'in_use' | 'incomplete'

type Size = { logical_bytes: number; allocated_bytes: number | null; files: number; directories: number }
export type Group = {
  key: string; name: string; path: string; root: string; size: Size
  created_at: number | null; modified_at: number | null; format: string
  manifest: string | null; producer: string | null; session_id: string | null
  shared_entries: number; linked_entries: number
  protection: Protection; reason: string; removal_available: boolean; complete: boolean
}
export type Page = { rows: Group[]; total: number; group_count: number; size: Size; complete: boolean; removal_available: boolean }
export type Preview = { groups: Group[]; size: Size; protected: number; removable: number; complete: boolean; receipt: string | null }
type Removed = { key: string; name: string; root: string }
export type Outcome = { deleted: Removed[]; not_deleted: (Removed & { reason: string; error: boolean })[] }
type Location = {
  path: string; root: string; kind: string; source: string
  recognized: boolean; owned: boolean; access: string; reason: string
}
export type Discovery = {
  platform: string; architecture: string; selected_root: string
  installations: { path: string; kind: string; architecture: string | null; version: string | null }[]
  storage: Location[]
}

export const exportSelectionLimit = 32
export const exportsPath = '/api/storage/exports'

export function discoverCodex(signal: AbortSignal) {
  return request<Discovery>('/api/storage/codex', { signal })
}

export function listExports(path: string, query: URLSearchParams, signal: AbortSignal) {
  return request<Page>(`${path}?${query}`, { signal })
}

export function refreshExports() {
  return request<void>(`${exportsPath}/refresh`, { method: 'POST' })
}

export function reviewExports(keys: readonly string[], signal: AbortSignal) {
  return request<Preview>(`${exportsPath}/preview`, {
    method: 'POST',
    body: JSON.stringify({ keys }),
    signal,
  })
}

export function confirmExports(receipt: string) {
  return request<Outcome>(`${exportsPath}/operations/${encodeURIComponent(receipt)}/confirm`, {
    method: 'POST',
    body: JSON.stringify({ confirm: true }),
  })
}
