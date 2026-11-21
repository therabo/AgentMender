import { useEffect, useRef, useState, type ReactNode } from 'react'
import { RefreshCw } from 'lucide-react'
import { InventoryHeader } from '../../components/layout/InventoryHeader'
import { EmptyState, LoadingState, Notice, Pagination } from '../../components/ui/feedback'
import { GlassButton, GlassPanel, SearchField, SelectField } from '../../components/ui/glass'
import { errorText } from '../../lib/api/client'
import { usePagedResource } from '../../lib/hooks/usePagedResource'
import { numberLabel } from '../../lib/utils/format'
import { exportSelectionLimit, exportsPath, listExports, refreshExports, type Outcome } from './api'
import { categoryLabel } from './category'
import { ExportOutcome, ExportRemoval } from './ExportRemoval'
import { ExportTable } from './ExportTable'

const pageSize = 20

export function Exports({ heading, switched }: { heading: ReactNode; switched: boolean }) {
  const [search, setSearch] = useState('')
  const [sort, setSort] = useState('size')
  const [category, setCategory] = useState('')
  const [pageIndex, setPageIndex] = useState(0)
  const [revision, setRevision] = useState(0)
  const [checked, setChecked] = useState<ReadonlySet<string>>(() => new Set())
  const [outcome, setOutcome] = useState<Outcome>()
  const [review, setReview] = useState<string[]>()
  const [refreshError, setRefreshError] = useState('')
  const [refreshing, setRefreshing] = useState(false)
  const refreshPending = useRef(false)
  const submittedReview = useRef(false)
  const query = new URLSearchParams({ search: search.trim(), sort })
  if (category) query.set('category', category)
  const { page, loading, error } = usePagedResource(exportsPath, query.toString(), pageIndex, pageSize, revision, listExports, { delay: 150 })

  useEffect(() => {
    if (!loading && !error && page && pageIndex > 0 && pageIndex * pageSize >= page.total) setPageIndex(0)
  }, [loading, error, page, pageIndex])

  function invalidate() {
    setPageIndex(0)
    setRevision(value => value + 1)
  }

  function toggle(key: string) {
    setChecked(previous => {
      if (!previous.has(key) && previous.size >= exportSelectionLimit) return previous
      const next = new Set(previous)
      if (!next.delete(key)) next.add(key)
      return next
    })
  }

  function clearSelection() {
    setChecked(new Set())
    window.requestAnimationFrame(() => document.querySelector<HTMLElement>('.exports-panel')?.focus({ preventScroll: true }))
  }

  async function refresh() {
    if (refreshPending.current || review) return
    refreshPending.current = true
    setRefreshing(true)
    setRefreshError('')
    try {
      await refreshExports()
      setChecked(new Set())
      invalidate()
    } catch (failure) {
      setRefreshError(errorText(failure))
    } finally {
      refreshPending.current = false
      setRefreshing(false)
    }
  }

  function recordOutcome(result?: Outcome) {
    submittedReview.current = true
    setOutcome(result)
    invalidate()
  }

  function closeReview() {
    setReview(undefined)
    if (submittedReview.current) clearSelection()
  }

  return <>
    <GlassPanel className="inventory-panel exports-panel" data-switched={switched} aria-label="Exports inventory" tabIndex={-1}>
      <InventoryHeader heading={heading}
        count={page ? `${numberLabel(page.total)} ${page.complete ? '' : 'observed '}${page.total === 1 ? 'export group' : 'export groups'}` : '— export groups'}
        action={<GlassButton className="square" tone="quiet" aria-label="Refresh exports" title="Refresh exports"
          disabled={refreshing || loading || !!review} aria-busy={refreshing} onClick={() => void refresh()}>
          <RefreshCw className={refreshing ? 'spin' : undefined} size={15} aria-hidden />
        </GlassButton>}>
        <SearchField label="Search exports" maxLength={64} placeholder="Search exports by name or root…" value={search}
          onChange={value => { setSearch(value); setPageIndex(0) }} />
        <SelectField label="Sort exports" value={sort} onChange={value => { setSort(value); setPageIndex(0) }}>
          <option value="size">Largest first</option><option value="modified">Last modified</option>
        </SelectField>
        <SelectField label="Export category" value={category} onChange={value => { setCategory(value); setPageIndex(0) }}>
          <option value="">All export types</option>
          {Object.entries(categoryLabel).map(([value, label]) => <option value={value} key={value}>{label}</option>)}
        </SelectField>
      </InventoryHeader>
      <div className="inventory-body" aria-busy={loading}>
        {refreshError && <Notice tone="error">{refreshError}</Notice>}
        {outcome && <ExportOutcome outcome={outcome} />}
        {error ? <Notice tone="error">{error}</Notice> :
          loading ? <LoadingState title="Loading export groups" /> :
          page?.rows.length ? <ExportTable rows={page.rows} checked={checked} disabled={refreshing || !!review} onCheck={toggle} /> :
          <EmptyState title={search || category ? 'No matching export groups' : 'No export groups observed'} />}
        {page && !loading && !error && <Pagination total={page.total} page={pageIndex} pageSize={pageSize} onPage={setPageIndex} label="Export group pages" />}
      </div>
      {checked.size > 0 && <footer className="export-selection">
        <span role="status">{numberLabel(checked.size)} selected{checked.size === exportSelectionLimit ? ' · limit reached' : ''}</span>
        <GlassButton tone="quiet" disabled={refreshing || !!review} onClick={clearSelection}>Clear</GlassButton>
        <GlassButton tone="danger" disabled={refreshing || loading || !!error} onClick={event => {
          if (refreshPending.current || review) return
          event.currentTarget.focus()
          setOutcome(undefined)
          submittedReview.current = false
          setReview([...checked])
        }}>Remove selected</GlassButton>
      </footer>}
    </GlassPanel>
    {review && <ExportRemoval keys={review} onClose={closeReview} onOutcome={recordOutcome} />}
  </>
}
