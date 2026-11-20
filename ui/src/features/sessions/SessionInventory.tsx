import { useEffect, useRef, useState, type ReactNode } from 'react'
import { RefreshCw } from 'lucide-react'
import { InventoryHeader } from '../../components/layout/InventoryHeader'
import { SplitWorkspace } from '../../components/layout/SplitWorkspace'
import { EmptyState, LoadingState, Notice, Pagination } from '../../components/ui/feedback'
import { GlassButton, GlassPanel } from '../../components/ui/glass'
import { errorText } from '../../lib/api/client'
import type { Project, Provider } from '../../lib/api/contracts'
import { usePagedResource } from '../../lib/hooks/usePagedResource'
import { usePreview } from '../../lib/hooks/usePreview'
import { numberLabel, pathLabel } from '../../lib/utils/format'
import { listProjects, projectsPath, projectPath, refreshSessions, unassignedPath } from './api'
import { ClaudeDiagnostics } from './ClaudeDiagnostics'
import { ClaudeUsageView } from './ClaudeUsage'
import { useSessionFilters } from './hooks/useSessionFilters'
import { AdvancedFilters, SessionFilterFields } from './SessionFilters'
import { SessionGroup, SessionList, UnassignedSessions } from './SessionList'
import { SessionDetail } from './SessionDetail'
import { RemovalDialog } from './removal'

const projectPageSize = 12

function ProjectSessions({ project, index, query, revision, selected, onSelect, onRemove }: {
  project: Project; index: number; query: string; revision: number; selected?: string
  onSelect: (key: string, opener: HTMLElement) => void; onRemove: (project: Project) => void
}) {
  const [expanded, setExpanded] = useState(index < 2)
  const [visited, setVisited] = useState(expanded)
  const context = (project.roots.length ? project.roots : [
    project.workspace, project.namespace, project.archive_root && pathLabel(project.archive_root),
  ]).filter(Boolean).join(' · ')

  return <SessionGroup title={project.name}
    context={[project.provider === 'codex' ? 'Registered project' : 'Project', context].filter(Boolean).join(' · ')}
    expanded={expanded} onToggle={() => { setExpanded(!expanded); setVisited(true) }}
    count={project.main_conversations} incomplete={project.incomplete}
    onRemove={project.removal_available ? () => onRemove(project) : undefined}>
    {visited && <SessionList key={query} path={`${projectPath(project.provider, project.key)}/sessions`} label={project.name}
      query={query} revision={revision} selected={selected} onSelect={onSelect} expanded={expanded} />}
  </SessionGroup>
}

