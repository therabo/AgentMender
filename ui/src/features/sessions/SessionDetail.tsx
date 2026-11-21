import { useEffect, useState } from 'react'
import { EmptyState, LoadingState, Notice } from '../../components/ui/feedback'
import { ClosePreview } from '../../components/ui/glass'
import { Fact } from '../../components/ui/Fact'
import { errorText } from '../../lib/api/client'
import type { Detail, Provider } from '../../lib/api/contracts'
import { activityLabel, dateLabel, sizeLabel } from '../../lib/utils/format'
import { readSession, sessionPath } from './api'
import { Conversation } from './Conversation'
import { SessionStatus } from './SessionList'
import { SessionRemoval } from './removal'

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
      <h2 title={session?.title ?? undefined}>{session ? session.title || 'Untitled session' : 'Session details'}</h2>
      {session && <SessionStatus session={session} />}
      <ClosePreview onClick={onClose} />
    </header>
    {detail && session ? <>
      <dl className="preview-facts">
        <div><dt>Last activity</dt><dd title={dateLabel(session.last_active_at)}>{activityLabel(session.last_active_at)}</dd></div>
        <Fact label="Session size" value={sizeLabel(session.transcript_bytes)} />
        <Fact label="Session ID" value={session.native_id} mono />
      </dl>
      <div className="preview-body session-preview-body">
        {detail.conversation_preview ? <Conversation path={path} /> : <EmptyState title="Conversation preview unavailable" />}
      </div>
      <SessionRemoval path={path} detail={detail} onChange={onChange} />
    </> : error ? <Notice tone="error">{error}</Notice> : <LoadingState title="Loading session details" />}
  </>
}
