import { useState } from 'react'
import { usePreview } from '../../../lib/hooks/usePreview'
import { exportSelectionLimit, type Group } from '../api'

export function useExportSelection() {
  const preview = usePreview<Group>()
  const [checked, setChecked] = useState<ReadonlySet<string>>(() => new Set())

  function add(group: Group) {
    setChecked(previous => previous.has(group.key) || previous.size >= exportSelectionLimit ? previous : new Set([...previous, group.key]))
  }

  function inspect(group: Group, opener: HTMLElement) {
    add(group)
    preview.show(group, opener)
  }

  function toggle(group: Group, opener: HTMLElement) {
    if (checked.has(group.key)) {
      setChecked(previous => new Set([...previous].filter(key => key !== group.key)))
      if (preview.selection?.item.key === group.key) preview.close()
    } else {
      inspect(group, opener)
    }
  }

  function clear() {
    setChecked(new Set())
    preview.close()
  }

  return { preview, checked, inspect, toggle, clear }
}
