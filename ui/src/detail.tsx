import { useEffect, useState } from 'react'
import { AlertTriangle, ArrowLeft, FileText, ShieldCheck, Trash2 } from 'lucide-react'
import { projectPath, request, sessionPath, type ClaudeStorage, type Detail, type Preview, type Project, type ProjectPreview, type Provider, type Transcript } from './api'
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

function ClaudeDependencies({ path, reason }: { path: string; reason: string }) {
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
    <dl className="facts detail-facts">
      <Fact label="Pulizia standard" value="Disabilitata" />
      <Fact label="Pulizia approfondita" value="Disabilitata" />
    </dl>
    <p>{reason}</p>
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
          <Fact label="Backup" value="Non creato" />
          <Fact label="Disponibile sul volume" value={sizeLabel(storage.available_volume_bytes)} />
        </dl>
        <p>{claudeWriter[storage.writer_observation] ?? claudeWriter.unavailable}.</p>
        {!storage.diagnostic_complete && <Notice tone="warning">Alcune risorse o relazioni non sono state verificate.</Notice>}
        <h4>Anteprima per modalità</h4>
        <div className="claude-plans">{storage.plans.map(plan => <section key={plan.mode}>
          <h5>{plan.mode === 'standard' ? 'Standard' : 'Approfondita'}</h5>
          <dl className="facts detail-facts">
            <Fact label="File candidati" value={numberLabel(plan.candidate_files)} />
            <Fact label="Dimensione associata" value={sizeLabel(plan.associated_logical_bytes)} />
            <Fact label="Allocazione osservata" value={sizeLabel(plan.allocated_observed)} />
            <Fact label="Esclusività verificata" value={sizeLabel(plan.exclusive_logical_bytes)} />
            <Fact label="Condivisi e protetti" value={sizeLabel(plan.shared_logical_bytes)} />
            <Fact label="Recupero autorizzato" value={sizeLabel(plan.authorized_logical_bytes)} />
          </dl>
          <p>{plan.mode === 'deep' ? 'Ciclo di vita di checkpoint, task e allegati da verificare.' : 'Esclusione dei writer, integrità dello stato condiviso e recupero da verificare.'}</p>
        </section>)}</div>
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

function RemovalReview({ preview, onBack, onConfirm }: { preview: Preview; onBack: () => void; onConfirm: (id: string, phrase: string) => void }) {
  return <div className="detail-body">
    <button className="button quiet back" type="button" onClick={onBack}><ArrowLeft size={16} aria-hidden /> Dettagli sessione</button>
    <h2 id="session-title">Conferma rimozione</h2>
    <dl className="facts review-facts">
      <Fact label="Conversazione principale" value={preview.main_title ?? 'Senza titolo'} />
      <Fact label="UUID" value={preview.main_id} mono />
      <Fact label="Subagent verificati" value={preview.spawned_descendants.toLocaleString('it')} />
      <Fact label="Sessioni coinvolte" value={preview.total_sessions.toLocaleString('it')} />
    </dl>
    <Notice tone="warning">La rimozione è irreversibile. Codex rimuoverà la conversazione e l’albero di subagent verificato.</Notice>
    <ConfirmForm preview={preview} onCancel={onBack} onConfirm={onConfirm} />

  </div>
}

function ConfirmForm({ preview, onCancel, onConfirm }: {
  preview: Pick<Preview, 'id' | 'confirmation' | 'valid_seconds'>
  onCancel: () => void
  onConfirm: (id: string, phrase: string) => void
}) {
  const [entered, setEntered] = useState('')
  const [expires] = useState(() => Date.now() + preview.valid_seconds * 1000)
  const [expired, setExpired] = useState(false)
  useEffect(() => {
    const timer = window.setTimeout(() => setExpired(true), Math.max(0, expires - Date.now()))
    return () => window.clearTimeout(timer)
  }, [expires])
  return <form className="confirm-form" onSubmit={event => {
      event.preventDefault()
      if (!expired && entered === preview.confirmation) onConfirm(preview.id, entered)
    }}>
      <label htmlFor="confirmation">Digita <strong>{preview.confirmation}</strong> per confermare</label>
      <input id="confirmation" autoComplete="off" spellCheck={false} value={entered} onChange={event => setEntered(event.target.value)} />
      {expired && <Notice tone="warning">Anteprima scaduta. Richiedi una nuova anteprima.</Notice>}
      <div className="detail-actions">
        <button className="button" type="button" onClick={onCancel}>Annulla</button>
        <button className="button danger" disabled={expired || entered !== preview.confirmation} type="submit"><Trash2 size={16} aria-hidden />Rimuovi definitivamente</button>
      </div>
    </form>
}

export function ProjectRemoval({ project, onClose, onConfirm }: {
  project: Project; onClose: () => void; onConfirm: (id: string, phrase: string) => void
}) {
  const [preview, setPreview] = useState<ProjectPreview>()
  const [error, setError] = useState('')
  const path = projectPath(project.provider, project.key)
  useEffect(() => {
    const controller = new AbortController()
    request<ProjectPreview>(`${path}/removal-preview`, { method: 'POST', signal: controller.signal })
      .then(result => { if (!controller.signal.aborted) setPreview(result) })
      .catch(failure => { if (!controller.signal.aborted) setError(errorText(failure)) })
    return () => controller.abort()
  }, [path])

  return <Dialog onClose={onClose} label="project-removal-title">
    <header className="drawer-header"><span>Rimozione delle sessioni del progetto</span><CloseButton onClose={onClose} /></header>
    <div className="detail-body">
      <h2 id="project-removal-title">{preview?.project_name ?? project.name}</h2>
      {error ? <Notice tone="error">{error}</Notice>
        : !preview ? <Empty title="Verifica dell’archivio in corso" busy />
        : <>
          <dl className="facts review-facts project-review-facts">
            <Fact label="Provider" value={providerLabel[preview.provider]} />
            <Fact label="Root registrate" value={preview.roots.join(" · ") || 'Nessuna'} mono />
            <Fact label="Identificativo del progetto" value={preview.project_key} mono />
            <Fact label="Conversazioni principali" value={preview.main_conversations.toLocaleString('it')} />
            <Fact label="Subagent e discendenti" value={preview.descendants.toLocaleString('it')} />
            <Fact label="Sessioni coinvolte" value={preview.total_sessions.toLocaleString('it')} />
            <Fact label="Verifica" value={preview.complete ? 'Perimetro verificato' : 'Copertura incompleta'} />
            <Fact label="Perimetro" value={preview.scope === 'sessions' ? 'Sessioni associate e discendenti' : 'Non disponibile'} />
          </dl>
          <section className="detail-section">
            <h3>Fuori dal perimetro</h3>
            <ul>{preview.excluded.map(item => <li key={item}>{item}</li>)}</ul>
          </section>
          <Notice tone="warning">La rimozione è irreversibile. Codex procede un sottoalbero alla volta e si ferma se rileva un cambiamento o un esito incerto.</Notice>
          <ConfirmForm preview={preview} onCancel={onClose} onConfirm={onConfirm} />
        </>}
    </div>
  </Dialog>
}

export function SessionDetail({ provider, rowKey, onClose, onConfirm }: {
  provider: Provider; rowKey: string; onClose: () => void; onConfirm: (id: string, phrase: string) => void
}) {
  const [detail, setDetail] = useState<Detail>()
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const [preview, setPreview] = useState<Preview>()
  const path = sessionPath(provider, rowKey)
  useEffect(() => {
    const controller = new AbortController()
    request<Detail>(path, { signal: controller.signal }).then(setDetail).catch(failure => {
      if (!controller.signal.aborted) setError(errorText(failure))
    })
    return () => controller.abort()
  }, [path])

  async function prepare() {
    setBusy(true)
    setError('')
    try {
      setPreview(await request<Preview>(`${path}/removal-preview`, { method: 'POST' }))
    } catch (failure) {
      setError(errorText(failure))
    } finally {
      setBusy(false)
    }
  }
  const session = detail?.session
  const close = () => preview ? setPreview(undefined) : onClose()
  return <Dialog onClose={close} label="session-title" className={session?.claude ? "drawer claude-session" : "drawer"}>
    <header className="drawer-header"><span>Dettagli sessione</span><CloseButton onClose={close} /></header>
    {detail && preview ? <RemovalReview preview={preview} onBack={close} onConfirm={onConfirm} />
      : detail && session ? <div className="detail-body">
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
          <Fact label="Compatibilità" value={session.removal_reason ? 'Sola lettura' : 'Rimozione soggetta a verifica'} />
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
        {session.claude && session.removal_reason
          ? <ClaudeDependencies path={path} reason={session.removal_reason} />
          : <section className="detail-section removal-section">
          <h3>Rimozione</h3>
          {session.removal_reason && <p><AlertTriangle size={15} aria-hidden />{session.removal_reason}</p>}
          {error && <Notice tone="error">{error}</Notice>}
          <button className="button danger-outline" disabled={!!session.removal_reason || busy} onClick={() => void prepare()}><Trash2 size={16} aria-hidden />{busy ? 'Verifica in corso…' : 'Anteprima rimozione'}</button>
        </section>}
      </div> : <div className="detail-body"><h2 id="session-title">Dettagli sessione</h2>{error ? <Notice tone="error">{error}</Notice> : <Empty title="Caricamento dettagli" busy />}</div>}
  </Dialog>
}
