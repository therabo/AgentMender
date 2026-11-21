import type { ReactNode } from 'react'

export function InventoryHeader({ heading, count, action, children }: { heading: ReactNode; count: string; action?: ReactNode; children: ReactNode }) {
  return <header className="inventory-header">
    <div className="inventory-title">{heading}<div className="inventory-count"><span>{count}</span>{action}</div></div>
    <div className="inventory-filters">{children}</div>
  </header>
}
