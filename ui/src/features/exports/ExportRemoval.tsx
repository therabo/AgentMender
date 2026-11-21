import { useEffect, useRef, useState } from 'react'
import { ConfirmationDialog } from '../../components/ui/ConfirmationDialog'
import { LoadingState, Notice } from '../../components/ui/feedback'
import { errorText } from '../../lib/api/client'
import { numberLabel, sizeLabel } from '../../lib/utils/format'
import { confirmExports, reviewExports, type Outcome, type Preview } from './api'

export function ExportOutcome({ outcome }: { outcome: Outcome }) {
  const tone = !outcome.deleted.length && outcome.not_deleted.some(item => item.error)
    ? 'error' : outcome.not_deleted.length ? 'warning' : 'success'
  return <Notice tone={tone}>
    <strong>{numberLabel(outcome.deleted.length)} removed · {numberLabel(outcome.not_deleted.length)} preserved</strong>
    <ul className="removal-targets">
      {outcome.deleted.map(item => <li key={item.key}><strong>{item.name}</strong> · Removed<br /><code>{item.root}</code></li>)}
      {outcome.not_deleted.map(item => <li key={item.key}><strong>{item.name}</strong> · {item.error ? 'Error' : 'Preserved'}<br />
        <code>{item.root}</code><br />{item.reason}</li>)}
    </ul>
  </Notice>
}

export function ExportRemoval({ keys, onClose, onOutcome }: {
  keys: readonly string[]; onClose: () => void; onOutcome: (outcome?: Outcome) => void
}) {
  const [preview, setPreview] = useState<Preview>()
  const [error, setError] = useState('')
  const [outcome, setOutcome] = useState<Outcome>()
  const [busy, setBusy] = useState(false)
  const submitted = useRef(false)

  useEffect(() => {
    const controller = new AbortController()
    reviewExports(keys, controller.signal).then(value => {
      if (!controller.signal.aborted) setPreview(value)
    }).catch(failure => {
      if (!controller.signal.aborted) setError(errorText(failure))
    })
    return () => controller.abort()
  }, [keys])

  async function remove() {
    if (submitted.current || !preview?.receipt) return
    submitted.current = true
    setBusy(true)
    try {
      const result = await confirmExports(preview.receipt)
      setOutcome(result)
      onOutcome(result)
    } catch (failure) {
      setError(errorText(failure))
      onOutcome()
    } finally {
      setBusy(false)
    }
  }

  const finished = !!(outcome || submitted.current && error)
  const singular = keys.length === 1
  return <ConfirmationDialog title={singular ? 'Remove selected export?' : 'Remove selected exports?'}
    description={singular ? 'The reviewed export group will be deleted from local storage.' :
      'Removable groups in this review will be deleted from local storage.'}
    onClose={onClose} onConfirm={preview?.receipt && !finished ? () => void remove() : undefined} busy={busy} finished={finished}>
    {error && <Notice tone="error">{error}</Notice>}
    {!preview && !error && <LoadingState title="Preparing export review" />}
    {preview && <div className="confirmation-summary">
      <strong>{numberLabel(preview.groups.length)} {singular ? 'export group' : 'export groups'} selected</strong>
      <p>{numberLabel(preview.removable)} removable · {numberLabel(preview.protected)} protected · {sizeLabel(preview.size.logical_bytes)} {preview.complete ? 'total' : 'observed'}</p>
      <ul className="removal-targets">
        {preview.groups.map(group => <li key={group.key}>
          <strong>{group.name}</strong> · {group.removal_available ? 'Removable' : group.reason}<br /><code>{group.path}</code>
        </li>)}
      </ul>
    </div>}
    {outcome && <ExportOutcome outcome={outcome} />}
  </ConfirmationDialog>
}
