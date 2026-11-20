import { useEffect, useState } from 'react'
import { EmptyState, LoadingState, Notice } from '../../components/ui/feedback'
import { ClosePreview } from '../../components/ui/glass'
import { Fact } from '../../components/ui/Fact'
import { errorText } from '../../lib/api/client'
import type { Detail, Provider } from '../../lib/api/contracts'
import { activityLabel, dateLabel, kindLabel, numberLabel, pathLabel, sizeLabel } from '../../lib/utils/format'
import { readSession, sessionPath } from './api'
import { ClaudeDependencies } from './ClaudeDependencies'
import { Conversation } from './Conversation'
import { SessionStatus } from './SessionList'
import { SessionRemoval } from './removal'

function SessionDiagnostics({ detail, path }: { detail: Detail; path: string }) {
  const session = detail.session
  return <details className="preview-diagnostics">
    <summary>Session metadata</summary>
    <dl className="facts">
      <Fact label="Session ID" value={session.native_id} mono />
      <Fact label="Created (UTC)" value={dateLabel(session.created_at)} />
      <Fact label="Type" value={kindLabel[session.kind]} />
      <Fact label="Workspace" value={session.workspace ?? 'Unavailable'} mono />
      <Fact label="Version" value={(session.claude ? session.claude.versions.join(', ') : detail.version) || 'Unavailable'} />
      {session.claude && <>
        <Fact label="Namespace" value={session.namespace ?? 'Unavailable'} mono />
        <Fact label="Archive root" value={pathLabel(session.archive_root)} mono />
        <Fact label="Surface" value={session.claude.surface ?? 'Unknown'} />
        <Fact label="Title source" value={session.claude.title_source.replaceAll('_', ' ')} />
        <Fact label="Scanned metadata" value={session.claude.metadata_complete ? 'Complete' : 'Partial'} />
        <Fact label="File modified (UTC)" value={dateLabel(session.claude.modified_at)} />
        {session.claude.same_uuid_elsewhere > 0 && <Fact label="Same UUID elsewhere" value={numberLabel(session.claude.same_uuid_elsewhere)} />}
      </>}
    </dl>
    {session.claude && <ClaudeDependencies path={path} />}
  </details>
}

export function SessionDetail({ provider, rowKey, onClose, onChange }: {
  provider: Provider; rowKey: string; onClose: () => void; onChange: () => void
}) {
  const [detail, setDetail] = useState<Detail>()
  const [error, setError] = useState('')
  const path = sessionPath(provider, rowKey)

  useEffect(() => {
    const controller = new AbortController()
    readSession(path, controller.signal).then(result => {
      if (!controller.signal.aborted) setDetail(result)
    }).catch(failure => {
      if (!controller.signal.aborted) setError(errorText(failure))
    })
    return () => controller.abort()
  }, [path])

  const session = detail?.session
  return <>
    <header className="preview-heading">
      <h2>{session ? session.title || 'Untitled session' : 'Session details'}</h2>
      {session && <SessionStatus session={session} />}
      <ClosePreview onClick={onClose} />
    </header>
    {detail && session ? <>
      <dl className="preview-facts">
        <div><dt>Last activity</dt><dd title={dateLabel(session.last_active_at)}>{activityLabel(session.last_active_at)}</dd></div>
        <Fact label="Session size" value={sizeLabel(session.transcript_bytes)} />
        <Fact label="Session ID" value={session.native_id} mono />
      </dl>
      <div className="preview-body">
        {detail.conversation_preview ? <Conversation path={path} /> : <EmptyState title="Conversation preview unavailable" />}
        <SessionDiagnostics detail={detail} path={path} />
      </div>
      <SessionRemoval path={path} detail={detail} onChange={onChange} />
    </> : error ? <Notice tone="error">{error}</Notice> : <LoadingState title="Loading session details" />}
  </>
}
