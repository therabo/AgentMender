import { ClaudeUsageView } from './claude-usage'
import { useEffect, useState } from 'react'
import { Archive, ChevronRight, Database, HardDrive, RefreshCw, Search, ShieldCheck } from 'lucide-react'
import { Empty, Notice, Pagination } from './components'
import { dateLabel, numberLabel, sizeLabel } from './format'
import { request } from './api'

type Size = { logical_bytes: number; allocated_bytes: number | null; files: number; directories: number }
type Group = {
  key: string; name: string; path: string; root: string; size: Size
  created_at: number | null; modified_at: number | null; format: string
  manifest: string | null; producer: string | null; session_id: string | null
  shared_entries: number; linked_entries: number
  protection: keyof typeof protection; reason: string; complete: boolean
}
type Page = { rows: Group[]; total: number; group_count: number; size: Size; complete: boolean }
type Preview = { groups: Group[]; size: Size; protected: number; removable: number; complete: boolean; reason: string }
type Location = {
  path: string; root: string; kind: string; source: string
  recognized: boolean; owned: boolean; access: string; reason: string
}
type Discovery = {
  platform: string; architecture: string; selected_root: string
  installations: { path: string; kind: string; architecture: string | null; version: string | null }[]
  storage: Location[]
}
const pageSize = 20
const protection = {
  provider_managed: 'Gestito dal provider', independent_backup: 'Backup indipendente',
  user_output: 'Output utente', unknown_owner: 'Proprietà sconosciuta',
  shared: 'Riferimenti condivisi', in_use: 'In uso', incomplete: 'Copertura parziale',
}
const storageLabels: Record<string, string> = {
  home: 'Root Codex', sessions: 'Rollout delle sessioni', archived_sessions: 'Rollout archiviati',
  state_database: 'Database di stato', history_database: 'Database cronologia', desktop_catalog: 'Catalogo Desktop',
  export_candidate: 'Candidato export', artifact_session_exports: 'Export Artifact Session', external_exports: 'Export esterni',
  attachments: 'Allegati', generated_images: 'Immagini generate', cache: 'Cache', logs: 'Log',
  worktrees: 'Worktree', user_output: 'Output utente', desktop_profile: 'Profilo Desktop',
}
function errorText(error: unknown) {
  return error instanceof Error ? error.message : 'Il servizio locale non risponde.'
}

function ExportDetails({ group }: { group: Group }) {
  return <div className="storage-detail">
    <code>{group.path}</code>
    <dl>
      <div><dt>Formato</dt><dd>{group.format}</dd></div>
      <div><dt>File e directory</dt><dd>{numberLabel(group.size.files)} file · {numberLabel(group.size.directories)} directory interne</dd></div>
      <div><dt>Allocazione osservata</dt><dd>{sizeLabel(group.size.allocated_bytes)}</dd></div>
      <div><dt>Creazione (UTC)</dt><dd>{dateLabel(group.created_at)}</dd></div>
      {group.manifest && <div><dt>Manifest</dt><dd>{group.manifest}</dd></div>}
      {group.producer && <div><dt>Produttore</dt><dd>{group.producer}</dd></div>}
      {group.session_id && <div><dt>Sessione dichiarata</dt><dd><code>{group.session_id}</code></dd></div>}
      {(group.shared_entries > 0 || group.linked_entries > 0) &&
        <div><dt>Link osservati</dt><dd>{numberLabel(group.shared_entries)} hardlink · {numberLabel(group.linked_entries)} collegamenti saltati</dd></div>}
      <div><dt>Rimozione</dt><dd>{group.reason}</dd></div>
    </dl>
  </div>
}

