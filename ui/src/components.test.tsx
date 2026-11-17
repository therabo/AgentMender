// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'
import { useState } from 'react'
import type { Project, Session } from './api'
import { Pagination, ProjectAccordion, SessionList, SessionTable } from './components'
import { dateLabel, sizeLabel } from './format'

afterEach(cleanup)

it('opens only the selected session and labels missing metadata and protected rows', () => {
  const base: Session = {
    key: 'second', native_id: 'native-id', title: null, provider: 'codex', project: null,
    workspace: null, created_at: null, last_active_at: null, transcript_bytes: null,
    state: 'archived', kind: 'unknown', protected: true,
    removal_reason: 'Protetta',
  }
  const select = vi.fn()
  render(<SessionTable records={[{ ...base, key: 'first', native_id: 'different-id', title: 'Altro' }, base]} onSelect={select} />)
  fireEvent.click(screen.getByRole('button', { name: 'Senza titolo' }))
  expect(select.mock.calls).toEqual([['second']])
  expect(screen.getAllByText('Non disponibile')).toHaveLength(4)
  expect(screen.getAllByText('Creata · Non disponibile')).toHaveLength(2)
  expect(screen.getAllByText('Attività · Non disponibile')).toHaveLength(2)
  expect(screen.getAllByText('Protetta')).toHaveLength(2)
})

it('bounds keyboard-accessible pagination to the result count', () => {
  const change = vi.fn()
  const view = render(<Pagination total={18} page={0} pageSize={12} onPage={change} />)
  expect((screen.getByRole('button', { name: 'Pagina precedente' }) as HTMLButtonElement).disabled).toBe(true)
  fireEvent.click(screen.getByRole('button', { name: 'Pagina successiva' }))
  expect(change).toHaveBeenCalledWith(1)
  view.rerender(<Pagination total={18} page={1} pageSize={12} onPage={change} />)
  expect(screen.getByText('13–18 di 18')).toBeTruthy()
  expect((screen.getByRole('button', { name: 'Pagina successiva' }) as HTMLButtonElement).disabled).toBe(true)
})

it('distinguishes unknown metadata from zero bytes and the Unix epoch', () => {
  expect(dateLabel(null)).toBe('Non disponibile')
  expect(dateLabel(0)).toContain('1970')
  expect(sizeLabel(null)).toBe('Non disponibile')
  expect(sizeLabel(0)).toBe('0 B')
  expect(sizeLabel(1024)).toBe('1 KiB')
})

it('expands projects independently and keeps removal separate from the accordion toggle', () => {
  const remove = vi.fn()
  const base: Project = {
    key: 'atlas', name: 'Atlas', provider: 'codex', workspace: '/one/Atlas', namespace: null,
    main_conversations: 2, last_active_at: null, transcript_bytes: 1024,
    transcript_bytes_incomplete: true, incomplete: true, persistent_instance: true,
    roots: ['/one/Atlas', '/two/Atlas'], registration: 'native', removal_available: true, removal_reason: null,
  }
  function Projects() {
    const [open, setOpen] = useState<Record<string, boolean>>({})
    return [base, { ...base, key: 'other', name: 'Altro', removal_available: false }].map(project =>
      <ProjectAccordion key={project.key} project={project} expanded={!!open[project.key]}
        onToggle={() => setOpen(current => ({ ...current, [project.key]: !current[project.key] }))}
        onRemove={remove}>
        <p>Sessioni di {project.name}</p>
      </ProjectAccordion>)
  }

  render(<Projects />)
  expect(screen.getAllByText('/one/Atlas · /two/Atlas')).toHaveLength(2)
  const atlas = screen.getByRole('button', { name: 'Atlas' })
  const other = screen.getByRole('button', { name: 'Altro' })
  expect(atlas.getAttribute('aria-expanded')).toBe('false')
  expect(screen.queryByText('Sessioni di Atlas')).toBeNull()
  fireEvent.click(atlas)
  fireEvent.click(other)
  expect(atlas.getAttribute('aria-expanded')).toBe('true')
  expect(document.getElementById(atlas.getAttribute('aria-controls')!)?.textContent).toContain('Sessioni di Atlas')
  expect(screen.getByText('Sessioni di Altro')).toBeTruthy()
  fireEvent.click(screen.getByRole('button', { name: 'Rimuovi sessioni del progetto Atlas' }))
  expect(remove).toHaveBeenCalledOnce()
  expect(atlas.getAttribute('aria-expanded')).toBe('true')
  expect(screen.queryByRole('button', { name: 'Rimuovi sessioni del progetto Altro' })).toBeNull()
  fireEvent.click(atlas)
  expect(screen.queryByText('Sessioni di Atlas')).toBeNull()
  expect(screen.getByText('Sessioni di Altro')).toBeTruthy()
})

it('pages and opens Recenti with the current archive filters', async () => {
  const recent: Session = {
    key: 'recent', native_id: 'recent-id', title: 'Conversazione indipendente',
    provider: 'codex', project: null, workspace: '/one/Atlas',
    created_at: null, last_active_at: null, transcript_bytes: null,
    state: 'archived', kind: 'main', protected: false, removal_reason: null,
  }
  const fetch = vi.spyOn(globalThis, 'fetch')
    .mockResolvedValueOnce(Response.json({ rows: [{ ...recent, key: 'first', title: 'Prima pagina' }], total: 13 }))
    .mockResolvedValueOnce(Response.json({ rows: [recent], total: 13 }))
  const select = vi.fn()
  try {
    render(<SessionList path="/api/codex/recents" filters="state=archived&search=Atlas"
      revision="0" paginationLabel="Pagine delle conversazioni recenti" emptyLabel="Nessuna conversazione"
      onSelect={select} />)
    await screen.findByRole('button', { name: 'Prima pagina' })
    fireEvent.click(screen.getByRole('button', { name: 'Pagina successiva' }))
    const session = await screen.findByRole('button', { name: 'Conversazione indipendente' })
    expect(fetch.mock.calls.map(([url]) => url)).toEqual([
      '/api/codex/recents?state=archived&search=Atlas&limit=12&offset=0',
      '/api/codex/recents?state=archived&search=Atlas&limit=12&offset=12',
    ])
    fireEvent.click(session)
    expect(select).toHaveBeenCalledWith('recent')
  } finally {
    fetch.mockRestore()
  }
})
