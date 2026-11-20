import type { ClaudeDiscovery } from '../../lib/api/contracts'
import { numberLabel, pathLabel } from '../../lib/utils/format'

const reasons: Record<string, string> = {
  unrecognized_locator: 'Percorso o nome del transcript non riconosciuto',
  protected_or_unreadable: 'File protetto o illeggibile',
  changed_during_scan: 'File cambiato durante la scansione',
  unknown_entrypoint: 'Origine del transcript non riconosciuta',
  identity_conflict: 'Identità in conflitto',
  unverified_fork: 'Relazione con la sessione di origine non verificata',
  missing_identity: 'Identità della sessione non verificata',
  invalid_records: 'Record malformati o incompleti',
  malformed_record: 'Record malformati',
  unterminated_record: 'Riga finale incompleta',
}

const surfaces: Record<string, string> = {
  chat: 'Claude Chat', cowork: 'Cowork', code_cloud: 'Code Cloud', ssh: 'SSH', wsl: 'WSL',
}

function Counts({ rows }: { rows: [string, number][] }) {
  return <dl className="facts diagnostic-counts">{rows.map(([label, count]) =>
    <div key={label}><dt>{label}</dt><dd>{numberLabel(count)}</dd></div>,
  )}</dl>
}

export function ClaudeDiagnostics({ discovery, incomplete }: { discovery: ClaudeDiscovery; incomplete: boolean }) {
  const { transcripts, desktop, projects_incomplete, out_of_scope } = discovery.reconciliation
  const localStatus = incomplete
    ? 'Inventario locale parziale.'
    : discovery.roots.some(root => root.projects_access === 'available')
      ? 'Nessuna lacuna rilevata nelle fonti locali analizzate.'
      : 'Nessuna root di progetto analizzata.'
  const rootState = { available: 'Analizzato', missing: 'Non rilevato', unavailable: 'Non accessibile' }
  const namedReasons = (counts: Record<string, number>): [string, number][] =>
    Object.entries(counts).map(([reason, count]) => [reasons[reason] ?? reason, count])

  return <details className="claude-discovery" lang="it">
    <summary>Diagnostica inventario Claude Code</summary>
    <p>{localStatus} I conteggi precedono i filtri della lista.</p>
    <section aria-label="Transcript Code">
      <h3>Transcript Code</h3>
      <Counts rows={[
        ['Conversazioni principali', transcripts.main],
        ['Desktop locale', transcripts.desktop_main],
        ['CLI', transcripts.cli_main],
        ['Origine mista', transcripts.mixed_main],
        ['Origine sconosciuta', transcripts.unknown_surface_main],
        ['Archiviate', transcripts.archived_main],
        ['Non archiviate', transcripts.not_archived_main],
        ['Stato di archivio sconosciuto', transcripts.unknown_archive_state_main],
        ['Subagent', transcripts.subagents],
        ['Fork', transcripts.forks],
        ['Sidechain', transcripts.sidechains],
        ['Transcript senza classificazione', transcripts.unclassified],
        ['Transcript classificati con lettura parziale', transcripts.partial],
      ]} />
      {transcripts.unclassified > 0 && <><h4>Esclusioni per motivo</h4><Counts rows={namedReasons(transcripts.exclusions)} /></>}
      {transcripts.partial > 0 && <><h4>Motivi delle letture parziali</h4><Counts rows={namedReasons(transcripts.partial_reasons)} /></>}
    </section>
    <section aria-label="Registrazioni Desktop Code">
      <h3>Registrazioni Desktop Code</h3>
      <Counts rows={[
        ['Registrazioni osservate', desktop.registrations],
        ['Con transcript associato', desktop.associated],
        ['Senza transcript candidato', desktop.without_transcript],
        ['Associazione non verificata', desktop.unresolved],
        ['Record illeggibili o non riconosciuti', desktop.unreadable_records],
      ]} />
      {desktop.without_cli_uuid > 0 && <p>Registrazioni senza UUID CLI incluso nel registro · {numberLabel(desktop.without_cli_uuid)}</p>}
      {desktop.incomplete && <p>Lettura del registro Desktop parziale.</p>}
      {desktop.roots.length === 0 && <p>Nessun registro Desktop configurato.</p>}
    </section>
    <section aria-label="Fonti locali">
      <h3>Root e installazioni</h3>
      {projects_incomplete && <p>Enumerazione delle root dei progetti parziale.</p>}
      <ul className="claude-artifacts">
        {discovery.roots.map((root, index) => <li key={index}>
          <code>{pathLabel(root.path)}</code>
          <span>{root.same_filesystem_root ? 'Stesso archivio di un’altra root rilevata' : rootState[root.projects_access]}
            {root.projects_access === 'available' && !root.same_filesystem_root && ` · ${numberLabel(root.namespaces)} namespace · ${numberLabel(root.transcript_candidates)} transcript candidati`}
            {root.enumeration_incomplete && ' · enumerazione parziale'}
          </span>
          {root.project_override && <span>Namespace configurato · {root.project_override}</span>}
        </li>)}
        {desktop.roots.map((root, index) => <li key={`desktop-${index}`}>
          <code>{pathLabel(root.path)}</code>
          <span>Registro Desktop · {rootState[root.access]}{root.incomplete && ' · lettura parziale'}</span>
        </li>)}
        {discovery.installations.map((install, index) => <li key={`install-${index}`}>
          <code>{pathLabel(install.path)}</code><span>{install.confirmed ? 'Installazione rilevata' : 'Installazione candidata'}{install.version && ` · ${install.version}`}</span>
        </li>)}
      </ul>
    </section>
    <p>Fuori copertura · {out_of_scope.map(surface => surfaces[surface] ?? surface).join(', ')}. La diagnostica copre le copie Code presenti nelle root locali analizzate.</p>
  </details>
}
