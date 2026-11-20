import { useEffect, useRef, useState } from 'react'
import { ConfirmationDialog } from '../../components/ui/ConfirmationDialog'
import { LoadingState, Notice } from '../../components/ui/feedback'
import { GlassButton } from '../../components/ui/glass'
import { errorText } from '../../lib/api/client'
import type { ClaudeReview, CodexReview, DeletionResult, Detail } from '../../lib/api/contracts'
import { numberLabel, pathLabel, sizeLabel } from '../../lib/utils/format'
import { confirmRemoval, reviewRemoval } from './api'

type Review = ClaudeReview | CodexReview

function ReviewedResources({ review }: { review: Review }) {
  if ('trees' in review.plan) {
    const descendants = review.plan.trees.flatMap(tree => tree.descendants)
    return <>
      <p>{numberLabel(review.plan.trees.length)} selected {review.plan.trees.length === 1 ? 'session' : 'sessions'} · {numberLabel(descendants.length)} verified descendants</p>
      {descendants.length > 0 && <details>
        <summary>Verified descendants</summary>
        <ul className="removal-targets">
          {descendants.map(child => <li key={child.id}><code>{child.id}</code><br />
            Level {numberLabel(child.depth)} · Parent <code>{child.parent}</code>
            {child.archived && ' · Archived'}{!child.present && ' · Transcript and record unavailable'}
          </li>)}
        </ul>
      </details>}
    </>
  }

  const { directories, files, logical_bytes, history_matches, notes } = review.plan
  return <>
    <p>{numberLabel(files.length)} files · {numberLabel(directories.length)} folders · {sizeLabel(logical_bytes)}</p>
    <details>
      <summary>Files and folders</summary>
      <ul className="removal-targets">
        {directories.map(directory => <li key={JSON.stringify([directory.root, directory.relative_path])}>
          <code>{pathLabel(directory.relative_path)}/</code><br />{pathLabel(directory.root)}
        </li>)}
        {files.map(file => <li key={JSON.stringify([file.root, file.relative_path])}>
          <code>{pathLabel(file.relative_path)}</code><br />{pathLabel(file.root)} · {sizeLabel(file.bytes)}
        </li>)}
      </ul>
    </details>
    {history_matches !== null && <p>{numberLabel(history_matches)} matching history entries</p>}
    {notes.length > 0 && <ul>{notes.map(note => <li key={note}>{note}</li>)}</ul>}
    <p>Close Claude before confirming to reduce the chance of concurrent changes.</p>
  </>
}

export function RemovalDialog({ path, title, project = false, onClose, onChange }: {
  path: string; title: string; project?: boolean; onClose: () => void; onChange: (result?: DeletionResult) => void
}) {
  const [review, setReview] = useState<Review>()
  const [reviewError, setReviewError] = useState('')
  const [error, setError] = useState('')
  const [result, setResult] = useState<DeletionResult>()
  const [busy, setBusy] = useState(false)
  const submitted = useRef(false)

  useEffect(() => {
    const controller = new AbortController()
    reviewRemoval<Review>(path, controller.signal).then(response => {
      if (!controller.signal.aborted) setReview(response)
    }).catch(failure => {
      if (!controller.signal.aborted) setReviewError(errorText(failure))
    })
    return () => controller.abort()
  }, [path])

  async function remove() {
    if (!review || submitted.current) return
    submitted.current = true
    setBusy(true)
    try {
      const outcome = await confirmRemoval(path, review.review)
      setResult(outcome)
      onChange(outcome)
    } catch (failure) {
      setError(errorText(failure))
      onChange()
    } finally {
      setBusy(false)
    }
  }

  const finished = !!(result || error)
  return <ConfirmationDialog title={project ? 'Remove project sessions?' : 'Remove this session?'}
    description={project ? 'The reviewed sessions and their verified descendants will be removed.'
      : review && 'files' in review.plan ? 'The reviewed session files and linked local artifacts will be removed.'
      : 'The selected session and its verified descendants will be removed.'}
    onClose={onClose} onConfirm={review && !finished ? () => void remove() : undefined} busy={busy} finished={finished}>
    <div className="confirmation-summary">
      <strong>{title}</strong>
      {review && <ReviewedResources review={review} />}
      {!review && !reviewError && <LoadingState title="Preparing removal review" />}
    </div>
    {(reviewError || error) && <Notice tone="error">{reviewError || error}</Notice>}
    {result && <Notice tone={result.status === 'deleted' ? 'success' : result.status === 'partial' ? 'warning' : 'error'}>
      <strong>{result.status === 'deleted' ? 'Removed' : result.status === 'partial' ? 'Partially removed' : 'Removal failed'}</strong>
      <p>{result.message}</p>
      {result.errors.length > 0 && <ul>{result.errors.map((message, index) => <li key={index}>{message}</li>)}</ul>}
    </Notice>}
  </ConfirmationDialog>
}

export function SessionRemoval({ path, detail, onChange }: { path: string; detail: Detail; onChange: () => void }) {
  const [reviewing, setReviewing] = useState(false)
  const [outcome, setOutcome] = useState<DeletionResult>()
  const unavailable = detail.claude_cleanup_supported ? null : detail.session.removal_reason
  return <footer className="preview-footer">
    {unavailable && <p className="removal-reason">{unavailable}</p>}
    {outcome && <p role="status" className="removal-reason">{outcome.message}</p>}
    <GlassButton tone="danger" disabled={!!unavailable || outcome?.status === 'deleted'}
      onClick={() => setReviewing(true)}>Remove session</GlassButton>
    {reviewing && <RemovalDialog path={path} title={detail.session.title || detail.session.native_id}
      onChange={result => { setOutcome(result); onChange() }} onClose={() => setReviewing(false)} />}
  </footer>
}
