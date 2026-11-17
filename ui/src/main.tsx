import { useEffect, useState } from 'react'
import { createRoot } from 'react-dom/client'
import { Archive, Braces, ChevronRight, Circle, HardDrive, Layers3, RefreshCw, Search, ShieldCheck, SlidersHorizontal, Sparkles } from 'lucide-react'
import { authorize, projectPath, request, type Operation, type Project, type ProjectPage, type Provider, type State } from './api'
import { Empty, Notice, Pagination, ProjectAccordion, SessionList } from './components'
import { ProjectRemoval, SessionDetail } from './detail'
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
  const [projectToRemove, setProjectToRemove] = useState<Project>()
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set())
  const [page, setPage] = useState<ProjectPage>()
  const [version, invalidate] = useState(0)
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(true)
  const [refreshing, setRefreshing] = useState(false)
  const hasFilters = !!(filters.search || filters.state || filters.since || filters.until)

  const query = new URLSearchParams({
    sort: filters.sort.replace('-asc', ''), ascending: String(filters.sort.endsWith('-asc')),
  })
  if (filters.search.trim()) query.set('search', filters.search.trim())
  if (filters.state) query.set('state', filters.state)
  if (filters.since) query.set('since', filters.since)
  if (filters.until) query.set('until', filters.until)
  const filterQuery = query.toString()

  useEffect(() => {
    const controller = new AbortController()
    setLoading(true)
    setError('')
    const timeout = window.setTimeout(() => {
      const query = new URLSearchParams(filterQuery)
      query.set('limit', String(PAGE_SIZE))
      query.set('offset', String(pageIndex * PAGE_SIZE))
      request<ProjectPage>(`/api/${provider}/projects?${query}`, { signal: controller.signal }).then(result => {
        if (controller.signal.aborted) return
        if (pageIndex > 0 && pageIndex * PAGE_SIZE >= result.total) {
          setPageIndex(0)
        } else {
          setPage(result)
        }
      }).catch(failure => { if (!controller.signal.aborted) setError(errorText(failure)) })
        .finally(() => { if (!controller.signal.aborted) setLoading(false) })
    }, 150)
    return () => { window.clearTimeout(timeout); controller.abort() }
  }, [provider, filterQuery, pageIndex, revision, version])

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
  const observed = (count: number | undefined) => info?.coverage === 'partial' && count !== undefined ? numberLabel(count) : '—'
  const emptySessions = hasFilters ? 'Nessuna conversazione corrisponde ai filtri.' : 'Nessuna conversazione osservata.'

  return <>
    <div className="headline">
      <div><div className="breadcrumb"><Archive size={15} aria-hidden /> Archivio locale <ChevronRight size={13} aria-hidden /> {providerLabel[provider]}</div>
        <h1>{provider === 'codex' ? 'Archivio Codex' : `Progetti ${providerLabel[provider]}`}</h1></div>
      <button className="button" onClick={() => void refresh()} disabled={refreshing}><RefreshCw size={16} className={refreshing ? 'spin' : undefined} aria-hidden />Aggiorna</button>
    </div>

    <div className="summary">
      {provider === 'codex' ? <>
        <div className="summary-item"><span className="summary-icon"><Layers3 size={19} aria-hidden /></span><div><span>Progetti registrati osservati</span><strong>{observed(info?.registered_projects)}</strong></div></div>
        <div className="summary-item"><span className="summary-icon"><Archive size={19} aria-hidden /></span><div><span>Conversazioni nei progetti</span><strong>{observed(info?.project_conversations)}</strong></div></div>
        <div className="summary-item"><span className="summary-icon"><HardDrive size={19} aria-hidden /></span><div><span>Conversazioni recenti</span><strong>{observed(info?.recent_conversations)}</strong></div></div>
      </> : <>
        <div className="summary-item"><span className="summary-icon"><Layers3 size={19} aria-hidden /></span><div><span>Conversazioni principali osservate</span><strong>{observed(info?.total)}</strong></div></div>
        <div className="summary-item"><span className="summary-icon"><HardDrive size={19} aria-hidden /></span><div><span>Provider</span><strong>{info ? labels[info.presence] : '—'}</strong></div></div>
        <div className="summary-item"><span className="summary-icon"><ShieldCheck size={19} aria-hidden /></span><div><span>Accesso</span><strong>{info ? 'Sola lettura' : '—'}</strong></div></div>
      </>}
    </div>
    {info && <Notice tone="warning"><strong>{info.coverage === 'partial' ? 'Copertura parziale.' : info.coverage === 'not_detected' ? 'Provider non rilevato.' : 'Archivio indisponibile.'}</strong> {info.coverage_description}{info.id === 'codex' && info.coverage === 'partial' && info.removal_reason && <p>{info.removal_reason}</p>}</Notice>}

    <section className="inventory" aria-label="Inventario dei progetti">
      <div className="inventory-heading"><h2>Progetti</h2><span>{page ? `${numberLabel(page.total)} ${page.total === 1 ? 'progetto' : 'progetti'}${info?.projects_incomplete || info?.inventory_incomplete ? ' osservati' : ''}` : '—'}</span></div>
      <div className="controls">
        <label className="search"><Search size={18} aria-hidden /><span className="sr-only">Cerca progetto, titolo, UUID o workspace</span>
          <input type="search" placeholder="Cerca progetto, titolo, UUID o workspace" value={filters.search} maxLength={128} onChange={event => filter('search', event.target.value)} /></label>
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
        {loading && !page ? <Empty title="Caricamento inventario" busy />
          : error ? <Empty title="Inventario non disponibile" />
          : page && page.rows.length > 0 ? page.rows.map(project =>
            <ProjectAccordion key={project.key} project={project} expanded={expanded.has(project.key)}
              onToggle={() => setExpanded(current => {
                const next = new Set(current)
                if (next.has(project.key)) next.delete(project.key)
                else next.add(project.key)
                return next
              })}
              onRemove={() => setProjectToRemove(project)}>
              <SessionList key={filterQuery} path={`${projectPath(provider, project.key)}/sessions`}
                filters={filterQuery} revision={`${revision}:${version}`} onSelect={setSelected}
                emptyLabel={emptySessions} paginationLabel={`Pagine delle sessioni di ${project.name}`} />
            </ProjectAccordion>)
          : <Empty title={hasFilters ? 'Nessun risultato' : info?.coverage === 'partial' ? 'Nessun progetto registrato osservato' : 'Nessun inventario disponibile'}>
            {hasFilters ? 'Modifica la ricerca o i filtri.' : info?.coverage === 'partial' ? 'L’archivio potrebbe contenere altre sessioni.' : info?.coverage_description}
          </Empty>}
      </div>
      {page && page.total > PAGE_SIZE && <Pagination total={page.total} page={pageIndex} pageSize={PAGE_SIZE} onPage={setPageIndex} />}
    </section>
    {provider === 'codex' && <section className="inventory" aria-label="Conversazioni recenti">
      <div className="inventory-heading"><h2>Recenti</h2></div>
      <div className="table-area">
        <SessionList key={filterQuery} path="/api/codex/recents" filters={filterQuery}
          revision={`${revision}:${version}`} onSelect={setSelected}
          emptyLabel={emptySessions} paginationLabel="Pagine delle conversazioni recenti" />
      </div>
    </section>}
    {projectToRemove && <ProjectRemoval project={projectToRemove} onClose={() => setProjectToRemove(undefined)}
      onConfirm={(id, phrase) => { setProjectToRemove(undefined); onConfirm(id, phrase) }} />}
    {selected && <SessionDetail key={selected} rowKey={selected} provider={provider} onClose={() => setSelected(undefined)} onConfirm={(id, phrase) => {
      setSelected(undefined)
      onConfirm(id, phrase)
    }} />}
  </>
}