export function SessionInventory({ provider, heading, switched, onInventoryChange }: {
  provider: Provider; heading: ReactNode; switched: boolean; onInventoryChange: () => void
}) {
  const filters = useSessionFilters()
  const [revision, setRevision] = useState(0)
  const { loading, error, page } = usePagedResource(projectsPath(provider), filters.query, filters.pageIndex, projectPageSize, revision, listProjects, { delay: 150 })
  const detail = usePreview<string>()
  const [projectToRemove, setProjectToRemove] = useState<Project>()
  const [refreshing, setRefreshing] = useState(false)
  const [refreshError, setRefreshError] = useState('')
  const refreshPending = useRef(false)
  const [storageOpened, setStorageOpened] = useState(false)
  const info = page?.provider
  const selected = detail.open ? detail.selection?.item : undefined
  const unavailable = info && info.coverage !== 'partial'
  const emptyTitle = filters.active ? 'No matching sessions or projects' : 'No projects observed'

  useEffect(() => {
    if (!loading && !error && page && filters.pageIndex > 0 && filters.pageIndex * projectPageSize >= page.total) filters.setPageIndex(0)
  }, [loading, error, page, filters.pageIndex, filters.setPageIndex])

  function invalidate() {
    setRevision(value => value + 1)
    onInventoryChange()
  }

  async function refresh() {
    if (refreshPending.current) return
    refreshPending.current = true
    setRefreshing(true)
    setRefreshError('')
    try {
      await refreshSessions(provider)
      filters.setPageIndex(0)
      invalidate()
    } catch (failure) {
      setRefreshError(errorText(failure))
    } finally {
      refreshPending.current = false
      setRefreshing(false)
    }
  }

  return <>
    <SplitWorkspace label="Session details" open={detail.open} opener={detail.selection?.opener}
      onClose={detail.close} preview={detail.selection && <SessionDetail key={detail.selection.item} provider={provider}
        rowKey={detail.selection.item} onClose={detail.close} onChange={invalidate} />}>
      <GlassPanel className="inventory-panel" data-switched={switched} tabIndex={-1} aria-label="Sessions inventory">
        <InventoryHeader heading={heading} count={info?.coverage === 'partial'
          ? `${info.inventory_incomplete ? '≥ ' : ''}${numberLabel(info.total)} sessions`
          : '— sessions'}>
          <SessionFilterFields filters={filters.filters} onChange={filters.update} />
        </InventoryHeader>
        <div className="inventory-body" aria-busy={loading}>
          {refreshError && <Notice tone="error">{refreshError}</Notice>}
          {error ? <Notice tone="error">{error}</Notice>
            : !page && loading ? <LoadingState title="Loading projects" />
            : page?.rows.map((project, index) => <ProjectSessions key={project.key} project={project} index={index}
              query={filters.query} revision={revision} selected={selected} onSelect={detail.show} onRemove={setProjectToRemove} />)}
          {!error && !loading && page?.total === 0 && info?.recent_conversations === 0 &&
            <EmptyState title={unavailable ? 'Archive unavailable' : emptyTitle}>
              {unavailable && <p lang="it">{info.coverage_description}</p>}
            </EmptyState>}
          {page && !error && <Pagination total={page.total} page={filters.pageIndex} pageSize={projectPageSize}
            onPage={filters.setPageIndex} label="Project pages" />}
          {info?.recent_conversations ? <UnassignedSessions key={filters.query} path={unassignedPath(provider)}
            query={filters.query} revision={revision} selected={selected} onSelect={detail.show} label="Unassigned sessions"
            emptyTitle={!loading && page?.total === 0 ? emptyTitle : undefined} /> : null}
        </div>
      </GlassPanel>
    </SplitWorkspace>
    <div className="session-tools">
      <AdvancedFilters filters={filters.filters} provider={provider} active={filters.active}
        onChange={filters.update} onReset={filters.reset} />
      <GlassButton className="square refresh-button" tone="quiet" aria-label="Refresh sessions" title="Refresh sessions"
        disabled={refreshing} onClick={() => void refresh()}><RefreshCw className={refreshing ? 'spin' : undefined} size={15} aria-hidden /></GlassButton>
    </div>
    {info && <details className="inventory-diagnostics">
      <summary>{info.coverage !== 'partial' ? 'Archive unavailable' :
        info.inventory_incomplete || info.projects_incomplete ? 'Partial archive coverage' : 'Archive coverage and diagnostics'}</summary>
      <p lang="it">{info.coverage_description}</p>
      {provider === 'codex' && info.removal_reason && <p lang="it">{info.removal_reason}</p>}
      {info.claude_storage && <ClaudeDiagnostics discovery={info.claude_storage} incomplete={info.inventory_incomplete} />}
      {provider === 'claude' && <details onToggle={event => { if (event.currentTarget.open) setStorageOpened(true) }}>
        <summary>Claude Code storage</summary>
        {storageOpened && <ClaudeUsageView key={revision} onSelect={detail.show} />}
      </details>}
    </details>}
    {projectToRemove && <RemovalDialog path={projectPath(provider, projectToRemove.key)} title={projectToRemove.name}
      project onClose={() => setProjectToRemove(undefined)} onChange={invalidate} />}
  </>
}
