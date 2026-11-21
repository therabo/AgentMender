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
import { numberLabel } from '../../lib/utils/format'
import { listProjects, projectsPath, projectPath, refreshSessions, unassignedPath } from './api'
import { useSessionFilters } from './hooks/useSessionFilters'
import { SessionFilterFields } from './SessionFilters'
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
  const context = project.roots.join(' · ')

  return <SessionGroup title={project.name}
    context={[project.provider === 'codex' ? 'Registered project' : 'Project', context].filter(Boolean).join(' · ')}
    expanded={expanded} onToggle={() => { setExpanded(!expanded); setVisited(true) }}
    count={project.main_conversations}
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
  const info = page?.provider
  const selected = detail.open ? detail.selection?.item : undefined
  const unavailable = info && info.coverage !== 'available'
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
        <InventoryHeader heading={heading} count={info?.coverage === 'available'
          ? `${numberLabel(info.total)} ${info.sessions_observed_only ? 'observed ' : ''}sessions`
          : '— sessions'}
          action={<GlassButton className="square" tone="quiet" aria-label="Refresh sessions" title="Refresh sessions"
            disabled={refreshing} onClick={() => void refresh()}><RefreshCw className={refreshing ? 'spin' : undefined} size={15} aria-hidden /></GlassButton>}>
          <SessionFilterFields filters={filters.filters} onChange={filters.update} />
        </InventoryHeader>
        <div className="inventory-body" aria-busy={loading}>
          {refreshError && <Notice tone="error">{refreshError}</Notice>}
          {error ? <Notice tone="error">{error}</Notice>
            : !page && loading ? <LoadingState title="Loading projects" />
            : page?.rows.map((project, index) => <ProjectSessions key={project.key} project={project} index={index}
              query={filters.query} revision={revision} selected={selected} onSelect={detail.show} onRemove={setProjectToRemove} />)}
          {!error && !loading && page?.total === 0 && info?.unassigned_sessions === 0 &&
            <EmptyState title={unavailable ? 'Archive unavailable' : emptyTitle}>
              {unavailable && <p lang="it">{info.archive_error}</p>}
            </EmptyState>}
          {page && !error && <Pagination total={page.total} page={filters.pageIndex} pageSize={projectPageSize}
            onPage={filters.setPageIndex} label="Project pages" />}
          {info?.unassigned_sessions ? <UnassignedSessions key={filters.query} path={unassignedPath(provider)}
            query={filters.query} revision={revision} selected={selected} onSelect={detail.show} label="Unassigned sessions"
            count={info.unassigned_sessions}
            emptyTitle={!loading && page?.total === 0 ? emptyTitle : undefined} /> : null}
        </div>
      </GlassPanel>
    </SplitWorkspace>
    {projectToRemove && <RemovalDialog path={projectPath(provider, projectToRemove.key)} title={projectToRemove.name}
      project onClose={() => setProjectToRemove(undefined)} onChange={invalidate} />}
  </>
}
