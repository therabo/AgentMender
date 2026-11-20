import { useEffect, useId, useRef, useState, type ReactNode } from 'react'
import trashIcon from '../../assets/confirm-trash.svg'
import closeIcon from '../../assets/confirm-close.svg'
import { motionDuration } from '../../lib/utils/motion'
import { GlassButton } from './glass'

export function ConfirmationDialog({ title, description, children, onClose, onConfirm, busy, finished = false }: {
  title: string
  description?: string
  children: ReactNode
  onClose: () => void
  onConfirm?: () => void
  busy: boolean
  finished?: boolean
}) {
  const ref = useRef<HTMLDialogElement>(null)
  const closeTimer = useRef<number | undefined>(undefined)
  const [closing, setClosing] = useState(false)
  const titleId = useId()
  const descriptionId = useId()
  useEffect(() => {
    const dialog = ref.current
    if (!dialog) return
    const opener = document.activeElement
    dialog.showModal()
    dialog.querySelector<HTMLButtonElement>('[data-cancel]')?.focus()
    return () => {
      window.clearTimeout(closeTimer.current)
      dialog.close()
      if (opener instanceof HTMLElement && opener.isConnected) opener.focus({ preventScroll: true })
    }
  }, [])

  useEffect(() => {
    if (finished && !busy) ref.current?.querySelector<HTMLButtonElement>('[data-cancel]')?.focus()
  }, [finished, busy])

  function dismiss() {
    if (busy || closeTimer.current !== undefined) return
    const duration = motionDuration('--motion-dialog-close')
    if (!duration) {
      onClose()
      return
    }
    setClosing(true)
    closeTimer.current = window.setTimeout(onClose, duration)
  }

  return <dialog ref={ref} className="glass confirmation" data-closing={closing}
    aria-labelledby={titleId} aria-describedby={description ? descriptionId : undefined}
    onCancel={event => {
      event.preventDefault()
      event.stopPropagation()
      dismiss()
    }}
    onClick={event => { if (event.target === event.currentTarget) dismiss() }}>
    <div className="confirmation-content">
      <header>
        <span className="confirmation-icon"><img src={trashIcon} alt="" width={25} height={26} /></span>
        <h2 id={titleId}>{title}</h2>
        <GlassButton className="square" aria-label="Close confirmation" onClick={dismiss} disabled={busy || closing}>
          <img src={closeIcon} alt="" width={19} height={20} />
        </GlassButton>
      </header>
      {description && <p id={descriptionId}>{description}</p>}
      <div className="confirmation-targets">{children}</div>
      {onConfirm && <p className="removal-warning">This action cannot be undone. No backup will be created.</p>}
      <footer>
        <GlassButton onClick={dismiss} disabled={busy || closing} data-cancel>{finished ? 'Close' : 'Cancel'}</GlassButton>
        {onConfirm && <GlassButton tone="danger" onClick={onConfirm} disabled={busy || closing}>{busy ? 'Removing…' : 'Confirm removal'}</GlassButton>}
      </footer>
      {busy && <span className="sr-only" role="status">Removal in progress</span>}
    </div>
  </dialog>
}
