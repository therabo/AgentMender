import { useEffect, useState } from 'react'
import { authorize, errorText } from '../lib/api/client'
import type { Provider, ProviderInfo } from '../lib/api/contracts'
import { EmptyState, LoadingState, Notice } from '../components/ui/feedback'
import { SegmentedControl } from '../components/ui/glass'
import { listProjects, projectsPath } from '../features/sessions/api'
import { SessionInventory } from '../features/sessions/SessionInventory'
import { Exports } from '../features/exports/Exports'
import { AppShell } from './AppShell'
import { ArchiveStats } from './ArchiveStats'

type View = 'sessions' | 'exports'
const views = [
  { value: 'sessions', label: 'Sessions' },
  { value: 'exports', label: 'Exports' },
] as const

export function App() {
  const [auth, setAuth] = useState<'waiting' | 'ready' | 'failed'>('waiting')
  const [authError, setAuthError] = useState('')
  const [provider, setProvider] = useState<Provider>('codex')
  const [view, setView] = useState<View>('sessions')
  const [viewSwitched, setViewSwitched] = useState(false)
  const [summary, setSummary] = useState<ProviderInfo>()
  const [summaryError, setSummaryError] = useState('')
  const [revision, setRevision] = useState(0)

  useEffect(() => {
    authorize().then(() => setAuth('ready')).catch(failure => {
      setAuthError(errorText(failure))
      setAuth('failed')
    })
  }, [])
  useEffect(() => {
    if (auth !== 'ready') return
    const controller = new AbortController()
    setSummaryError('')
    listProjects(projectsPath(provider), new URLSearchParams({ limit: '1' }), controller.signal)
      .then(page => { if (!controller.signal.aborted) setSummary(page.provider) })
      .catch(failure => {
        if (!controller.signal.aborted) setSummaryError(errorText(failure))
      })
    return () => controller.abort()
  }, [auth, provider, revision])

  function switchProvider(next: Provider) {
    if (next === provider) return
    setProvider(next)
    setView('sessions')
    setViewSwitched(false)
  }

  function switchView(next: View) {
    if (next === view) return
    setView(next)
    setViewSwitched(true)
    window.requestAnimationFrame(() => {
      document.querySelector<HTMLButtonElement>('.inventory-switch button[aria-pressed="true"]')?.focus({ preventScroll: true })
    })
  }

  const heading = provider === 'codex'
    ? <><h1 className="sr-only">{view === 'sessions' ? 'Sessions' : 'Exports'}</h1>
        <SegmentedControl label="Codex inventory" className="inventory-switch" items={views} value={view} onChange={switchView} />
      </>
    : <h1>Sessions</h1>

  return <AppShell provider={provider} onProviderChange={switchProvider}>
    {auth === 'waiting' && <LoadingState title="Connecting to SessionMender" />}
    {auth === 'failed' && <EmptyState title="Local access"><Notice tone="error">{authError}</Notice></EmptyState>}
    {auth === 'ready' && <>
      <ArchiveStats key={`stats-${provider}`} provider={provider} info={summary?.id === provider ? summary : undefined} />
      {summaryError && view === 'exports' && <Notice tone="error">{summaryError}</Notice>}
      {view === 'exports' ? <Exports heading={heading} switched={viewSwitched} /> :
        <SessionInventory key={`sessions-${provider}`} provider={provider} heading={heading} switched={viewSwitched} onInventoryChange={() => setRevision(value => value + 1)} />}
    </>}
  </AppShell>
}
