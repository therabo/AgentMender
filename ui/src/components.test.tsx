// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'
import type { Session } from './api'
import { Pagination, SessionTable } from './components'
import { dateLabel, sizeLabel } from './format'

afterEach(cleanup)

it('opens only the selected session and labels missing metadata and protected rows', () => {
  const base: Session = {
    key: 'second', native_id: 'native-id', title: null, provider: 'codex', project: null,
    workspace: null, created_at: null, last_active_at: null, transcript_bytes: null,
    state: 'archived', kind: 'unknown', parent: null, forked_from: null, protected: true,
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
