import fileIcon from '../../assets/export-document.svg'
import { dateLabel, numberLabel, sizeLabel } from '../../lib/utils/format'
import { exportSelectionLimit, type Group } from './api'
import { categoryLabel } from './category'

export function ExportTable({ rows, checked, disabled, onCheck }: {
  rows: Group[]; checked: ReadonlySet<string>; disabled: boolean
  onCheck: (key: string) => void
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
      return <tr key={group.key} data-selected={selected}>
        <td>
          <label className="export-name">
            <span className="export-check">
              <input type="checkbox" aria-label={`Select ${group.name}`} checked={selected}
                disabled={disabled || !selected && checked.size >= exportSelectionLimit} onChange={() => onCheck(group.key)} />
            </span>
            <span className="export-description">
              <img src={fileIcon} alt="" width={18} height={20} />
              <span className="row-description">
                <strong title={group.name}>{group.name}</strong>
                <span className="export-path" title={group.path}>{group.path}</span>
                <span className="export-compact">{categoryLabel[group.protection]} · {numberLabel(group.size.files)} files</span>
              </span>
            </span>
          </label>
        </td>
        <td className="export-category">{categoryLabel[group.protection]}</td>
        <td className="export-size">{sizeLabel(group.complete ? group.size.logical_bytes : null)}</td>
        <td className="export-modified"><time>{dateLabel(group.modified_at)}</time></td>
      </tr>
    })}</tbody>
  </table>
}
