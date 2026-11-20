import { useId } from 'react'
import { GlassButton } from '../../components/ui/glass'
import { LoadingState, Notice } from '../../components/ui/feedback'
import { useTranscript } from './hooks/useTranscript'

export function Conversation({ path }: { path: string }) {
  const title = useId()
  const transcript = useTranscript(path)
  return <section className="conversation">
    <h3 id={title}>Conversation preview</h3>
    <div className="conversation-scroll" role="region" aria-labelledby={title} aria-busy={transcript.loading} tabIndex={0}>
      {transcript.messages.map((message, index) =>
        <article className="chat-message" key={index} data-role={message.role}
          data-step={index >= transcript.pageStart ? Math.min(5, index - transcript.pageStart) : undefined}>
          <strong>{message.role === 'user' ? 'User' : 'Assistant'}</strong>
          <p>{message.text}</p>
        </article>)}
      {transcript.loading && <LoadingState title="Loading conversation" />}
      {transcript.error && <Notice tone="error">{transcript.error}</Notice>}
      {!transcript.loading && !transcript.error && transcript.messages.length === 0 && <p>No displayable messages.</p>}
      {transcript.complete === false && !transcript.next && <Notice tone="warning">Partial preview. Some records are unavailable.</Notice>}
      <div className="conversation-actions">
        {transcript.next && <GlassButton disabled={transcript.loading} onClick={transcript.loadMore}>{transcript.error ? 'Retry next page' : 'Load more messages'}</GlassButton>}
        {transcript.error && <GlassButton tone="quiet" onClick={transcript.restart}>Reload conversation</GlassButton>}
      </div>
    </div>
  </section>
}
