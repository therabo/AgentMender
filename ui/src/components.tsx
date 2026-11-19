import { useEffect, useId, useRef, useState, type ReactNode } from 'react'
import { AlertCircle, Archive, ChevronDown, ChevronLeft, ChevronRight, Database, Folder, RefreshCw, Trash2, X } from 'lucide-react'
import { request, type Page, type Project, type Session } from './api'
import { dateLabel, kindLabel, numberLabel, pathLabel, sizeLabel, stateLabel } from './format'

export function Notice({ children, tone = 'info' }: { children: ReactNode; tone?: 'info' | 'warning' | 'error' | 'success' }) {
  return <div className={`notice ${tone}`} role={tone === 'error' ? 'alert' : 'status'}>
    <AlertCircle size={17} aria-hidden /><div>{children}</div>
  </div>
}

export function SessionStatus({ session }: { session: Session }) {
  return <span className={`state state-${session.state}`}><i aria-hidden />{stateLabel[session.state]}</span>
}

function SessionTable({ records, onSelect }: { records: Session[]; onSelect: (key: string) => void }) {
  return <table className="sessions">
    <thead><tr><th scope="col">Sessione</th><th scope="col">Ultima attività <small>UTC</small></th><th scope="col">Transcript</th><th scope="col">Stato</th><th scope="col"><span className="sr-only">Apri</span></th></tr></thead>
    <tbody>
      {records.map(session => <tr key={session.key}>
        <td className="session-cell">
          <div className="session-icon" aria-hidden><Archive size={17} /></div>
          <div className="session-description">
            <button className="session-title" title={session.title ?? undefined} onClick={() => onSelect(session.key)}>{session.title ?? 'Senza titolo'}</button>
            <span className="session-project" title={session.workspace ?? undefined}><Folder size={13} aria-hidden />{session.project ?? session.workspace ?? 'Workspace sconosciuto'}<span className="project-separator">/</span>{kindLabel[session.kind]}</span>
            <code className="table-id" title={session.native_id}>{session.native_id}</code>
            <span className="mobile-activity">Attività · {dateLabel(session.last_active_at)}</span>
          </div>
        </td>
        <td className="date-cell">{dateLabel(session.last_active_at)}
          <span className="created">Creata · {dateLabel(session.created_at)}</span>
        </td>
        <td className="size-cell">{sizeLabel(session.transcript_bytes)}</td>
        <td><SessionStatus session={session} />{session.protected && <span className="protected-label">Protetta</span>}{session.claude && <span className="protected-label">{session.claude.metadata_complete ? 'Sola lettura' : 'Metadati parziali'}</span>}</td>
        <td><ChevronRight size={16} aria-hidden className="row-chevron" /></td>
      </tr>)}
    </tbody>
  </table>
}

export function ProjectAccordion({ project, expanded, onToggle, onRemove, children }: {
  project: Project; expanded: boolean; onToggle: () => void; onRemove: () => void; children: ReactNode
}) {
  const panelId = useId()
  const titleId = useId()
  const contextId = useId()
  const context = (project.roots.length ? project.roots : [
    project.workspace, project.namespace, project.archive_root && pathLabel(project.archive_root),
  ]).filter(Boolean).join(' · ')
  return <article className="project">
    <div className="project-header">
      <h3>
        <button type="button" className="project-toggle" aria-labelledby={titleId}
          aria-expanded={expanded} aria-controls={panelId} aria-describedby={context ? contextId : undefined} onClick={onToggle}>
          <span className="project-icon"><Folder size={19} aria-hidden /></span>
          <span className="project-title">
            <span id={titleId}>{project.name}</span>
            {context && <span className="project-context" id={contextId} title={context}>{context}</span>}
          </span>
          <span className="project-stats">
            <span>{project.incomplete ? 'Almeno ' : ''}{numberLabel(project.main_conversations)} {project.main_conversations === 1 ? 'conversazione' : 'conversazioni'}</span>
            {project.last_active_at !== null && <span title="Ultima attività (UTC)">{dateLabel(project.last_active_at)}</span>}
            {project.transcript_bytes !== null && <span title="Dimensione nota dei transcript">{project.transcript_bytes_incomplete ? '≥ ' : ''}{sizeLabel(project.transcript_bytes)}</span>}
          </span>
          <ChevronDown size={18} aria-hidden className="project-chevron" />
        </button>
      </h3>
      {project.removal_available && <button type="button" className="button square danger-outline project-remove"
        aria-label={`Rimuovi sessioni del progetto ${project.name}`} title="Anteprima rimozione delle sessioni" onClick={onRemove}>
        <Trash2 size={17} aria-hidden />
      </button>}
    </div>
    <div className="project-panel" role="group" id={panelId} aria-labelledby={titleId} hidden={!expanded}>
      {expanded && <>
        {project.removal_reason && <p className="project-reason">{project.removal_reason}</p>}
        {children}
      </>}
    </div>
  </article>
}

