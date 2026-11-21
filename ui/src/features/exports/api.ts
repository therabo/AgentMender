import { request } from '../../lib/api/client'

export type Protection =
  | 'provider_managed' | 'independent_backup' | 'user_output' | 'unknown_owner'
  | 'shared' | 'in_use' | 'incomplete'

type Size = { logical_bytes: number; files: number }
export type Group = {
  key: string; name: string; path: string; size: Size
  modified_at: number | null
  protection: Protection; reason: string; removal_available: boolean; complete: boolean
}
export type Page = { rows: Group[]; total: number; complete: boolean }
export type Preview = { groups: Group[]; size: Size; protected: number; removable: number; complete: boolean; receipt: string | null }
export type Outcome = { deleted: { key: string }[]; not_deleted: { key: string; reason: string; error: boolean }[] }
export const exportSelectionLimit = 32
export const exportsPath = '/api/storage/exports'

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

export function confirmExports(receipt: string, signal: AbortSignal) {
  return request<Outcome>(`${exportsPath}/operations/${encodeURIComponent(receipt)}/confirm`, {
    method: 'POST',
    body: JSON.stringify({ confirm: true }),
    signal,
  })
}