const pending = (operation: Operation) => operation.status === 'preparing' || operation.status === 'running'

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
        if (pending(result)) {
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
            <Notice tone={operation?.status === 'verified' ? 'success' : operationError || (operation && !pending(operation)) ? 'warning' : 'info'}>
              <div>{posting ? 'Invio conferma…' : operation?.message ?? (operationError || 'Verifica esito…')}</div>
              {operation?.native_id && <code>{operation.native_id}</code>}
              {operation?.project && <details className="operation-project">
                <summary>{operation.project.name} · {operation.project.removed.length} sottoalberi verificati</summary>
                <ul>{operation.project.removed.map(tree => <li key={tree.main_id}>Rimosso · <code>{tree.main_id}</code> · {tree.total_sessions} sessioni</li>)}
                  {operation.project.remaining.map(tree => <li key={tree.main_id}>Rimozione non avviata · <code>{tree.main_id}</code> · {tree.total_sessions} sessioni</li>)}
                  {operation.project.uncertain && <li>{pending(operation) ? 'In esecuzione' : 'Esito incerto'} · <code>{operation.project.uncertain.main_id}</code> · {operation.project.uncertain.total_sessions} sessioni</li>}
                </ul>
              </details>}
              {operationError && <button className="button quiet" onClick={() => retryStatus(value => value + 1)}>Verifica esito</button>}
              {!posting && (operationError || (operation && !pending(operation))) && <button className="button quiet" onClick={dismiss}>Chiudi esito</button>}
            </Notice>
          </div>}
          <Inventory key={provider} provider={provider} revision={revision} onConfirm={(id, phrase) => void confirm(id, phrase)} />
        </>}
    </main>
  </div>
}

createRoot(document.getElementById('root')!).render(<App />)
