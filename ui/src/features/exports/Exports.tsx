import { useEffect, useRef, useState, type ReactNode } from 'react'
import { RefreshCw } from 'lucide-react'
import { InventoryHeader } from '../../components/layout/InventoryHeader'
import { SplitWorkspace } from '../../components/layout/SplitWorkspace'
import { EmptyState, LoadingState, Notice, Pagination } from '../../components/ui/feedback'
import { GlassButton, GlassPanel, SearchField, SelectField } from '../../components/ui/glass'
import { errorText } from '../../lib/api/client'
import { usePagedResource } from '../../lib/hooks/usePagedResource'
import { numberLabel, sizeLabel } from '../../lib/utils/format'
import { exportSelectionLimit, exportsPath, listExports, refreshExports, type Outcome } from './api'
import { categoryLabel } from './category'
import { CodexDiscovery } from './CodexDiscovery'
import { ExportDetail } from './ExportDetail'
import { ExportOutcome, ExportRemoval } from './ExportRemoval'
import { ExportTable } from './ExportTable'
import { useExportSelection } from './hooks/useExportSelection'

const pageSize = 20

export function Exports({ heading, switched }: { heading: ReactNode; switched: boolean }) {
  const [search, setSearch] = useState('')
  const [sort, setSort] = useState('size')
  const [category, setCategory] = useState('')
  const [pageIndex, setPageIndex] = useState(0)
  const [revision, setRevision] = useState(0)
  const [outcome, setOutcome] = useState<Outcome>()
  const [review, setReview] = useState<string[]>()
  const [refreshError, setRefreshError] = useState('')
  const [refreshing, setRefreshing] = useState(false)
  const [discoveryOpened, setDiscoveryOpened] = useState(false)
  const refreshPending = useRef(false)
  const submittedReview = useRef(false)
  const selection = useExportSelection()
  const { preview, checked } = selection
  const query = new URLSearchParams({ search: search.trim(), sort })
  if (category) query.set('category', category)
  const { page, loading, error } = usePagedResource(exportsPath, query.toString(), pageIndex, pageSize, revision, listExports, { delay: 150 })
  const group = preview.selection?.item

  useEffect(() => {
    if (!loading && !error && page && pageIndex > 0 && pageIndex * pageSize >= page.total) setPageIndex(0)
  }, [loading, error, page, pageIndex])

  function invalidate() {
    setPageIndex(0)
    setRevision(value => value + 1)
  }

  async function refresh() {
    if (refreshPending.current) return
    refreshPending.current = true
    setRefreshing(true)
    setRefreshError('')
    try {
      await refreshExports()
      selection.clear()
      invalidate()
    } catch (failure) {
      setRefreshError(errorText(failure))
    } finally {
      refreshPending.current = false
      setRefreshing(false)
    }
  }

  function reviewSelected() {
    setOutcome(undefined)
    submittedReview.current = false
    setReview([...checked])
  }

  function recordOutcome(result?: Outcome) {
    submittedReview.current = true
    setOutcome(result)
    invalidate()
  }

  function closeReview() {
    setReview(undefined)
    if (submittedReview.current) selection.clear()
  }

  return <>
    <SplitWorkspace label="Export details" open={preview.open} opener={preview.selection?.opener} onClose={preview.close}
      preview={group && <ExportDetail group={group} canReview={!loading && checked.has(group.key)} onReview={reviewSelected} onClose={preview.close} />}>
      <GlassPanel className="inventory-panel exports-panel" data-switched={switched} aria-label="Exports inventory" tabIndex={-1}>
        <InventoryHeader heading={heading} count={page ? `${page.complete ? '' : '≥ '}${numberLabel(page.total)} export groups` : '— export groups'}>
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
            page?.rows.length ? <ExportTable rows={page.rows} checked={checked} active={preview.open ? group?.key : undefined}
              onCheck={selection.toggle} onInspect={selection.inspect} /> :
            <EmptyState title={search || category ? 'No matching export groups' : 'No export groups observed'} />}
          {page && !loading && !error && <Pagination total={page.total} page={pageIndex} pageSize={pageSize} onPage={setPageIndex} label="Export group pages" />}
        </div>
        <footer className="inventory-footer export-footer">
          <span>{checked.size > 0
            ? `${numberLabel(checked.size)} export ${checked.size === 1 ? 'group' : 'groups'} selected${checked.size === exportSelectionLimit ? ' · limit reached' : ''}`
            : 'Select export groups to review removal'}</span>
          {checked.size > 0 && <>
            {(!preview.open || !group || !checked.has(group.key)) &&
              <GlassButton tone="quiet" onClick={reviewSelected} disabled={loading}>Review selection</GlassButton>}
            <GlassButton tone="quiet" onClick={selection.clear}>Clear</GlassButton>
          </>}
          <GlassButton className="square refresh-button" tone="quiet" aria-label="Refresh exports" title="Refresh exports"
            disabled={refreshing} onClick={() => void refresh()}><RefreshCw className={refreshing ? 'spin' : undefined} size={15} aria-hidden /></GlassButton>
        </footer>
      </GlassPanel>
    </SplitWorkspace>
    <details className="inventory-diagnostics" onToggle={event => { if (event.currentTarget.open) setDiscoveryOpened(true) }}>
      <summary>{page?.complete === false ? 'Partial export coverage · storage diagnostics' : 'Export storage and diagnostics'}</summary>
      {page && <p>{numberLabel(page.group_count)} groups · {sizeLabel(page.size.logical_bytes)} logical · {sizeLabel(page.size.allocated_bytes)} allocated
        {page.complete ? '' : ' · Observed entries only'}</p>}
      {discoveryOpened && <CodexDiscovery />}
    </details>
    {review && <ExportRemoval keys={review} onClose={closeReview} onOutcome={recordOutcome} />}
  </>
}
