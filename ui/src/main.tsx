import { useEffect, useState } from 'react'
import { createRoot } from 'react-dom/client'
import { Archive, Braces, ChevronRight, Circle, HardDrive, Layers3, RefreshCw, Search, ShieldCheck, SlidersHorizontal, Sparkles } from 'lucide-react'
import { authorize, request, type Operation, type Page, type Provider, type State } from './api'
import { Empty, Notice, Pagination, SessionTable } from './components'
import { SessionDetail } from './detail'
import { numberLabel, providerLabel } from './format'
import './style.css'

const PAGE_SIZE = 12
const operationKey = 'agentmender.operation'
const filtersInitial = { search: '', state: '', sort: 'activity', since: '', until: '' }
const labels = { installed: 'Installato', state_only: 'Solo archivio', not_detected: 'Non rilevato' }

function errorText(failure: unknown) {
  return failure instanceof Error ? failure.message : 'Il servizio locale non risponde.'
}

function Inventory({ provider, revision, onConfirm }: {
  provider: Provider; revision: number; onConfirm: (id: string, phrase: string) => void
}) {
  const [filters, setFilters] = useState(filtersInitial)
  const [pageIndex, setPageIndex] = useState(0)
  const [showDates, setShowDates] = useState(false)
  const [selected, setSelected] = useState<string>()
  const [page, setPage] = useState<Page>()
  const [version, invalidate] = useState(0)
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(true)
  const [refreshing, setRefreshing] = useState(false)
  const hasFilters = !!(filters.search || filters.state || filters.since || filters.until)

  useEffect(() => {
    const controller = new AbortController()
    setLoading(true)
    setError('')
    const timeout = window.setTimeout(() => {
      const query = new URLSearchParams({
        limit: String(PAGE_SIZE), offset: String(pageIndex * PAGE_SIZE),
        sort: filters.sort.replace('-asc', ''), ascending: String(filters.sort.endsWith('-asc')),
      })
      if (filters.search.trim()) query.set('search', filters.search.trim())
      if (filters.state) query.set('state', filters.state)
      if (filters.since) query.set('since', filters.since)
      if (filters.until) query.set('until', filters.until)
      request<Page>(`/api/${provider}/sessions?${query}`, { signal: controller.signal }).then(result => {
        if (pageIndex > 0 && pageIndex * PAGE_SIZE >= result.total) {
          setPageIndex(0)
        } else {
          setPage(result)
        }
      }).catch(failure => { if (!controller.signal.aborted) setError(errorText(failure)) })
        .finally(() => { if (!controller.signal.aborted) setLoading(false) })
    }, 150)
    return () => { window.clearTimeout(timeout); controller.abort() }
  }, [provider, filters, pageIndex, revision, version])

  async function refresh() {
    setRefreshing(true)
    setError('')
    try {
      await request(`/api/${provider}/refresh`, { method: 'POST' })
      setPageIndex(0)
      invalidate(value => value + 1)
    } catch (failure) {
      setError(errorText(failure))
    } finally {
      setRefreshing(false)
    }
  }
  function filter(name: keyof typeof filters, value: string) {
    setFilters(current => ({ ...current, [name]: value }))
    setPageIndex(0)
  }
  const info = page?.provider

  return <>
    <div className="headline">
      <div><div className="breadcrumb"><Archive size={15} aria-hidden /> Archivio locale <ChevronRight size={13} aria-hidden /> {providerLabel[provider]}</div>
        <h1>Sessioni {providerLabel[provider]}</h1></div>
      <button className="button" onClick={() => void refresh()} disabled={refreshing}><RefreshCw size={16} className={refreshing ? 'spin' : undefined} aria-hidden />Aggiorna</button>
    </div>

    <div className="summary">
      <div className="summary-item"><span className="summary-icon"><Layers3 size={19} aria-hidden /></span><div><span>Sessioni rilevate</span><strong>{info?.coverage === 'partial' ? numberLabel(info.total) : '—'}</strong></div></div>
      <div className="summary-item"><span className="summary-icon"><HardDrive size={19} aria-hidden /></span><div><span>Provider</span><strong>{info ? labels[info.presence] : '—'}</strong></div></div>
      <div className="summary-item"><span className="summary-icon"><ShieldCheck size={19} aria-hidden /></span><div><span>Accesso</span><strong>{info ? info.removal_available ? 'Rimozione Codex' : 'Sola lettura' : '—'}</strong></div></div>
    </div>
    {info && <Notice tone="warning"><strong>{info.coverage === 'partial' ? 'Copertura parziale.' : info.coverage === 'not_detected' ? 'Provider non rilevato.' : 'Archivio indisponibile.'}</strong> {info.coverage_description}</Notice>}

    <section className="inventory" aria-label="Inventario delle sessioni">
      <div className="inventory-heading"><h2>Inventario</h2><span>{page ? `${numberLabel(page.total)} ${page.total === 1 ? 'risultato' : 'risultati'}` : '—'}</span></div>
      <div className="controls">
        <label className="search"><Search size={18} aria-hidden /><span className="sr-only">Cerca titolo, UUID o workspace</span>
          <input type="search" placeholder="Cerca titolo, UUID o workspace" value={filters.search} maxLength={128} onChange={event => filter('search', event.target.value)} /></label>
        <label className="control"><span className="sr-only">Stato</span><select value={filters.state} onChange={event => filter('state', event.target.value as State | '')}>
          <option value="">Tutti gli stati</option><option value="active">Attive</option><option value="archived">Archiviate</option><option value="unknown">Stato sconosciuto</option>
        </select></label>
        <label className="control"><span className="sr-only">Ordina</span><select value={filters.sort} onChange={event => filter('sort', event.target.value)}>
          <option value="activity">Ultima attività</option><option value="activity-asc">Attività meno recente</option>
          <option value="created">Creazione più recente</option><option value="created-asc">Creazione meno recente</option>
          <option value="size">Transcript più grandi</option><option value="size-asc">Transcript più piccoli</option>
        </select></label>
        <button className="button date-toggle" type="button" aria-expanded={showDates} onClick={() => setShowDates(value => !value)}><SlidersHorizontal size={16} aria-hidden /> Date</button>
      </div>
      {showDates && <div className="date-controls">
        <span>Ultima attività (UTC)</span>
        <label>Da <input type="date" value={filters.since} max={filters.until || undefined} onChange={event => filter('since', event.target.value)} /></label>
        <label>A <input type="date" value={filters.until} min={filters.since || undefined} onChange={event => filter('until', event.target.value)} /></label>
      </div>}
      {hasFilters && <button className="button quiet clear-filters" onClick={() => { setFilters(filtersInitial); setPageIndex(0) }}>Azzera filtri</button>}
      {error && <div className="inventory-error"><Notice tone="error">{error}</Notice></div>}
      <div className="table-area" aria-busy={loading}>
        {loading ? <Empty title="Caricamento inventario" busy />
          : error ? <Empty title="Inventario non disponibile" />
          : page && page.rows.length > 0 ? <SessionTable records={page.rows} onSelect={setSelected} />
          : <Empty title={hasFilters ? 'Nessun risultato' : info?.coverage === 'partial' ? 'Nessuna sessione riconosciuta' : 'Nessun inventario disponibile'}>
            {hasFilters ? 'Modifica la ricerca o i filtri.' : info?.coverage === 'partial' ? 'L’archivio potrebbe contenere altre sessioni.' : info?.coverage_description}
          </Empty>}
      </div>
      <Pagination total={page?.total ?? 0} page={pageIndex} pageSize={PAGE_SIZE} onPage={setPageIndex} />
    </section>
    {selected && <SessionDetail key={selected} rowKey={selected} provider={provider} onClose={() => setSelected(undefined)} onConfirm={(id, phrase) => {
      setSelected(undefined)
      onConfirm(id, phrase)
    }} />}
  </>
}

