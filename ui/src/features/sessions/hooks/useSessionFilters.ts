import { useState } from 'react'

const initial = { search: '', state: '', sort: 'activity', since: '', until: '', minSize: '', maxSize: '' }
export type SessionFilters = typeof initial

export function useSessionFilters() {
  const [filters, setFilters] = useState(initial)
  const [pageIndex, setPageIndex] = useState(0)
  const active = Object.entries(filters).some(([key, value]) => key !== 'sort' && !!value)
  const params = new URLSearchParams({
    sort: filters.sort.replace('-asc', ''),
    ascending: String(filters.sort.endsWith('-asc')),
  })
  if (filters.search.trim()) params.set('search', filters.search.trim())
  if (filters.state) params.set('state', filters.state)
  if (filters.since) params.set('since', filters.since)
  if (filters.until) params.set('until', filters.until)
  if (filters.minSize) params.set('min_bytes', String(Math.max(0, Math.round(Number(filters.minSize) * 1024))))
  if (filters.maxSize) params.set('max_bytes', String(Math.max(0, Math.round(Number(filters.maxSize) * 1024))))

  function update(key: keyof SessionFilters, value: string) {
    setFilters(current => ({ ...current, [key]: value }))
    setPageIndex(0)
  }

  function reset() {
    setFilters(initial)
    setPageIndex(0)
  }

  return { filters, query: params.toString(), active, pageIndex, setPageIndex, update, reset }
}
