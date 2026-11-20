import { useEffect, useState } from 'react'
import { Fact } from '../../components/ui/Fact'
import { Notice } from '../../components/ui/feedback'
import { errorText, request } from '../../lib/api/client'
import type { ClaudeStorage } from '../../lib/api/contracts'
import { claudeCategories, claudeOwnership, claudeWriter, numberLabel, pathLabel, sizeLabel } from '../../lib/utils/format'

const historyLabels = {
  not_observed: 'Non osservata',
  unverified: 'Lettura non verificabile',
  uuid_only: 'Corrispondenza per UUID, ownership incerta',
  partial_uuid_only: 'Lettura parziale, corrispondenza per UUID',
}

export function ClaudeDependencies({ path }: { path: string }) {
  const [storage, setStorage] = useState<ClaudeStorage>()
  const [error, setError] = useState('')
  const [open, setOpen] = useState(false)
  useEffect(() => {
    if (!open || storage) return
    setError('')
    const controller = new AbortController()
    request<ClaudeStorage>(`${path}/storage`, { signal: controller.signal })
      .then(result => { if (!controller.signal.aborted) setStorage(result) })
      .catch(failure => { if (!controller.signal.aborted) setError(errorText(failure)) })
    return () => controller.abort()
  }, [path, open, storage])

  return <section className="removal-section" lang="it">
    <h3>Storage e dipendenze</h3>
    <details onToggle={event => setOpen(event.currentTarget.open)}>
      <summary>Risorse osservate</summary>
      {error ? <Notice tone="error">{error}</Notice> : !storage ? <p role="status">Lettura dell’archivio…</p> : <>
        <dl className="facts detail-facts">
          <Fact label="Transcript selezionato" value={`${numberLabel(storage.transcript_files)} file · ${sizeLabel(storage.logical_bytes)}`} />
          <Fact label="Subagent riconosciuti" value={numberLabel(storage.subagents)} />
          <Fact label="Fork riconosciuti" value={numberLabel(storage.forks)} />
          <Fact label="Sidecar candidati" value={numberLabel(storage.sidecar_candidates)} />
          <Fact label="Checkpoint candidato" value={storage.checkpoint_present ? 'Osservato' : 'Non osservato'} />
          <Fact label="Righe history" value={storage.history_matches === null ? historyLabels[storage.history_observation] : `${numberLabel(storage.history_matches)} · ${historyLabels[storage.history_observation]}`} />
          <Fact label="Registro dei processi" value={storage.process_registry === 'present_unverified' ? 'Presente · writer non verificati' : storage.process_registry === 'blocked' ? 'Inaccessibile o protetto' : 'Non osservato · writer non verificati'} />
        </dl>
        <p>{claudeWriter[storage.writer_observation] ?? claudeWriter.unavailable}.</p>
        {!storage.diagnostic_complete && <Notice tone="warning">Alcune risorse o relazioni non sono state verificate.</Notice>}
        <h4>Categorie osservate</h4>
        <ul className="claude-artifacts">{storage.categories.map(category =>
          <li key={category.key}>
            <strong>{claudeCategories[category.key] ?? category.key}</strong>
            <span>{numberLabel(category.files)} file · {sizeLabel(category.logical_bytes)} · {category.state === 'not_observed' ? 'Non osservati' : category.state === 'protected' ? 'Protetti' : category.state === 'exclusive_verified' ? 'Esclusività verificata' : 'Da verificare'}</span>
          </li>
        )}</ul>
        {storage.fork_references.length > 0 && <p>Fork da preservare · {storage.fork_references.join(', ')}</p>}
        <details><summary>Percorsi e identità</summary>
          <ul className="claude-artifacts">{storage.observed.map((resource, index) => {
            const relativePath = pathLabel(resource.relative_path)
            return <li key={index}><code>{relativePath}</code><span>{pathLabel(resource.root)}</span><span>{claudeOwnership[resource.ownership]} · {sizeLabel(resource.bytes)}{resource.identity && ` · ${resource.identity}`}</span></li>
          })}</ul>
        </details>
      </>}
    </details>
  </section>
}
