import type { ReactNode } from 'react'
import { AlertCircle, ChevronLeft, ChevronRight, Database, LoaderCircle } from 'lucide-react'
import { numberLabel } from '../../lib/utils/format'
import { GlassButton } from './glass'

export function Notice({ children, tone = 'info' }: { children: ReactNode; tone?: 'info' | 'warning' | 'error' | 'success' }) {
  return <div className={`notice ${tone}`} role={tone === 'error' ? 'alert' : 'status'}>
    <AlertCircle size={17} aria-hidden /><div>{children}</div>
  </div>
}

export function EmptyState({ title, children }: { title: string; children?: ReactNode }) {
  return <div className="empty">
    <Database size={22} aria-hidden /><h2>{title}</h2>{children}
  </div>
}

export function LoadingState({ title }: { title: string }) {
  return <div className="loading-state" role="status"><LoaderCircle size={17} className="spin" aria-hidden />{title}…</div>
}

export function Pagination({ total, page, pageSize, onPage, label = 'Pages' }: {
  total: number; page: number; pageSize: number; onPage: (page: number) => void; label?: string
}) {
  const start = page * pageSize
  const end = Math.min(start + pageSize, total)
  if (total <= pageSize) return null
  return <footer className="pagination">
    <span>{numberLabel(start + 1)}–{numberLabel(end)} of {numberLabel(total)}</span>
    <div role="group" aria-label={label}>
      <GlassButton className="square" aria-label="Previous page" disabled={page === 0} onClick={() => onPage(page - 1)}><ChevronLeft size={16} aria-hidden /></GlassButton>
      <GlassButton className="square" aria-label="Next page" disabled={end >= total} onClick={() => onPage(page + 1)}><ChevronRight size={16} aria-hidden /></GlassButton>
    </div>
  </footer>
}
