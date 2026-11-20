import { useEffect, useState } from 'react'
import { Database, HardDrive } from 'lucide-react'
import { LoadingState, Notice } from '../../components/ui/feedback'
import { errorText } from '../../lib/api/client'
import { discoverCodex, type Discovery } from './api'

const storageLabels: Record<string, string> = {
  home: 'Root Codex', sessions: 'Rollout delle sessioni', archived_sessions: 'Rollout archiviati',
  state_database: 'Database di stato', history_database: 'Database cronologia', desktop_catalog: 'Catalogo Desktop',
  export_candidate: 'Candidato export', artifact_session_exports: 'Export Artifact Session', external_exports: 'Export esterni',
  attachments: 'Allegati', generated_images: 'Immagini generate', cache: 'Cache', logs: 'Log',
  worktrees: 'Worktree', user_output: 'Output utente', desktop_profile: 'Profilo Desktop',
}
export function CodexDiscovery() {
  const [data, setData] = useState<Discovery>()
  const [error, setError] = useState('')
  useEffect(() => {
    const controller = new AbortController()
    discoverCodex(controller.signal).then(setData).catch(e => {
      if (!controller.signal.aborted) setError(errorText(e))
    })
    return () => controller.abort()
  }, [])
  return <section className="storage-diagnostics" lang="it">
    <div className="inventory-heading"><h2>Codex rilevato</h2>{data && <span>{data.platform} · {data.architecture}</span>}</div>
    {error ? <Notice tone="error">{error}</Notice> : !data ? <LoadingState title="Rilevamento Codex" /> : <>
      <div className="storage-launch"><span>Root selezionata per le sessioni</span><code>{data.selected_root}</code></div>
      <ul className="storage-locations">
        {data.installations.map(item => <li key={item.kind + item.path}>
          <Database size={16} aria-hidden /><div><strong>{item.kind === 'desktop' ? 'Codex Desktop' : 'CLI Codex'}</strong>
            <span>{[item.version, item.architecture].filter(Boolean).join(' · ') || 'Versione non verificata'}</span><code>{item.path}</code></div>
        </li>)}
        {data.storage.map((item, index) => <li key={item.root + item.kind + item.path + index}>
          <HardDrive size={16} aria-hidden /><div>
            <strong>{storageLabels[item.kind] ?? item.kind}</strong>
            <span>{item.access === 'accessible' ? item.owned ? 'Proprietà riconosciuta' : item.recognized ? 'Formato riconosciuto' : 'Osservato' : item.access === 'linked' ? 'Collegamento protetto' : 'Inaccessibile'} · {item.source}</span>
            <code>{item.path}</code><small>{item.reason}</small>
          </div>
        </li>)}
      </ul>
    </>}
  </section>
}