function App() {
  const [auth, setAuth] = useState<'waiting' | 'ready' | 'failed'>('waiting')
  const [authError, setAuthError] = useState('')
  const [provider, setProvider] = useState<Provider>('codex')
  const [operationId, setOperationId] = useState(() => sessionStorage.getItem(operationKey))
  const [operation, setOperation] = useState<Operation>()
  const [operationError, setOperationError] = useState('')
  const [poll, retryStatus] = useState(0)
  const [revision, bump] = useState(0)
  const [posting, setPosting] = useState(false)
  useEffect(() => {
    authorize().then(() => setAuth('ready')).catch(failure => { setAuthError(errorText(failure)); setAuth('failed') })
  }, [])
  useEffect(() => {
    if (!operationId || auth !== 'ready' || posting) return
    const controller = new AbortController()
    let timer: number | undefined
    async function status() {
      setOperationError('')
      try {
        const result = await request<Operation>(`/api/operations/${operationId}`, { signal: controller.signal })
        if (controller.signal.aborted) return
        setOperation(result)
        if (result.status === 'running') {
          timer = window.setTimeout(() => void status(), 850)
        } else {
          bump(value => value + 1)
        }
      } catch (failure) {
        if (!controller.signal.aborted) setOperationError(errorText(failure))
      }
    }
    void status()
    return () => { controller.abort(); window.clearTimeout(timer) }
  }, [operationId, auth, poll, posting])

  async function confirm(id: string, phrase: string) {
    if (posting) return
    setPosting(true)
    setOperation(undefined)
    setOperationError('')
    sessionStorage.setItem(operationKey, id)
    setOperationId(id)
    try {
      await request(`/api/operations/${id}/confirm`, { method: 'POST', body: JSON.stringify({ confirmation: phrase }) })
    } catch (failure) {
      setOperationError(errorText(failure))
    } finally {
      setPosting(false)
    }
  }
  function dismiss() {
    sessionStorage.removeItem(operationKey)
    setOperationId(null)
    setOperation(undefined)
    setOperationError('')
  }

  return <div className="shell">
    <aside className="sidebar">
      <div className="brand"><div className="brand-icon"><Layers3 size={22} aria-hidden /></div><span>AgentMender</span></div>
      <nav aria-label="Provider">
        <p className="nav-label">Provider</p>
        {(['codex', 'claude'] as const).map(id => {
          const Icon = id === 'codex' ? Braces : Sparkles
          return <button type="button" key={id} aria-current={id === provider ? 'page' : undefined} className={`nav-item ${id === provider ? 'current' : ''}`} onClick={() => setProvider(id)}>
            <Icon size={18} aria-hidden /><span>{providerLabel[id]}</span>{id === provider && <ChevronRight className="nav-chevron" size={15} aria-hidden />}
          </button>
        })}
      </nav>
      <div className="local"><Circle size={8} fill="currentColor" aria-hidden /><span>Sessione locale</span></div>
    </aside>
    <main className="workspace">
      {auth === 'waiting' ? <Empty title="Connessione ad AgentMender" busy />
        : auth === 'failed' ? <><h1>Accesso locale</h1><Notice tone="error">{authError}</Notice></>
        : <>
          {operationId && <div className="operation">
            <Notice tone={operation?.status === 'command_succeeded' ? 'success' : operationError || operation?.status === 'outcome_unknown' || operation?.status === 'changed' ? 'warning' : 'info'}>
              <div>{posting ? 'Invio conferma…' : operation?.message ?? (operationError || 'Verifica esito…')}</div>
              {operation?.native_id && <code>{operation.native_id}</code>}
              {operationError && <button className="button quiet" onClick={() => retryStatus(value => value + 1)}>Verifica esito</button>}
              {!posting && (operationError || (operation && operation.status !== 'running')) && <button className="button quiet" onClick={dismiss}>Chiudi esito</button>}
            </Notice>
          </div>}
          <Inventory key={provider} provider={provider} revision={revision} onConfirm={(id, phrase) => void confirm(id, phrase)} />
        </>}
    </main>
  </div>
}

createRoot(document.getElementById('root')!).render(<App />)
