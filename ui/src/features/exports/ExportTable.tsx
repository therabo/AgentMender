import fileIcon from '../../assets/export-document.svg'
import { dateLabel, numberLabel, sizeLabel } from '../../lib/utils/format'
import { exportSelectionLimit, type Group } from './api'
import { categoryLabel } from './category'

export function ExportTable({ rows, checked, active, onInspect, onCheck }: {
  rows: Group[]; checked: ReadonlySet<string>; active?: string
  onInspect: (group: Group, opener: HTMLElement) => void
  onCheck: (group: Group, opener: HTMLElement) => void
}) {
  return <table className="export-table" aria-label="Export groups">
    <thead><tr>
      <th scope="col">Export group</th>
      <th scope="col" className="export-category">Category</th>
      <th scope="col">Size</th>
      <th scope="col" className="export-modified">Last modified</th>
    </tr></thead>
    <tbody>{rows.map(group => {
      const selected = checked.has(group.key)
      return <tr key={group.key} data-active={active === group.key} onClick={event => {
        const opener = event.currentTarget.querySelector<HTMLButtonElement>('button')
        if (opener) onInspect(group, opener)
      }}>
        <td>
          <div className="export-name">
            <label className="export-check" onClick={event => event.stopPropagation()}>
              <input type="checkbox" aria-label={`Select ${group.name}`} checked={selected}
                disabled={!selected && checked.size >= exportSelectionLimit} onChange={event => onCheck(group, event.currentTarget)} />
            </label>
            <button type="button" className="export-open" aria-pressed={active === group.key}
              onClick={event => { event.stopPropagation(); onInspect(group, event.currentTarget) }}>
              <img src={fileIcon} alt="" width={18} height={20} />
              <span className="row-description">
                <strong title={group.name}>{group.name}</strong>
                <span className="export-path" title={group.path}>{group.path}</span>
                <span className="export-compact">{categoryLabel[group.protection]} · {numberLabel(group.size.files)} files</span>
              </span>
            </button>
          </div>
        </td>
        <td className="export-category">{categoryLabel[group.protection]}</td>
        <td className="export-size">{sizeLabel(group.complete ? group.size.logical_bytes : null)}</td>
        <td className="export-modified"><time>{dateLabel(group.modified_at)}</time></td>
      </tr>
    })}</tbody>
  </table>
}
