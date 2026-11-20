import { StatCard } from '../components/ui/glass'
import type { Provider, ProviderInfo } from '../lib/api/contracts'
import { numberLabel, sizeLabel } from '../lib/utils/format'

export function ArchiveStats({ provider, info }: { provider: Provider; info?: ProviderInfo }) {
  const readable = info?.coverage === 'partial'
  const projects = readable ? `${info.projects_incomplete ? '≥ ' : ''}${numberLabel(info.registered_projects)}` : '—'
  const sessions = readable ? `${info.inventory_incomplete ? '≥ ' : ''}${numberLabel(info.total)}` : '—'
  const storage = info?.transcript_bytes != null
    ? `${info.transcript_bytes_incomplete ? '≥ ' : ''}${sizeLabel(info.transcript_bytes)}`
    : '—'

  return <div className="archive-stats" aria-label="Archive summary">
    <StatCard label={provider === 'codex' ? 'Registered projects' : 'Observed projects'} value={projects} />
    <StatCard label="Sessions in archive" value={sessions} />
    <StatCard label="Observed transcript storage" value={storage}
      description="Total known logical bytes of observed main-session transcripts." />
  </div>
}
