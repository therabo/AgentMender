import { useEffect, useState } from 'react'
import { errorText } from '../api/client'

type PageState<T> = { page?: T; error?: string; loading: boolean }

export function usePagedResource<T extends { total: number }>(
  path: string, query: string, pageIndex: number, pageSize: number, revision: number,
  load: (path: string, query: URLSearchParams, signal: AbortSignal) => Promise<T>,
  { delay = 0, enabled = true } = {},
) {
  const [state, setState] = useState<PageState<T>>({ loading: true })

  useEffect(() => {
    if (!enabled) return
    const controller = new AbortController()
    setState(previous => ({ page: previous.page, loading: true }))
    const timer = window.setTimeout(() => {
      const params = new URLSearchParams(query)
      params.set('limit', String(pageSize))
      params.set('offset', String(pageIndex * pageSize))
      load(path, params, controller.signal)
        .then(page => { if (!controller.signal.aborted) setState({ page, loading: false }) })
        .catch(failure => {
          if (!controller.signal.aborted) setState(previous => ({ page: previous.page, error: errorText(failure), loading: false }))
        })
    }, delay)
    return () => {
      window.clearTimeout(timer)
      controller.abort()
    }
  }, [path, query, pageIndex, pageSize, revision, load, delay, enabled])

  return state
}
