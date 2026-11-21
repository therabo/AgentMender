import { StatCard } from '../components/ui/glass'
import type { Provider, ProviderInfo } from '../lib/api/contracts'
import { numberLabel, sizeLabel } from '../lib/utils/format'

export function ArchiveStats({ provider, info }: { provider: Provider; info?: ProviderInfo }) {
  const available = info?.coverage === 'available'
  const projects = !info ? '—' : available ? numberLabel(info.project_count) : 'Unavailable'
  const sessions = !info ? '—' : available ? numberLabel(info.total) : 'Unavailable'
  const storage = info ? sizeLabel(info.observed_transcript_bytes) : '—'
  const registered = provider === 'codex' && !info?.projects_observed_only

  return <div className="archive-stats" aria-label="Archive summary">
    <StatCard label={registered ? 'Registered projects' : 'Observed projects'} value={projects} />
    <StatCard label={info?.sessions_observed_only ? 'Observed sessions' : 'Sessions in archive'} value={sessions} />
    <StatCard label="Observed transcript storage" value={storage}
      description="Measured bytes of distinct main-session transcripts in the observed inventory. Transcripts with unknown size are excluded." />
  </div>
}
