import type { ButtonHTMLAttributes, HTMLAttributes, ReactNode } from 'react'
import closeIcon from '../../assets/close-preview.svg'
import searchIcon from '../../assets/search.svg'
import selectIcon from '../../assets/select-chevron.svg'

export function GlassPanel({ className = '', ...props }: HTMLAttributes<HTMLElement>) {
  return <section className={`glass ${className}`} {...props} />
}

type GlassButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & { tone?: 'danger' | 'quiet' }
export function GlassButton({ className = '', tone, type = 'button', ...props }: GlassButtonProps) {
  return <button type={type} className={`button ${tone ?? ''} ${className}`} {...props} />
}

export function ClosePreview({ onClick }: { onClick: () => void }) {
  return <GlassButton className="preview-close square" aria-label="Close preview" data-preview-close onClick={onClick}>
    <img src={closeIcon} alt="" width={18} height={18} />
  </GlassButton>
}

export function SegmentedControl<T extends string>({ label, items, value, onChange, className = '' }: {
  label: string
  items: readonly { value: T; label: string }[]
  value: T
  onChange: (value: T) => void
  className?: string
}) {
  return <div className={`segmented ${className}`} role="group" aria-label={label} data-side={items[0]?.value === value ? 'first' : 'last'}>
    {items.map(item => <button type="button" key={item.value} aria-pressed={value === item.value}
      onClick={() => onChange(item.value)}>{item.label}</button>)}
  </div>
}

export function SearchField({ label, placeholder, value, onChange, maxLength = 128 }: {
  label: string; placeholder: string; value: string; onChange: (value: string) => void; maxLength?: number
}) {
  return <label className="search-field">
    <img src={searchIcon} alt="" width={18} height={18} />
    <span className="sr-only">{label}</span>
    <input type="search" placeholder={placeholder} value={value} maxLength={maxLength} onChange={event => onChange(event.target.value)} />
  </label>
}

export function SelectField({ label, value, onChange, children }: {
  label: string; value: string; onChange: (value: string) => void; children: ReactNode
}) {
  return <label className="select-field">
    <span className="sr-only">{label}</span>
    <select value={value} onChange={event => onChange(event.target.value)}>{children}</select>
    <img src={selectIcon} alt="" width={16} height={16} />
  </label>
}

export function StatCard({ label, value, description }: { label: string; value: string; description?: string }) {
  return <GlassPanel className="stat-card">
    <span className="stat-orb" aria-hidden />
    <div title={description}><strong>{value}</strong><span>{label}</span></div>
  </GlassPanel>
}

export function StatusBadge({ tone, children }: { tone: string; children: ReactNode }) {
  return <span className={`status-badge status-${tone}`}>{children}</span>
}
