import { useEffect, useId, useState, type ReactNode } from 'react'
import { Trash2 } from 'lucide-react'
import downIcon from '../../assets/chevron-down.svg'
import folderIcon from '../../assets/project-folder.svg'
import unassignedIcon from '../../assets/unassigned.svg'
import dotIcon from '../../assets/session-dot.svg'
import rowChevron from '../../assets/session-chevron.svg'
import { EmptyState, LoadingState, Notice, Pagination } from '../../components/ui/feedback'
import { GlassButton, StatusBadge } from '../../components/ui/glass'
import type { Session } from '../../lib/api/contracts'
import { activityLabel, dateLabel, kindLabel, numberLabel, sizeLabel, stateLabel } from '../../lib/utils/format'
import { usePagedResource } from '../../lib/hooks/usePagedResource'
import { listSessions } from './api'

const pageSize = 12

export function SessionStatus({ session }: { session: Session }) {
  return <StatusBadge tone={session.state}>{stateLabel[session.state]}</StatusBadge>
}

type GroupProps = {
  title: string; context?: string; count: number
  expanded: boolean; onToggle: () => void; onRemove?: () => void
  unassigned?: boolean; children: ReactNode
}

export function SessionGroup({ title, context, count, expanded, onToggle, onRemove, unassigned, children }: GroupProps) {
  const groupId = useId()
  const buttonId = useId()
  return <section className="session-group" data-unassigned={!!unassigned}>
    <header className="group-heading">
      <h2>
        <button type="button" className="group-toggle" id={buttonId} aria-expanded={expanded} aria-controls={groupId} onClick={onToggle}>
          <img src={unassigned ? unassignedIcon : folderIcon} alt="" width={21} height={21} />
          <span className="row-description">
            <strong>{title}</strong>
            {context && <span title={context}>{context}</span>}
          </span>
          <span className="group-count">{numberLabel(count)} {count === 1 ? 'session' : 'sessions'}</span>
          <img className="group-chevron" src={downIcon} alt="" width={18} height={18} />
        </button>
      </h2>
      {onRemove && <GlassButton className="square project-remove" aria-label={`Review removal for ${title}`}
        title="Review removal of project sessions" onClick={onRemove}><Trash2 size={15} aria-hidden /></GlassButton>}
    </header>
    <div className="group-content" data-expanded={expanded} inert={!expanded} id={groupId} role="group" aria-labelledby={buttonId}>
      <div>{children}</div>
    </div>
  </section>
}

type ListProps = {
  path: string; query: string; revision: number; label: string; selected?: string; expanded?: boolean
  onSelect: (key: string, opener: HTMLElement) => void
}

function SessionRow({ session, selected, onSelect }: {
  session: Session; selected: boolean; onSelect: (key: string, opener: HTMLElement) => void
}) {
  const title = session.title || 'Untitled session'
  return <li>
    <button type="button" className="session-row" aria-pressed={selected} onClick={event => onSelect(session.key, event.currentTarget)}>
      <img className="session-dot" src={dotIcon} alt="" width={10} height={10} />
      <span className="row-description">
        <strong title={title}>{title}</strong>
        <span>{kindLabel[session.kind]}{session.protected && ' · Protected'}</span>
      </span>
      <time className="session-activity" title={dateLabel(session.last_active_at)}>{activityLabel(session.last_active_at)}</time>
      <span className="session-size">{sizeLabel(session.transcript_bytes)}</span>
      <span className="session-state"><SessionStatus session={session} /></span>
      <img src={rowChevron} alt="" width={16} height={16} />
    </button>
  </li>
}

export function SessionList({ path, query, revision, label, selected, onSelect, expanded = true }: ListProps) {
  const [pageIndex, setPageIndex] = useState(0)
  const { loading, error, page } = usePagedResource(path, query, pageIndex, pageSize, revision, listSessions, { enabled: expanded })
  useEffect(() => {
    if (!loading && !error && page && pageIndex > 0 && pageIndex * pageSize >= page.total) setPageIndex(0)
  }, [loading, error, page, pageIndex])

  if (error) return <Notice tone="error">{error}</Notice>
  if (loading) return <LoadingState title="Loading sessions" />
  if (!page?.rows.length) return <p className="group-empty">No sessions to display.</p>

  return <>
    <ul className="session-rows" aria-label={label}>
      {page.rows.map(session => <SessionRow key={session.key} session={session} selected={selected === session.key} onSelect={onSelect} />)}
    </ul>
    <Pagination total={page.total} page={pageIndex} pageSize={pageSize} onPage={setPageIndex} label={`Pages for ${label}`} />
  </>
}

export function UnassignedSessions({ path, query, revision, label, selected, onSelect, count, emptyTitle }: ListProps & { count: number; emptyTitle?: string }) {
  const [expanded, setExpanded] = useState(true)
  const [pageIndex, setPageIndex] = useState(0)
  const { loading, error, page } = usePagedResource(path, query, pageIndex, pageSize, revision, listSessions, { delay: 180 })
  useEffect(() => {
    if (!loading && !error && page && pageIndex > 0 && pageIndex * pageSize >= page.total) setPageIndex(0)
  }, [loading, error, page, pageIndex])

  if (error) return <Notice tone="error">{error}</Notice>
  if (loading) return emptyTitle ? <LoadingState title="Loading unassigned sessions" /> : null
  if (!page?.total) return emptyTitle ? <EmptyState title={emptyTitle} /> : null
  return <SessionGroup title="Unassigned sessions" unassigned context="Sessions without a verified project"
    count={count}
    expanded={expanded} onToggle={() => setExpanded(value => !value)}>
    <ul className="session-rows" aria-label={label}>
      {page.rows.map(session => <SessionRow key={session.key} session={session} selected={selected === session.key} onSelect={onSelect} />)}
    </ul>
    <Pagination total={page.total} page={pageIndex} pageSize={pageSize} onPage={setPageIndex} label="Unassigned session pages" />
  </SessionGroup>
}
