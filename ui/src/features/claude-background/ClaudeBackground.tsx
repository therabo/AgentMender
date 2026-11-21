import { useEffect, useRef } from 'react'
import { motionDuration } from '../../lib/utils/motion'
import type { ClaudeScene } from './scene'
import './background.css'

const modelPath = '/assets/Clawd-Animator.glb'

export function ClaudeBackground({ active }: { active: boolean }) {
  const surface = useRef<HTMLDivElement>(null)
  const scene = useRef<ClaudeScene | null>(null)
  const bytes = useRef<ArrayBuffer | null>(null)
  const unavailable = useRef(false)

  useEffect(() => () => {
    scene.current?.dispose()
    scene.current = null
  }, [])

  useEffect(() => {
    const host = surface.current
    if (!host) return
    function release() {
      scene.current?.dispose()
      scene.current = null
    }

    if (!active) {
      host.dataset.visible = 'false'
      if (!scene.current) return
      scene.current.pause()
      const duration = motionDuration('--motion-provider-switch')
      if (!duration) {
        release()
        return
      }
      const timer = window.setTimeout(release, duration)
      return () => window.clearTimeout(timer)
    }

    if (unavailable.current) return
    const fallback = () => {
      unavailable.current = true
      host.dataset.visible = 'false'
      release()
      bytes.current = null
    }

    if (scene.current) {
      scene.current.start()
      host.dataset.visible = 'true'
      return
    }

    const load = new AbortController()
    const { signal } = load
    async function mount(target: HTMLDivElement) {
      try {
        if (!bytes.current) {
          const response = await fetch(modelPath, { signal, credentials: 'omit' })
          signal.throwIfAborted()
          if (!response.ok || response.headers.get('Content-Type') !== 'model/gltf-binary') {
            fallback()
            return
          }
          const model = await response.arrayBuffer()
          signal.throwIfAborted()
          bytes.current = model
        }
        const { createScene } = await import('./scene')
        signal.throwIfAborted()
        const next = await createScene(target, bytes.current, signal, fallback)
        if (signal.aborted || unavailable.current) {
          next.dispose()
          return
        }
        scene.current = next
        next.start()
        target.dataset.visible = 'true'
      } catch {
        if (!signal.aborted) fallback()
      }
    }

    void mount(host)
    return () => load.abort()
  }, [active])

  return <div className="claude-background" ref={surface} aria-hidden="true" />
}
