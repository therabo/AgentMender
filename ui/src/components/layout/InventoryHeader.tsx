import type { ReactNode } from 'react'

export function InventoryHeader({ heading, count, children }: { heading: ReactNode; count: string; children: ReactNode }) {
  return <header className="inventory-header">
    <div className="inventory-title">{heading}<span>{count}</span></div>
    <div className="inventory-filters">{children}</div>
  </header>
}
