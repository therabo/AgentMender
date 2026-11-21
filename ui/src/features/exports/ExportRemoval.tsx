import { useEffect, useRef, useState } from 'react'
import { ConfirmationDialog } from '../../components/ui/ConfirmationDialog'
import { LoadingState, Notice } from '../../components/ui/feedback'
import { errorText } from '../../lib/api/client'
import { numberLabel, sizeLabel } from '../../lib/utils/format'
import { confirmExports, reviewExports, type Outcome, type Preview } from './api'

type RemovalState =
  | { status: 'confirming'; preview?: Preview }
  | { status: 'deleting' | 'success' }
  | { status: 'review-error' | 'partial' | 'error'; message: string }

export function ExportRemoval({ keys, onClose, onSettled }: {
  keys: readonly string[]
  onClose: () => void
  onSettled: (preservedKeys: string[]) => void
}) {
  const [state, setState] = useState<RemovalState>({ status: 'confirming' })
  const operation = useRef<AbortController | null>(null)
  const submitted = useRef(false)

  useEffect(() => {
    const controller = new AbortController()
    operation.current = controller
    reviewExports(keys, controller.signal).then(preview => {
      if (!controller.signal.aborted) setState({ status: 'confirming', preview })
    }).catch(failure => {
      if (!controller.signal.aborted) setState({ status: 'review-error', message: errorText(failure) })
    })
    return () => controller.abort()
  }, [keys])

  useEffect(() => {
    if (state.status !== 'success') return
    const timer = window.setTimeout(onClose, 1200)
    return () => window.clearTimeout(timer)
  }, [state.status, onClose])

  async function remove(receipt: string) {
    const signal = operation.current?.signal
    if (submitted.current || !signal || signal.aborted) return
    submitted.current = true
    setState({ status: 'deleting' })

    let outcome: Outcome
    try {
      outcome = await confirmExports(receipt, signal)
    } catch (failure) {
      if (!signal.aborted) {
        setState({ status: 'error', message: errorText(failure) })
        onSettled([])
      }
      return
    }
    if (signal.aborted) return

    const protectedExports = outcome.not_deleted.filter(item => !item.error)
    const deletionError = outcome.not_deleted.find(item => item.error)
    const preservedMessage = 'Protected exports were preserved.'
    if (!outcome.deleted.length) {
      setState({ status: 'error', message: deletionError?.reason ?? (protectedExports.length ? preservedMessage : 'No exports were removed.') })
    } else if (outcome.not_deleted.length) {
      const message = deletionError
        ? `Some exports could not be removed.${protectedExports.length ? ` ${preservedMessage}` : ''}`
        : preservedMessage
      setState({ status: 'partial', message })
    } else {
      setState({ status: 'success' })
    }
    onSettled(protectedExports.map(item => item.key))
  }

  const preview = state.status === 'confirming' ? state.preview : undefined
  const receipt = preview?.receipt
  const singular = keys.length === 1
  const title = {
    confirming: singular ? 'Remove selected export?' : 'Remove selected exports?',
    'review-error': 'Export review unavailable',
    deleting: 'Removing exports',
    success: 'Deletion completed',
    partial: 'Deletion partially completed',
    error: 'Deletion failed',
  }[state.status]
  return <ConfirmationDialog title={title}
    description={state.status === 'confirming' ? 'Removable exports in this review will be deleted from local storage.' : undefined}
    onClose={onClose} onConfirm={receipt ? () => void remove(receipt) : undefined}
    busy={state.status === 'deleting'} finished={state.status === 'partial' || state.status === 'error'}
    completed={state.status === 'success'}>
    {state.status === 'partial' && <Notice tone="warning">{state.message}</Notice>}
    {(state.status === 'error' || state.status === 'review-error') && <Notice tone="error">{state.message}</Notice>}
    {state.status === 'confirming' && !preview && <LoadingState title="Preparing export review" />}
    {state.status === 'deleting' && <LoadingState title="Removing files" />}
    {preview && <div className="confirmation-summary">
      <strong>{numberLabel(preview.groups.length)} {singular ? 'export group' : 'export groups'} selected</strong>
      <p>{numberLabel(preview.removable)} removable · {numberLabel(preview.protected)} protected · {sizeLabel(preview.size.logical_bytes)} {preview.complete ? 'total' : 'observed'}</p>
      <ul className="removal-targets">
        {preview.groups.map(group => <li key={group.key}>
          <strong>{group.name}</strong> · {group.removal_available ? 'Removable' : group.reason}<br /><code>{group.path}</code>
        </li>)}
      </ul>
    </div>}
  </ConfirmationDialog>
}