const SESSION_PAGE_SIZE = 12

export function SessionList({ path, filters, revision, paginationLabel, emptyLabel, onSelect }: {
  path: string; filters: string; revision: string; paginationLabel: string; emptyLabel: string
  onSelect: (key: string) => void
}) {
  const [pageIndex, setPageIndex] = useState(0)
  const [page, setPage] = useState<Page>()
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    const controller = new AbortController()
    setLoading(true)
    setError('')
    const query = new URLSearchParams(filters)
    query.set('limit', String(SESSION_PAGE_SIZE))
    query.set('offset', String(pageIndex * SESSION_PAGE_SIZE))
    request<Page>(`${path}?${query}`, { signal: controller.signal }).then(result => {
      if (controller.signal.aborted) return
      if (pageIndex > 0 && pageIndex * SESSION_PAGE_SIZE >= result.total) setPageIndex(0)
      else setPage(result)
    }).catch(failure => {
      if (!controller.signal.aborted) setError(failure instanceof Error ? failure.message : 'Sessioni non disponibili.')
    }).finally(() => {
      if (!controller.signal.aborted) setLoading(false)
    })
    return () => controller.abort()
  }, [path, filters, revision, pageIndex])

  return <div aria-busy={loading}>
    {error ? <Notice tone="error">{error}</Notice>
      : page?.rows.length ? <SessionTable records={page.rows} onSelect={onSelect} />
      : loading ? <p className="project-reason" role="status">Caricamento sessioni…</p>
      : <p className="project-reason">{emptyLabel}</p>}
    {page && page.total > SESSION_PAGE_SIZE && <Pagination
      total={page.total} page={pageIndex} pageSize={SESSION_PAGE_SIZE} onPage={setPageIndex}
      label={paginationLabel} />}
  </div>
}

export function Pagination({ total, page, pageSize, onPage, label = "Pagine" }: { total: number; page: number; pageSize: number; onPage: (page: number) => void; label?: string }) {
  const start = page * pageSize
  const end = Math.min(start + pageSize, total)
  return <footer className="pagination">
    <span>{total > 0 ? `${numberLabel(start + 1)}–${numberLabel(end)} di ${numberLabel(total)}` : '0 risultati'}</span>
    <div role="group" aria-label={label}>
      <button className="button square" aria-label="Pagina precedente" disabled={page === 0} onClick={() => onPage(page - 1)}><ChevronLeft size={18} aria-hidden /></button>
      <button className="button square" aria-label="Pagina successiva" disabled={end >= total} onClick={() => onPage(page + 1)}><ChevronRight size={18} aria-hidden /></button>
    </div>
  </footer>
}

export function Empty({ title, children, busy = false }: { title: string; children?: ReactNode; busy?: boolean }) {
  return <div className="empty" role={busy ? 'status' : undefined}>
    <div className="empty-icon" aria-hidden>{busy ? <RefreshCw className="spin" size={23} /> : <Database size={23} />}</div>
    <h2>{title}</h2>
    {children && <p>{children}</p>}
  </div>
}

export function Dialog({ children, onClose, label, className = 'drawer' }: { children: ReactNode; onClose: () => void; label: string; className?: string }) {
  const dialog = useRef<HTMLDialogElement>(null)
  useEffect(() => { dialog.current?.showModal() }, [])
  return <dialog ref={dialog} className={className} aria-labelledby={label}
    onCancel={event => { event.preventDefault(); onClose() }}
    onClick={event => { if (event.target === event.currentTarget) onClose() }}>
    <div className="dialog-inner">{children}</div>
  </dialog>
}

export function CloseButton({ onClose }: { onClose: () => void }) {
  return <button className="button square" type="button" aria-label="Chiudi" onClick={onClose}><X size={18} aria-hidden /></button>
}
