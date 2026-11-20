import { useEffect, useRef, useState } from 'react'
import { motionDuration } from '../utils/motion'

export type PreviewSelection<T> = {
  item: T
  opener: HTMLElement
}

export function usePreview<T>() {
  const [selection, setSelection] = useState<PreviewSelection<T>>()
  const [open, setOpen] = useState(false)
  const closing = useRef<number | undefined>(undefined)

  useEffect(() => () => window.clearTimeout(closing.current), [])

  function show(item: T, opener: HTMLElement) {
    window.clearTimeout(closing.current)
    setSelection({ item, opener })
    setOpen(true)
  }

  function close() {
    setOpen(false)
    window.clearTimeout(closing.current)
    closing.current = window.setTimeout(() => setSelection(undefined), motionDuration('--motion-preview-close'))
  }

  return { selection, open, show, close }
}
