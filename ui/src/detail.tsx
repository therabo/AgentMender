import { useEffect, useState } from 'react'
import { AlertTriangle, ArrowLeft, FileText, ShieldCheck, Trash2 } from 'lucide-react'
import { request, sessionPath, type Detail, type Preview, type Provider, type Transcript } from './api'
import { CloseButton, Dialog, Empty, Notice, SessionStatus } from './components'
import { dateLabel, kindLabel, providerLabel, sizeLabel } from './format'

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

function RemovalReview({ detail, preview, onBack, onConfirm }: { detail: Detail; preview: Preview; onBack: () => void; onConfirm: (id: string, phrase: string) => void }) {
  const [entered, setEntered] = useState('')
  const [expires] = useState(() => Date.now() + preview.valid_seconds * 1000)
  const [expired, setExpired] = useState(false)
  const session = detail.session
  useEffect(() => {
    const timer = window.setTimeout(() => setExpired(true), Math.max(0, expires - Date.now()))
    return () => window.clearTimeout(timer)
  }, [expires])
  return <div className="detail-body">
    <button className="button quiet back" type="button" onClick={onBack}><ArrowLeft size={16} aria-hidden /> Dettagli sessione</button>
    <h2 id="session-title">Conferma rimozione</h2>
    <dl className="facts review-facts">
      <Fact label="Provider" value={providerLabel[session.provider]} />
      <Fact label="Titolo" value={session.title ?? 'Senza titolo'} />
      <Fact label="Identificativo" value={session.native_id} mono />
    </dl>
    <Notice tone="warning">La rimozione è permanente. Il comando Codex può includere subagent collegati. AgentMender ha verificato quelli rilevabili e ripeterà il controllo prima di avviare il comando.</Notice>
    <form className="confirm-form" onSubmit={event => {
      event.preventDefault()
      if (!expired && entered === preview.confirmation) onConfirm(preview.id, entered)
    }}>
      <label htmlFor="confirmation">Digita <strong>{preview.confirmation}</strong> per confermare</label>
      <input id="confirmation" autoComplete="off" spellCheck={false} value={entered} onChange={event => setEntered(event.target.value)} />
      {expired && <Notice tone="warning">Anteprima scaduta. Torna ai dettagli e richiedine una nuova.</Notice>}
      <div className="detail-actions">
        <button className="button" type="button" onClick={onBack}>Annulla</button>
        <button className="button danger" disabled={expired || entered !== preview.confirmation} type="submit"><Trash2 size={16} aria-hidden />Rimuovi la sessione</button>
      </div>
    </form>
  </div>
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
  return <Dialog onClose={close} label="session-title">
    <header className="drawer-header"><span>Dettagli sessione</span><CloseButton onClose={close} /></header>
    {detail && preview ? <RemovalReview detail={detail} preview={preview} onBack={close} onConfirm={onConfirm} />
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
          <Fact label="Versione rilevata" value={detail.version ?? 'Non disponibile'} />
          <Fact label="Compatibilità" value={session.removal_reason ? 'Sola lettura' : 'Rimozione soggetta a verifica'} />
          <Fact label="Workspace" value={session.workspace ?? 'Non disponibile'} mono />
        </dl>
        <section className="detail-section">
          <h3>Sessioni collegate</h3>
          {session.parent && <p>Sessione padre <code>{session.parent}</code></p>}
          {session.forked_from && <p>Fork di <code>{session.forked_from}</code></p>}
          {detail.children.length > 0 && <ul>{detail.children.map(id => <li key={id}><code>{id}</code></li>)}</ul>}
          {!session.parent && !session.forked_from && detail.children.length === 0 && <p>Nessuna relazione rilevata.</p>}
          <p>La copertura delle relazioni è parziale.</p>
        </section>
        {detail.conversation_preview ? <Conversation path={path} /> : <section className="detail-section"><h3>Conversazione</h3><p>Anteprima del transcript non disponibile.</p></section>}
        <section className="detail-section removal-section">
          <h3>Rimozione</h3>
          {session.removal_reason && <p><AlertTriangle size={15} aria-hidden />{session.removal_reason}</p>}
          {error && <Notice tone="error">{error}</Notice>}
          <button className="button danger-outline" disabled={!!session.removal_reason || busy} onClick={() => void prepare()}><Trash2 size={16} aria-hidden />{busy ? 'Verifica in corso…' : 'Anteprima rimozione'}</button>
        </section>
      </div> : <div className="detail-body"><h2 id="session-title">Dettagli sessione</h2>{error ? <Notice tone="error">{error}</Notice> : <Empty title="Caricamento dettagli" busy />}</div>}
  </Dialog>
}
