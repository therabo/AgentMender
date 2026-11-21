import { SearchField, SelectField } from '../../components/ui/glass'
import type { SessionFilters } from './hooks/useSessionFilters'

export function SessionFilterFields({ filters, onChange }: {
  filters: SessionFilters
  onChange: (key: keyof SessionFilters, value: string) => void
}) {
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
