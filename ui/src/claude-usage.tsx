import { useEffect, useState } from 'react'
import { RefreshCw } from 'lucide-react'
import { request, type ClaudeUsage } from './api'
import { Empty, Notice } from './components'
import { SessionDetail } from './detail'
import { claudeCategories, claudeOwnership, claudeWriter, dateLabel, numberLabel, pathLabel, sizeLabel } from './format'

const rootLabels: Record<string, string> = {
  code: 'Archivio Code', desktop: 'Dati Claude Desktop', temporary: 'Temporanei Code',
  runtime: 'Installazione', shared_cache: 'Cache condivisa', shared_logs: 'Log condivisi', global_config: 'Configurazione',
}

export function ClaudeUsageView() {
  const [report, setReport] = useState<ClaudeUsage>()
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(true)
  const [revision, setRevision] = useState(0)
  const [selected, setSelected] = useState<string>()

  useEffect(() => {
    const abort = new AbortController()
    setLoading(true)
    setError('')
    request<ClaudeUsage>('/api/storage/claude', { signal: abort.signal })
      .then(result => { if (!abort.signal.aborted) setReport(result) })
      .catch(failure => { if (!abort.signal.aborted) setError(String(failure.message)) })
      .finally(() => { if (!abort.signal.aborted) setLoading(false) })
    return () => abort.abort()
  }, [revision])

  async function refresh() {
    setLoading(true)
    setError('')
    try {
      await request('/api/storage/claude/refresh', { method: 'POST' })
      setRevision(value => value + 1)
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : 'Inventario Claude non disponibile.')
      setLoading(false)
    }
  }

  return <div className="claude-usage" aria-busy={loading}>
    <div className="inventory-heading">
      <h2>Occupazione delle root</h2>
      <button className="button" type="button" disabled={loading} onClick={() => void refresh()}>
        <RefreshCw size={16} aria-hidden />Aggiorna
      </button>
    </div>
    {error && <Notice tone="error">{error}</Notice>}
    {!report && loading && <Empty title="Scansione dello storage Claude Code" busy />}
    {report && <>
      <div className="summary">
        <div className="summary-item"><div><span>Dimensione logica osservata</span><strong>{sizeLabel(report.size.logical_bytes)}</strong></div></div>
        <div className="summary-item"><div><span>Allocazione osservata</span><strong>{sizeLabel(report.size.allocated_bytes)}</strong></div></div>
        <div className="summary-item"><div><span>File osservati</span><strong>{numberLabel(report.size.files)}</strong></div></div>
      </div>
      <Notice tone={report.complete && report.catalog_complete ? 'info' : 'warning'}>
        <p>{report.complete && report.catalog_complete ? 'Scansione completata nelle root osservate.' : 'Copertura parziale. I totali riguardano gli elementi osservati.'} {numberLabel(report.entries_examined)} elementi esaminati.</p>
        <p>L’allocazione osservata può comprendere blocchi condivisi. {claudeWriter[report.writer_observation] ?? claudeWriter.unavailable}.</p>
      </Notice>
      <section className="inventory claude-usage-panel" aria-label="Directory Claude">
        <div className="inventory-heading"><h2>Directory e file per dimensione</h2></div>
        {report.roots.map((root, index) => <details key={index} className="usage-root">
          <summary>
            <span><strong>{rootLabels[root.kind] ?? root.kind}</strong><code>{pathLabel(root.path)}</code></span>
            <span>{root.access === 'missing' ? 'Non osservata' : root.access === 'unavailable' ? 'Inaccessibile' : sizeLabel(root.size.logical_bytes)}{!root.size.complete && root.access === 'available' && ' · parziale'}</span>
          </summary>
          {root.groups.length > 0 && <ul className="claude-artifacts">
            {root.groups.map((group, groupIndex) => <li key={groupIndex}>
              <details>
                <summary>
                  <span><code>{pathLabel(group.relative_path) || 'File della root'}</code><small>{claudeCategories[group.category] ?? group.category} · {claudeOwnership[group.ownership]}</small></span>
                  <span>{sizeLabel(group.size.logical_bytes)}</span>
                </summary>
                <p>{numberLabel(group.size.files)} file · {numberLabel(group.size.directories)} directory · {sizeLabel(group.size.allocated_bytes)} allocati{!group.size.complete && ' · copertura parziale'}</p>
                <p>{numberLabel(group.size.links)} collegamenti · {group.size.hardlink_paths === null ? 'Hardlink non misurati' : `${numberLabel(group.size.hardlink_paths)} percorsi con hardlink`} · Modifica {dateLabel(group.size.last_modified)}</p>
                {group.formats.length > 0 && <p>Formati · {group.formats.map(([ext, count]) => `${ext} (${numberLabel(count)})`).join(', ')}</p>}
                {group.largest_files.length > 0 && <ul className="usage-files">
                  {group.largest_files.map((file, i) => <li key={i}><code>{pathLabel(file.relative_path) || 'File della root'}</code> · {sizeLabel(file.logical_bytes)}</li>)}
                </ul>}
              </details>
            </li>)}
          </ul>}
        </details>)}
      </section>
      <section className="inventory claude-usage-panel" aria-label="Ingombro delle conversazioni">
        <div className="inventory-heading"><h2>Sessioni per dimensione dei transcript</h2></div>
        <p>Transcript associati · {sizeLabel(report.associated_transcript_bytes)}. Esclusività verificata nel catalogo · {sizeLabel(report.exclusive_transcript_bytes)}.</p>
        {report.largest_sessions.length === 0 ? <p>Nessuna conversazione principale osservata.</p> : <ul className="usage-sessions">
          {report.largest_sessions.map(session => <li key={session.key}>
            <div><button className="session-title" type="button" onClick={() => setSelected(session.key)}>{session.title ?? session.native_id}</button>
            <small>{session.namespace ?? session.native_id}</small></div>
            <span>{sizeLabel(session.logical_bytes)}</span>
          </li>)}
        </ul>}
      </section>
      <p>{report.removal_reason}</p>
    </>}
    {selected && <SessionDetail key={selected} rowKey={selected} provider="claude" onClose={() => setSelected(undefined)} onChange={() => setRevision(value => value + 1)} />}
  </div>
}
