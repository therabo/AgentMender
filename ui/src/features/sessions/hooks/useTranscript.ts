import { useEffect, useState } from 'react'
import { errorText } from '../../../lib/api/client'
import type { Transcript } from '../../../lib/api/contracts'
import { readConversation } from '../api'

type Entry = Transcript['messages'][number]
type TranscriptState = {
  messages: Entry[]; next: string | null; complete?: boolean; loading: boolean; error?: string; pageStart: number
}
const initial: TranscriptState = { messages: [], next: null, loading: true, pageStart: 0 }

export function useTranscript(path: string) {
  const [state, setState] = useState(initial)
  const [request, setRequest] = useState<{ cursor?: string }>({})

  useEffect(() => {
    const controller = new AbortController()
    readConversation(path, controller.signal, request.cursor).then(page => {
      if (controller.signal.aborted) return
      setState(previous => ({
        messages: [...previous.messages, ...page.messages],
        pageStart: previous.messages.length,
        next: page.next,
        complete: page.complete,
        loading: false,
      }))
    }).catch(failure => {
      if (controller.signal.aborted) return
      setState(previous => ({ ...previous, loading: false, error: errorText(failure) }))
    })
    return () => controller.abort()
  }, [path, request])

  function loadMore() {
    if (state.loading || !state.next) return
    setState(previous => ({ ...previous, error: undefined, loading: true }))
    setRequest({ cursor: state.next })
  }

  function restart() {
    setState(initial)
    setRequest({})
  }

  return { ...state, loadMore, restart }
}
