import type { ReactNode } from 'react'
import { SegmentedControl } from '../components/ui/glass'
import { ClaudeBackground } from '../features/claude-background/ClaudeBackground'
import type { Provider } from '../lib/api/contracts'

const providers = [
  { value: 'codex', label: 'Codex' },
  { value: 'claude', label: 'Claude Code' },
] as const

export function AppShell({ provider, onProviderChange, children }: {
  provider: Provider; onProviderChange: (provider: Provider) => void; children: ReactNode
}) {
  return <div className="shell" data-provider={provider}>
    <div className="aurora" data-theme="codex" aria-hidden />
    <div className="aurora" data-theme="claude" aria-hidden />
    <ClaudeBackground active={provider === 'claude'} />
    <header className="app-header">
      <div className="app-mark" role="img" aria-label="SessionMender">S</div>
      <SegmentedControl className="provider-switch" label="Provider" items={providers} value={provider} onChange={onProviderChange} />
    </header>
    <main className="workspace">{children}</main>
  </div>
}
