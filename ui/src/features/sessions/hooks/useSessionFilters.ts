import { useState } from 'react'

const initial = { search: '', state: '', sort: 'activity' }
export type SessionFilters = typeof initial

export function useSessionFilters() {
  const [filters, setFilters] = useState(initial)
  const [pageIndex, setPageIndex] = useState(0)
  const active = Boolean(filters.search.trim() || filters.state)
  const params = new URLSearchParams({
    sort: filters.sort.replace('-asc', ''),
    ascending: String(filters.sort.endsWith('-asc')),
  })
  if (filters.search.trim()) params.set('search', filters.search.trim())
  if (filters.state) params.set('state', filters.state)

  function update(key: keyof SessionFilters, value: string) {
    setFilters(current => ({ ...current, [key]: value }))
    setPageIndex(0)
  }

  return { filters, query: params.toString(), active, pageIndex, setPageIndex, update }
}
