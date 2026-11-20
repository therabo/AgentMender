import { useEffect, useLayoutEffect, useRef, type ReactNode } from 'react'

const narrowView = '(max-width: 840px)'

function PreviewSurface({ open, label, opener, onClose, children }: {
  open: boolean; label: string; opener: HTMLElement; onClose: () => void; children: ReactNode
}) {
  const surface = useRef<HTMLDialogElement>(null)
  useLayoutEffect(() => {
    const dialog = surface.current
    if (!dialog) return
    const query = window.matchMedia(narrowView)
    function present() {
      if (!dialog) return
      if (dialog.open) dialog.close()
      if (query.matches) dialog.showModal()
      else dialog.show()
      dialog.querySelector<HTMLButtonElement>('[data-preview-close]')?.focus({ preventScroll: true })
    }
    present()
    query.addEventListener('change', present)
    return () => {
      query.removeEventListener('change', present)
      dialog.close()
      if (opener.isConnected) opener.focus({ preventScroll: true })
      else document.querySelector<HTMLElement>('.inventory-panel')?.focus({ preventScroll: true })
    }
  }, [opener])

  useEffect(() => {
    if (!open) return
    function escape(event: KeyboardEvent) {
      if (event.key === 'Escape' && !event.defaultPrevented && !document.querySelector(':modal')) {
        event.preventDefault()
        onClose()
      }
    }
    document.addEventListener('keydown', escape)
    return () => document.removeEventListener('keydown', escape)
  }, [open, onClose])

  return <dialog ref={surface} className="glass preview-surface" aria-label={label} data-open={open}
    inert={!open} onCancel={event => {
      if (event.target !== event.currentTarget) return
      event.preventDefault()
      if (open) onClose()
    }}
    onClick={event => { if (open && event.target === event.currentTarget) onClose() }}>
    <div className="preview-content">{children}</div>
  </dialog>
}

export function SplitWorkspace({ children, preview, label, opener, open, onClose }: {
  children: ReactNode; preview?: ReactNode; label: string; opener?: HTMLElement; open: boolean; onClose: () => void
}) {
  return <div className="split-workspace" data-split={open}>
    <div className="inventory-slot">{children}</div>
    <div className="preview-slot">
      {opener && <PreviewSurface open={open} label={label} opener={opener} onClose={onClose}>{preview}</PreviewSurface>}
    </div>
  </div>
}