function Installations() {
  const [data, setData] = useState<Discovery>()
  const [error, setError] = useState('')
  useEffect(() => {
    const controller = new AbortController()
    request<Discovery>('/api/storage/codex', { signal: controller.signal }).then(setData).catch(e => {
      if (!controller.signal.aborted) setError(errorText(e))
    })
    return () => controller.abort()
  }, [])
  return <section className="inventory">
    <div className="inventory-heading"><h2>Codex rilevato</h2>{data && <span>{data.platform} · {data.architecture}</span>}</div>
    {error ? <Notice tone="error">{error}</Notice> : !data ? <Empty title="Rilevamento Codex" busy /> : <>
      <div className="storage-launch"><span>Root selezionata per le sessioni</span><code>{data.selected_root}</code></div>
      <ul className="storage-locations">
        {data.installations.map(item => <li key={item.kind + item.path}>
          <Database size={16} aria-hidden /><div><strong>{item.kind === 'desktop' ? 'Codex Desktop' : 'CLI Codex'}</strong>
            <span>{[item.version, item.architecture].filter(Boolean).join(' · ') || 'Versione non verificata'}</span><code>{item.path}</code></div>
        </li>)}
        {data.storage.map((item, index) => <li key={item.root + item.kind + item.path + index}>
          <HardDrive size={16} aria-hidden /><div>
            <strong>{storageLabels[item.kind] ?? item.kind}</strong>
            <span>{item.access === 'accessible' ? item.owned ? 'Proprietà riconosciuta' : item.recognized ? 'Formato riconosciuto' : 'Osservato' : item.access === 'linked' ? 'Collegamento protetto' : 'Inaccessibile'} · {item.source}</span>
            <code>{item.path}</code><small>{item.reason}</small>
          </div>
        </li>)}
      </ul>
    </>}
  </section>
}

export function Storage() {
  const [tab, setTab] = useState<'exports' | 'roots' | 'claude'>('exports')
  return <>
    <header className="headline">
      <div><div className="breadcrumb"><HardDrive size={15} aria-hidden /> Storage <ChevronRight size={14} aria-hidden /> {tab === 'claude' ? 'Claude Code' : 'Codex'}</div>
        <h1>{tab === 'exports' ? 'Export e backup' : tab === 'roots' ? 'Installazioni e archivi' : 'Storage Claude Code'}</h1></div>
    </header>
    <nav className="storage-tabs" aria-label="Viste storage">
      <button className={tab === 'exports' ? 'button' : 'button quiet'} aria-current={tab === 'exports' ? 'page' : undefined} onClick={() => setTab('exports')}>Export e backup</button>
      <button className={tab === 'roots' ? 'button' : 'button quiet'} aria-current={tab === 'roots' ? 'page' : undefined} onClick={() => setTab('roots')}>Installazioni e archivi</button>
      <button className={tab === 'claude' ? 'button' : 'button quiet'} aria-current={tab === 'claude' ? 'page' : undefined} onClick={() => setTab('claude')}>Storage Claude Code</button>
    </nav>
    {tab === 'exports' ? <Exports /> : tab === 'roots' ? <Installations /> : <ClaudeUsageView />}
  </>
}

