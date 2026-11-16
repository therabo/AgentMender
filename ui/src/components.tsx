import { useEffect, useRef, type ReactNode } from 'react'
import { AlertCircle, Archive, ChevronLeft, ChevronRight, Database, Folder, RefreshCw, X } from 'lucide-react'
import type { Session } from './api'
import { dateLabel, kindLabel, numberLabel, sizeLabel, stateLabel } from './format'

export function Notice({ children, tone = 'info' }: { children: ReactNode; tone?: 'info' | 'warning' | 'error' | 'success' }) {
  return <div className={`notice ${tone}`} role={tone === 'error' ? 'alert' : 'status'}>
    <AlertCircle size={17} aria-hidden /><div>{children}</div>
  </div>
}

export function SessionStatus({ session }: { session: Session }) {
  return <span className={`state state-${session.state}`}><i aria-hidden />{stateLabel[session.state]}</span>
}

export function SessionTable({ records, onSelect }: { records: Session[]; onSelect: (key: string) => void }) {
  return <table className="sessions">
    <thead><tr><th scope="col">Sessione</th><th scope="col">Ultima attività <small>UTC</small></th><th scope="col">Transcript</th><th scope="col">Stato</th><th scope="col"><span className="sr-only">Apri</span></th></tr></thead>
    <tbody>
      {records.map(session => <tr key={session.key}>
        <td className="session-cell">
          <div className="session-icon" aria-hidden><Archive size={17} /></div>
          <div className="session-description">
            <button className="session-title" title={session.title ?? undefined} onClick={() => onSelect(session.key)}>{session.title ?? 'Senza titolo'}</button>
            <span className="session-project" title={session.workspace ?? undefined}><Folder size={13} aria-hidden />{session.project ?? 'Workspace sconosciuto'}<span className="project-separator">/</span>{kindLabel[session.kind]}</span>
            <code className="table-id" title={session.native_id}>{session.native_id}</code>
            <span className="mobile-activity">Attività · {dateLabel(session.last_active_at)}</span>
          </div>
        </td>
        <td className="date-cell">{dateLabel(session.last_active_at)}
          <span className="created">Creata · {dateLabel(session.created_at)}</span>
        </td>
        <td className="size-cell">{sizeLabel(session.transcript_bytes)}</td>
        <td><SessionStatus session={session} />{session.protected && <span className="protected-label">Protetta</span>}</td>
        <td><ChevronRight size={16} aria-hidden className="row-chevron" /></td>
      </tr>)}
    </tbody>
  </table>
}

export function Pagination({ total, page, pageSize, onPage }: { total: number; page: number; pageSize: number; onPage: (page: number) => void }) {
  const start = page * pageSize
  const end = Math.min(start + pageSize, total)
  return <footer className="pagination">
    <span>{total > 0 ? `${numberLabel(start + 1)}–${numberLabel(end)} di ${numberLabel(total)}` : '0 risultati'}</span>
    <div aria-label="Pagine">
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
