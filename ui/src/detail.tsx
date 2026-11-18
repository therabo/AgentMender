import { useEffect, useId, useState } from 'react'
import { AlertTriangle, ArrowLeft, FileText, ShieldCheck, Trash2, Undo2 } from 'lucide-react'
import { projectPath, request, sessionPath, type ClaudeBackup, type ClaudeMode, type ClaudeOutcome, type ClaudeReceipt, type ClaudeReview, type ClaudeStorage, type Detail, type Preview, type Project, type ProjectPreview, type Provider, type Transcript } from './api'
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
        <h4>Risorse candidate per modalità</h4>
        <div className="claude-plans">{storage.plans.map(plan => <section key={plan.mode}>
          <h5>{plan.mode === 'standard' ? 'Standard' : 'Approfondita'}</h5>
          <dl className="facts detail-facts">
            <Fact label="File candidati" value={numberLabel(plan.candidate_files)} />
            <Fact label="Dimensione associata" value={sizeLabel(plan.associated_logical_bytes)} />
            <Fact label="Allocazione osservata" value={sizeLabel(plan.allocated_observed)} />
            <Fact label="Esclusività verificata" value={sizeLabel(plan.exclusive_logical_bytes)} />
            <Fact label="Condivisi e protetti" value={sizeLabel(plan.shared_logical_bytes)} />
          </dl>
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

function ConfirmForm({ preview, onCancel, onConfirm, action = 'Rimuovi definitivamente', icon: Icon = Trash2 }: {
  preview: ClaudeReceipt
  action?: string
  icon?: typeof Trash2
  onCancel: () => void
  onConfirm: (id: string, phrase: string) => void
}) {
  const inputId = useId()
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
      <label htmlFor={inputId}>Digita <strong>{preview.confirmation}</strong> per confermare</label>
      <input id={inputId} autoComplete="off" spellCheck={false} value={entered} onChange={event => setEntered(event.target.value)} />
      {expired && <Notice tone="warning">Anteprima scaduta. Richiedi una nuova anteprima.</Notice>}
      <div className="detail-actions">
        <button className="button" type="button" onClick={onCancel}>Annulla</button>
        <button className="button danger" disabled={expired || entered !== preview.confirmation} type="submit"><Icon size={16} aria-hidden />{action}</button>
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

export function SessionDetail({ provider, rowKey, onClose, onConfirm, onClaudeChange }: {
  onClaudeChange?: () => void
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
          <Fact label="Compatibilità" value={detail.claude_cleanup_supported || !session.removal_reason ? 'Rimozione soggetta a verifica' : 'Sola lettura'} />
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
        {session.claude
          ? <>
            {detail.claude_cleanup_supported && <ClaudeCleanupReview path={path} onChange={onClaudeChange} />}
            <ClaudeDependencies path={path} reason={session.removal_reason ?? ''} />
          </>
          : <section className="detail-section removal-section">
          <h3>Rimozione</h3>
          {session.removal_reason && <p><AlertTriangle size={15} aria-hidden />{session.removal_reason}</p>}
          {error && <Notice tone="error">{error}</Notice>}
          <button className="button danger-outline" disabled={!!session.removal_reason || busy} onClick={() => void prepare()}><Trash2 size={16} aria-hidden />{busy ? 'Verifica in corso…' : 'Anteprima rimozione'}</button>
        </section>}
      </div> : <div className="detail-body"><h2 id="session-title">Dettagli sessione</h2>{error ? <Notice tone="error">{error}</Notice> : <Empty title="Caricamento dettagli" busy />}</div>}
  </Dialog>
}

const cleanupResult: Record<string, string> = {
  verified: 'Rimozione verificata',
  partial: 'Rimozione dei file verificata, con risorse residue',
  restored: 'Ripristino verificato',
  discarded: 'Backup eliminato',
  changed: 'Archivio cambiato',
  refused: 'Operazione rifiutata',
  uncertain: 'Esito incerto',
  recovery_required: 'Verifica il backup per il ripristino',
}

const cleanupReason: Record<string, string> = {
  changed: 'Un file o la sua destinazione è cambiato.',
  active_writer: 'Un writer Claude è attivo.',
  writer_unverified: 'Lo stato dei writer non è verificabile.',
  ownership_unverified: 'L’attribuzione dei file è cambiata.',
  unsupported_format: 'Il formato non rientra nella compatibilità verificata.',
  insufficient_backup_space: 'Spazio o volume di backup insufficiente.',
  partial_scope: 'Il perimetro non è verificabile.',
}

function useClaudeOperation(onChange?: () => void) {
  const [operation, setOperation] = useState<string>()
  const [posting, setPosting] = useState(false)
  const [error, setError] = useState('')
  const [outcome, setOutcome] = useState<ClaudeOutcome>()
  useEffect(() => {
    if (!operation) return
    const abort = new AbortController()
    let timer = 0
    const poll = async () => {
      try {
        const result = await request<ClaudeOutcome>(`/api/claude/operations/${operation}`, { signal: abort.signal })
        if (abort.signal.aborted) return
        if (result.status === 'running') {
          timer = window.setTimeout(poll, 350)
        } else {
          setOutcome(result)
          setOperation(undefined)
          onChange?.()
        }
      } catch (failure) {
        if (!abort.signal.aborted) {
          setError(errorText(failure))
          setOperation(undefined)
        }
      }
    }
    void poll()
    return () => { abort.abort(); window.clearTimeout(timer) }
  }, [operation, onChange])

  async function confirm(id: string, phrase: string) {
    setOutcome(undefined)
    setPosting(true)
    setError('')
    try {
      await request(`/api/claude/operations/${id}/confirm`, {
        method: 'POST', body: JSON.stringify({ confirmation: phrase }),
      })
      setOperation(id)
    } catch (failure) {
      setError(errorText(failure))
      setOperation(undefined)
    } finally {
      setPosting(false)
    }
  }

  return { operation, posting, confirm, outcome, error, setError }
}

function ClaudeOperationStatus({ operation, posting, outcome, error }: {
  operation: string | undefined; posting: boolean; outcome: ClaudeOutcome | undefined; error: string
}) {
  return <>
    {(operation || posting) && <p role="status">Operazione Claude in corso…</p>}
    {error && <Notice tone="error">{error}</Notice>}
    {outcome && <Notice tone={['verified', 'restored', 'discarded'].includes(outcome.status) ? 'info' : 'warning'}>
      <strong>{cleanupResult[outcome.status] ?? outcome.status}</strong>
      {outcome.reason && <p>{cleanupReason[outcome.reason]}</p>}
      {!!outcome.logical_bytes && <p>Dimensione logica rimossa · {sizeLabel(outcome.logical_bytes)}</p>}
      {outcome.backup_id && <p>Il backup è disponibile in Storage Claude Code.</p>}
    </Notice>}
  </>
}

function ClaudeCleanupReview({ path, onChange }: { path: string; onChange?: () => void }) {
  const [mode, setMode] = useState<ClaudeMode>('standard')
  const [preview, setPreview] = useState<ClaudeReview>()
  const [loading, setLoading] = useState(false)
  const operation = useClaudeOperation(onChange)

  async function prepare() {
    setLoading(true)
    operation.setError('')
    try {
      setPreview(await request<ClaudeReview>(`${path}/removal-preview`, {
        method: 'POST', body: JSON.stringify({ mode }),
      }))
    } catch (failure) {
      operation.setError(errorText(failure))
    } finally {
      setLoading(false)
    }
  }

  return <section className="detail-section removal-section">
    <h3>Rimozione Claude Code</h3>
    <ClaudeOperationStatus {...operation} />
    {!preview && !operation.operation && !operation.posting && !['verified', 'partial', 'recovery_required', 'uncertain'].includes(operation.outcome?.status ?? '') && <div className="detail-actions claude-cleanup-actions">
      <label>Modalità
        <select aria-label="Modalità di rimozione Claude" value={mode} disabled={loading} onChange={event => setMode(event.target.value as ClaudeMode)}>
          <option value="standard">Standard</option>
          <option value="deep">Approfondita</option>
        </select>
      </label>
      <button type="button" className="button danger-outline" disabled={loading} onClick={() => void prepare()}>
        <Trash2 size={16} aria-hidden />{loading ? 'Verifica in corso…' : 'Anteprima rimozione'}
      </button>
    </div>}
    {preview && <>
      <h4>{preview.review.mode === 'standard' ? 'Piano Standard' : 'Piano Approfondito'}</h4>
      <dl className="facts detail-facts">
        <Fact label="Verifica processi" value="Claude chiuso al momento del controllo" />
        <Fact label="Subagent inclusi" value={numberLabel(preview.review.operations.filter(target => target.role === 'subagent').length)} />
        <Fact label="Dimensione logica" value={sizeLabel(preview.review.logical_bytes)} />
        <Fact label="Spazio richiesto per il backup" value={sizeLabel(preview.review.backup_required_bytes)} />
        <Fact label="History" value={preview.review.history_state === 'not_observed' ? 'Assente' : `Conservata · ${preview.review.history_matches ?? 'conteggio incerto'}`} />
      </dl>
      <ul className="claude-artifacts">{preview.review.operations.map((file, index) => <li key={index}>
        <strong>{claudeCategories[file.role] ?? file.role}</strong>
        <code>{pathLabel(file.relative_path)}</code><span>{pathLabel(file.root)} · {sizeLabel(file.bytes)} · {sizeLabel(file.allocated_bytes)} allocati</span>
      </li>)}</ul>
      {preview.review.empty_directories.length > 0 && <>
        <h4>Directory da rimuovere se vuote</h4>
        <ul className="claude-artifacts">{preview.review.empty_directories.map((folder, index) => <li key={index}>
          <code>{pathLabel(folder.relative_path)}</code><span>{pathLabel(folder.root)}</span>
        </li>)}</ul>
      </>}
      {preview.review.excluded.length > 0 && <><h4>Risorse escluse</h4><ul>{preview.review.excluded.map(item => <li key={item}>{item}</li>)}</ul></>}
      <Notice tone="warning">{preview.review.residual_risk}</Notice>
      <ConfirmForm key={preview.id} preview={preview} action="Rimuovi con backup" onCancel={() => setPreview(undefined)}
        onConfirm={(id, phrase) => { setPreview(undefined); void operation.confirm(id, phrase) }} />
    </>}
  </section>
}

export function ClaudeBackupPanel({ revision, onChange }: { revision: number; onChange: () => void }) {
  const [list, setList] = useState<ClaudeBackup[]>()
  const [preview, setPreview] = useState<{ receipt: ClaudeReceipt; discard: boolean; backup: ClaudeBackup }>()
  const [loading, setLoading] = useState(false)
  const [changed, setChanged] = useState(0)
  const operation = useClaudeOperation(onChange)
  const outcome = operation.outcome
  useEffect(() => {
    const abort = new AbortController()
    request<ClaudeBackup[]>('/api/claude/backups', { signal: abort.signal })
      .then(result => { if (!abort.signal.aborted) setList(result) })
      .catch(failure => { if (!abort.signal.aborted) operation.setError(errorText(failure)) })
    return () => abort.abort()
  }, [outcome, changed, revision, operation.setError])

  async function prepare(backup: ClaudeBackup, discard: boolean) {
    setLoading(true)
    operation.setError('')
    try {
      const receipt = await request<ClaudeReceipt>(`/api/claude/backups/${backup.id}/preview`, {
        method: 'POST', body: JSON.stringify({ action: discard ? 'discard' : 'restore' }),
      })
      setPreview({ receipt, discard, backup })
    } catch (failure) {
      operation.setError(errorText(failure))
    } finally {
      setLoading(false)
    }
  }

  return <section className="inventory claude-usage-panel" aria-label="Backup di rimozione Claude">
    <div className="inventory-heading">
      <h2>Backup di rimozione</h2>
      <button type="button" className="button quiet" onClick={() => setChanged(value => value + 1)}>Aggiorna backup</button>
    </div>
    <div className="claude-backup-body">
      <p><code>~/.agentmender/claude-backups</code></p>
      <ClaudeOperationStatus {...operation} />
      {list?.length === 0 && <p>Nessun backup di rimozione.</p>}
      {list && list.length > 0 && <ul className="claude-artifacts">{list.map(item => <li key={item.id}>
        <strong><code>{item.native_id}</code></strong>
        <span>{sizeLabel(item.logical_bytes)} · {cleanupResult[item.phase] ?? (item.can_restore ? 'Ripristino disponibile' : 'Backup conservato')}</span>
        <div className="detail-actions">
          {item.can_restore && <button className="button" type="button" disabled={loading || !!preview || !!operation.operation || operation.posting} onClick={() => void prepare(item, false)}>Ripristina</button>}
          {item.can_discard && <button className="button danger-outline" type="button" disabled={loading || !!preview || !!operation.operation || operation.posting} onClick={() => void prepare(item, true)}>Elimina backup</button>}
        </div>
      </li>)}</ul>}
      {preview && <>
        <Notice tone="warning">{preview.discard ? 'Eliminare il backup rende inutilizzabile questa copia per il ripristino.' : 'Tieni Claude chiuso. Il ripristino si ferma se trova file ricreati.'} <code>{preview.backup.native_id}</code></Notice>
        <ConfirmForm key={preview.receipt.id} preview={preview.receipt} icon={preview.discard ? Trash2 : Undo2} action={preview.discard ? 'Elimina backup' : 'Ripristina sessione'}
          onCancel={() => setPreview(undefined)} onConfirm={(id, phrase) => { setPreview(undefined); void operation.confirm(id, phrase) }} />
      </>}
    </div>
  </section>
}
