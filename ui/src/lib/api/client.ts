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
