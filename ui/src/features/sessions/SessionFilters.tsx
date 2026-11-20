import { GlassButton, SearchField, SelectField } from '../../components/ui/glass'
import type { Provider } from '../../lib/api/contracts'
import type { SessionFilters } from './hooks/useSessionFilters'

type Props = {
  filters: SessionFilters; provider: Provider; active: boolean
  onChange: (key: keyof SessionFilters, value: string) => void
  onReset: () => void
}

export function SessionFilterFields({ filters, onChange }: Pick<Props, 'filters' | 'onChange'>) {
  return <>
    <SearchField label="Search sessions, projects or UUIDs" placeholder="Search session, project or UUID…" value={filters.search}
      onChange={value => onChange('search', value)} />
    <SelectField label="Session state" value={filters.state} onChange={value => onChange('state', value)}>
      <option value="">All states</option><option value="active">Active</option>
      <option value="archived">Archived</option><option value="unknown">Unknown</option>
    </SelectField>
    <SelectField label="Sort sessions" value={filters.sort} onChange={value => onChange('sort', value)}>
      <option value="activity">Last activity</option><option value="activity-asc">Oldest activity</option>
      <option value="created">Newest created</option><option value="created-asc">Oldest created</option>
      <option value="size">Largest transcript</option><option value="size-asc">Smallest transcript</option>
    </SelectField>
  </>
}

export function AdvancedFilters({ filters, provider, active, onChange, onReset }: Props) {
  return <details className="advanced-filters">
    <summary>Advanced filters{(filters.since || filters.until || filters.minSize || filters.maxSize) && ' · Active'}</summary>
    <div className="date-controls">
      <label>From (UTC)<input type="date" value={filters.since} max={filters.until || undefined}
        onChange={event => onChange('since', event.target.value)} /></label>
      <label>Through (UTC)<input type="date" value={filters.until} min={filters.since || undefined}
        onChange={event => onChange('until', event.target.value)} /></label>
      {provider === 'claude' && <>
        <label>Min. KiB<input type="number" min="0" max="1000000000" step="any" value={filters.minSize}
          onChange={event => onChange('minSize', event.target.value)} /></label>
        <label>Max. KiB<input type="number" min="0" max="1000000000" step="any" value={filters.maxSize}
          onChange={event => onChange('maxSize', event.target.value)} /></label>
      </>}
      {active && <GlassButton tone="quiet" onClick={onReset}>Clear filters</GlassButton>}
    </div>
  </details>
}
