import { useEffect, useState } from 'react'
import { FileText, ShieldCheck, Trash2 } from 'lucide-react'
import { projectPath, request, sessionPath, type ClaudeReview, type ClaudeStorage, type DeletionResult, type Detail, type Project, type Provider, type Transcript } from './api'
import { CloseButton, Dialog, Empty, Notice, SessionStatus } from './components'
import { claudeCategories, claudeOwnership, claudeWriter, dateLabel, kindLabel, numberLabel, pathLabel, providerLabel, sizeLabel } from './format'

const errorText = (failure: unknown) => failure instanceof Error ? failure.message : 'Operazione non disponibile.'

function Fact({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return <div><dt>{label}</dt><dd className={mono ? 'mono' : undefined}>{value}</dd></div>
}

function Conversation({ path }: { path: string }) {
  const [page, setPage] = useState<Transcript>()
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  async function load(cursor?: string) {
    setBusy(true)
    setError('')
    try {
      setPage(await request<Transcript>(`${path}/conversation${cursor ? `?cursor=${encodeURIComponent(cursor)}` : ''}`))
    } catch (failure) {
      setError(errorText(failure))
    } finally {
      setBusy(false)
    }
  }

  return <section className="detail-section">
    <h3>Conversazione</h3>
    {error && <Notice tone="error">{error}</Notice>}
    {page?.messages.map((message, index) => <article className="message" key={index}>
      <strong>{message.role === 'user' ? 'Utente' : 'Assistente'}</strong>
      <p>{message.text}</p>
    </article>)}
    {page && page.messages.length === 0 && <p>Nessun messaggio visualizzabile in questa pagina.</p>}
    {page && !page.complete && !page.next && <p>Anteprima parziale. Alcuni record non sono disponibili.</p>}
    <div className="detail-actions">
      {!page && <button type="button" className="button" disabled={busy} onClick={() => void load()}><FileText size={16} aria-hidden />{busy ? 'Caricamento…' : 'Leggi anteprima'}</button>}
      {page?.next && <button type="button" className="button" disabled={busy} onClick={() => void load(page.next ?? undefined)}>{busy ? 'Caricamento…' : 'Pagina successiva del transcript'}</button>}
      {page && <button type="button" className="button quiet" disabled={busy} onClick={() => setPage(undefined)}>Chiudi anteprima</button>}
    </div>
  </section>
}

const titleSources = {
  custom: 'Personalizzato',
  generated: 'Generato',
  first_request: 'Prima richiesta visibile',
  fallback: 'Identificativo breve',
}

const historyLabels = {
  not_observed: 'Non osservata',
  unverified: 'Lettura non verificabile',
  uuid_only: 'Corrispondenza per UUID, ownership incerta',
  partial_uuid_only: 'Lettura parziale, corrispondenza per UUID',
}

function ClaudeDependencies({ path }: { path: string }) {
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

  return <section className="detail-section removal-section">
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

function useDeletion(path: string, onChange: () => void) {
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [result, setResult] = useState<DeletionResult>()

  async function remove(review?: string) {
    setBusy(true)
    setError('')
    try {
      setResult(await request<DeletionResult>(`${path}/delete`, {
        method: 'POST', body: JSON.stringify({ confirm: true, review }),
      }))
    } catch (failure) {
      setError(errorText(failure))
    } finally {
      setBusy(false)
      onChange()
    }
  }

  return { busy, error, result, remove }
}

function RemovalResult({ result, error }: { result?: DeletionResult; error: string }) {
  return <div className="removal-result">
    {error && <Notice tone="error">{error}</Notice>}
    {result && <Notice tone={result.status === 'deleted' ? 'success' : 'warning'}>
      <p>{result.message}</p>
      {result.errors.length > 0 && <ul>{result.errors.map((message, index) => <li key={index}>{message}</li>)}</ul>}
    </Notice>}
  </div>
}

function DeleteConfirmation({ busy, onCancel, onDelete }: { busy: boolean; onCancel: () => void; onDelete: () => void }) {
  return <>
    <Notice tone="warning">L’eliminazione è definitiva e irreversibile.</Notice>
    <div className="detail-actions">
      <button className="button" type="button" onClick={onCancel} disabled={busy}>Annulla</button>
      <button className="button danger" type="button" onClick={onDelete} disabled={busy}>
        <Trash2 size={16} aria-hidden />{busy ? 'Eliminazione…' : 'Elimina'}
      </button>
    </div>
  </>
}

export function ProjectRemoval({ project, onClose, onChange }: {
  project: Project; onClose: () => void; onChange: () => void
}) {
  const removal = useDeletion(projectPath(project.provider, project.key), onChange)
  return <Dialog onClose={onClose} label="project-removal-title">
    <header className="drawer-header"><span>Eliminazione delle sessioni del progetto</span><CloseButton onClose={onClose} /></header>
    <div className="detail-body">
      <h2 id="project-removal-title">{project.name}</h2>
      <p>Codex eliminerà {numberLabel(project.main_conversations)} conversazioni principali del progetto.</p>
      <RemovalResult {...removal} />
      {!removal.result && <DeleteConfirmation busy={removal.busy} onCancel={onClose} onDelete={() => void removal.remove()} />}
    </div>
  </Dialog>
}

function SessionRemoval({ path, detail, onChange }: { path: string; detail: Detail; onChange: () => void }) {
  const [review, setReview] = useState<ClaudeReview>()
  const [confirming, setConfirming] = useState(false)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const removal = useDeletion(path, onChange)
  const isClaude = detail.session.provider === 'claude'
  const unavailable = isClaude && detail.claude_cleanup_supported ? null : detail.session.removal_reason

  async function prepare() {
    setError('')
    if (!isClaude) {
      setConfirming(true)
      return
    }
    setLoading(true)
    try {
      setReview(await request<ClaudeReview>(`${path}/removal-preview`, {
        method: 'POST',
      }))
      setConfirming(true)
    } catch (failure) {
      setError(errorText(failure))
    } finally {
      setLoading(false)
    }
  }

  return <section className="detail-section removal-section">
    <h3>Eliminazione</h3>
    {unavailable && <p>{unavailable}</p>}
    <RemovalResult result={removal.result} error={error || removal.error} />
    {!confirming && !removal.result && <div className="detail-actions">
      <button className="button danger-outline" type="button" disabled={!!unavailable || loading} onClick={() => void prepare()}>
        <Trash2 size={16} aria-hidden />{loading ? 'Caricamento…' : isClaude ? 'Anteprima eliminazione' : 'Elimina sessione'}
      </button>
    </div>}
    {confirming && !removal.result && <>
      {review && <>
        <p>{numberLabel(review.plan.files.length)} file · {numberLabel(review.plan.directories.length)} directory · {sizeLabel(review.plan.logical_bytes)}</p>
        <ul className="claude-artifacts claude-removal-targets" tabIndex={0} aria-label="File e directory da eliminare">
          {review.plan.directories.map(directory => <li key={JSON.stringify([directory.root, directory.relative_path])}>
            <code>{pathLabel(directory.relative_path)}/</code><span>{pathLabel(directory.root)}</span>
          </li>)}
          {review.plan.files.map(file => <li key={JSON.stringify([file.root, file.relative_path])}>
            <code>{pathLabel(file.relative_path)}</code><span>{pathLabel(file.root)} · {sizeLabel(file.bytes)}</span>
          </li>)}
        </ul>
        {review.plan.history_matches !== null && <p>Righe di cronologia associate · {numberLabel(review.plan.history_matches)}</p>}
        {review.plan.notes.length > 0 && <Notice tone="warning"><ul>{review.plan.notes.map(note => <li key={note}>{note}</li>)}</ul></Notice>}
        <p>Chiudi Claude prima di confermare per ridurre il rischio di modifiche concorrenti.</p>
      </>}
      <DeleteConfirmation busy={removal.busy} onCancel={() => setConfirming(false)} onDelete={() => void removal.remove(review?.review)} />
    </>}
  </section>
}

export function SessionDetail({ provider, rowKey, onClose, onChange }: {
  provider: Provider; rowKey: string; onClose: () => void; onChange: () => void
}) {
  const [detail, setDetail] = useState<Detail>()
  const [error, setError] = useState('')
  const path = sessionPath(provider, rowKey)
  useEffect(() => {
    const controller = new AbortController()
    request<Detail>(path, { signal: controller.signal }).then(setDetail).catch(failure => {
      if (!controller.signal.aborted) setError(errorText(failure))
    })
    return () => controller.abort()
  }, [path])

  const session = detail?.session
  return <Dialog onClose={onClose} label="session-title" className={session?.claude ? "drawer claude-session" : "drawer"}>
    <header className="drawer-header"><span>Dettagli sessione</span><CloseButton onClose={onClose} /></header>
    {detail && session ? <div className="detail-body">
        <div className="detail-labels"><SessionStatus session={session} />{session.protected && <span className="chip"><ShieldCheck size={14} aria-hidden />Protetta</span>}</div>
        <h2 id="session-title">{session.title ?? 'Senza titolo'}</h2>
        <code className="detail-id">{session.native_id}</code>
        <dl className="facts detail-facts">
          <Fact label="Provider" value={providerLabel[provider]} />
          <Fact label="Tipo" value={kindLabel[session.kind]} />
          <Fact label="Creazione (UTC)" value={dateLabel(session.created_at)} />
          <Fact label="Ultima attività (UTC)" value={dateLabel(session.last_active_at)} />
          <Fact label="Dimensione transcript" value={sizeLabel(session.transcript_bytes)} />
          <Fact label={session.claude ? "Versioni nel transcript" : "Versione rilevata"} value={(session.claude ? session.claude.versions.join(', ') : detail.version) || 'Non disponibile'} />
          <Fact label="Compatibilità" value={detail.claude_cleanup_supported || !session.removal_reason ? 'Eliminazione disponibile' : 'Sola lettura'} />
          <Fact label="Workspace" value={session.workspace ?? 'Non disponibile'} mono />
          {session.claude && <>
            <Fact label="Namespace" value={session.namespace ?? 'Non disponibile'} mono />
            <Fact label="Root archivio" value={pathLabel(session.archive_root)} mono />
            <Fact label="Superficie" value={session.claude.surface === 'desktop' ? 'Claude Code in Desktop' : session.claude.surface === 'cli' ? 'CLI' : session.claude.surface === 'mixed' ? 'CLI e Desktop' : 'Non osservata'} />
            <Fact label="Origine del titolo" value={titleSources[session.claude.title_source]} />
            <Fact label="Scansione metadati" value={session.claude.metadata_complete ? 'Completa' : 'Parziale'} />
            <Fact label="Ultima modifica del file (UTC)" value={dateLabel(session.claude.modified_at)} />
            {session.claude.same_uuid_elsewhere > 0 && <Fact label="Stesso UUID in altre posizioni" value={numberLabel(session.claude.same_uuid_elsewhere)} />}
          </>}
        </dl>
        {detail.conversation_preview ? <Conversation path={path} /> : <section className="detail-section"><h3>Conversazione</h3><p>Anteprima del transcript non disponibile.</p></section>}
        <SessionRemoval path={path} detail={detail} onChange={onChange} />
        {session.claude && <ClaudeDependencies path={path} />}
      </div> : <div className="detail-body"><h2 id="session-title">Dettagli sessione</h2>{error ? <Notice tone="error">{error}</Notice> : <Empty title="Caricamento dettagli" busy />}</div>}
  </Dialog>
}
