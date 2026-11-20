import { request } from '../../lib/api/client'
import type { ClaudeReview, CodexReview, DeletionResult, Detail, Page, ProjectPage, Provider, ProviderInfo, Transcript } from '../../lib/api/contracts'

export const sessionPath = (provider: Provider, key: string) =>
  `/api/${provider}/sessions/${encodeURIComponent(key)}`

export const projectPath = (provider: Provider, key: string) =>
  `/api/${provider}/projects/${encodeURIComponent(key)}`

export const projectsPath = (provider: Provider) => `/api/${provider}/projects`

export const unassignedPath = (provider: Provider) => `/api/${provider}/unassigned`

export function listProjects(path: string, query: URLSearchParams, signal: AbortSignal) {
  return request<ProjectPage>(`${path}?${query}`, { signal })
}

export function listSessions(path: string, query: URLSearchParams, signal: AbortSignal) {
  return request<Page>(`${path}?${query}`, { signal })
}

export function refreshSessions(provider: Provider) {
  return request<ProviderInfo>(`/api/${provider}/refresh`, { method: 'POST' })
}

export function readSession(path: string, signal: AbortSignal) {
  return request<Detail>(path, { signal })
}

export function readConversation(path: string, signal: AbortSignal, cursor?: string) {
  const query = cursor ? `?cursor=${encodeURIComponent(cursor)}` : ''
  return request<Transcript>(`${path}/conversation${query}`, { signal })
}

export function reviewRemoval<Review extends ClaudeReview | CodexReview>(path: string, signal?: AbortSignal) {
  return request<Review>(`${path}/removal-preview`, { method: 'POST', signal })
}

export function confirmRemoval(path: string, review: string) {
  return request<DeletionResult>(`${path}/delete`, {
    method: 'POST',
    body: JSON.stringify({ confirm: true, review }),
  })
}