function Exports() {
  const [page, setPage] = useState<Page>()
  const [offset, setOffset] = useState(0)
  const [search, setSearch] = useState('')
  const [sort, setSort] = useState('size')
  const [selected, setSelected] = useState<Set<string>>(new Set())
  const [preview, setPreview] = useState<Preview>()
  const [revision, setRevision] = useState(0)
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  useEffect(() => {
    const controller = new AbortController()
    const timer = window.setTimeout(() => {
      const params = new URLSearchParams({ search, sort, offset: String(offset), limit: String(pageSize) })
      request<Page>('/api/storage/exports?' + params, { signal: controller.signal })
        .then(data => { if (!controller.signal.aborted) setPage(data) })
        .catch(e => { if (!controller.signal.aborted) setError(errorText(e)) })
        .finally(() => { if (!controller.signal.aborted) setLoading(false) })
    }, 180)
    return () => { window.clearTimeout(timer); controller.abort() }
  }, [offset, revision, search, sort])

  function load(index = 0) {
    setLoading(true)
    setError('')
    setOffset(index)
  }
  async function refresh() {
    setBusy(true)
    setError('')
    try {
      await request('/api/storage/exports/refresh', { method: 'POST' })
      load()
      setSelected(new Set())
      setPreview(undefined)
      setRevision(value => value + 1)
    } catch (e) { setError(errorText(e)) } finally { setBusy(false) }
  }
  async function review() {
    setBusy(true)
    setError('')
    try {
      setPreview(await request<Preview>('/api/storage/exports/preview', {
        method: 'POST', body: JSON.stringify({ keys: [...selected] }),
      }))
    } catch (e) { setError(errorText(e)) } finally { setBusy(false) }
  }

  return <>
    {page && <div className="summary" aria-label="Occupazione export">
      <div className="summary-item"><span className="summary-icon"><HardDrive size={20} aria-hidden /></span><div><span>Dimensione logica osservata</span><strong>{sizeLabel(page.size.logical_bytes)}</strong></div></div>
      <div className="summary-item"><span className="summary-icon"><Database size={20} aria-hidden /></span><div><span>Allocazione osservata</span><strong>{sizeLabel(page.size.allocated_bytes)}</strong></div></div>
      <div className="summary-item"><span className="summary-icon"><Archive size={20} aria-hidden /></span><div><span>Gruppi e file di primo livello</span><strong>{numberLabel(page.group_count)}</strong></div></div>
    </div>}
    <Notice tone="info">I gruppi osservati sono protetti. La rimozione richiede un contratto verificato dell’esportatore.</Notice>
    {page && !page.complete && <div className="storage-message"><Notice tone="warning">L’inventario è parziale. Le dimensioni coprono gli elementi osservati.</Notice></div>}
    {error && <div className="storage-message"><Notice tone="error">{error}</Notice></div>}
    <section className="inventory">
      <div className="inventory-heading"><h2>Inventario export</h2><button className="button quiet" disabled={busy || loading} onClick={() => void refresh()}><RefreshCw size={15} aria-hidden /> Aggiorna</button></div>
      <div className="storage-tools">
        <label className="storage-search"><Search size={17} aria-hidden /><span className="sr-only">Cerca export</span><input value={search} placeholder="Cerca per nome o root" maxLength={64} onChange={event => { setSearch(event.target.value); load() }} /></label>
        <label><span className="sr-only">Ordina per</span><select value={sort} onChange={event => { setSort(event.target.value); load() }}><option value="size">Dimensione</option><option value="modified">Ultima modifica</option></select></label>
      </div>
      {selected.size > 0 && <div className="storage-selection">
        <span>{numberLabel(selected.size)}{selected.size === 1 ? ' gruppo selezionato' : ' gruppi selezionati'} per la verifica</span>
        <button className="button" disabled={busy || loading} onClick={() => void review()}>Anteprima selezione</button>
        <button className="button quiet" disabled={busy} onClick={() => { setSelected(new Set()); setPreview(undefined) }}>Azzera</button>
      </div>}
      {preview && <div className="storage-preview" aria-live="polite">
        <h3>Anteprima export</h3>
        <p>{numberLabel(preview.size.files)} file · {sizeLabel(preview.size.logical_bytes)} logici · {sizeLabel(preview.size.allocated_bytes)} allocati</p>
        <p>{preview.protected} protetti · {preview.removable} eliminabili{preview.complete ? '' : ' · inventario parziale'}</p>
        <ul>{preview.groups.map(item => <li key={item.key}><strong>{item.name}</strong> · {item.reason}</li>)}</ul>
        <p>{preview.reason}</p>
      </div>}
      {loading ? <Empty title="Lettura inventario export" busy /> : page?.rows.length ? <>
        <div className="storage-table-head" aria-hidden><span>Nome e protezione</span><span>Dimensione logica</span><span>Ultima modifica (UTC)</span></div>
        <ul className="export-list">
          {page.rows.map(group => <li key={group.key} className="export-row">
            <input type="checkbox" aria-label={'Includi ' + group.name + ' nella verifica'} disabled={busy || !group.complete || selected.size === 32 && !selected.has(group.key)}
              checked={selected.has(group.key)} onChange={() => {
                setSelected(previous => { const next = new Set(previous); if (!next.delete(group.key)) next.add(group.key); return next })
                setPreview(undefined)
              }} />
            <details><summary>
              <div><strong>{group.name}</strong><span className="storage-protection"><ShieldCheck size={14} aria-hidden />{protection[group.protection]}</span></div>
              <span>{sizeLabel(group.size.logical_bytes)}</span><time>{dateLabel(group.modified_at)}</time>
            </summary><ExportDetails group={group} /></details>
          </li>)}
        </ul>
        {page.total > pageSize && <Pagination page={offset / pageSize} total={page.total} pageSize={pageSize} onPage={page => load(page * pageSize)} />}
      </> : <Empty title={search ? 'Nessun risultato' : 'Nessun gruppo export osservato'} />}
    </section>
  </>
}
