import documentIcon from '../../assets/included-document.svg'
import { Fact } from '../../components/ui/Fact'
import { ClosePreview, GlassButton } from '../../components/ui/glass'
import { dateLabel, numberLabel, sizeLabel } from '../../lib/utils/format'
import type { Group } from './api'

export function ExportDetail({ group, canReview, onReview, onClose }: {
  group: Group; canReview: boolean; onReview: () => void; onClose: () => void
}) {
  return <>
    <header className="preview-heading">
      <h2>{group.name}</h2><ClosePreview onClick={onClose} />
    </header>
    <dl className="preview-facts">
      <Fact label="Export size" value={`${group.complete ? '' : '≥ '}${sizeLabel(group.size.logical_bytes)}`} />
      <Fact label="Files / folders" value={`${group.complete ? '' : '≥ '}${numberLabel(group.size.files)} / ${numberLabel(group.size.directories)}`} />
      <Fact label="Last modified (UTC)" value={dateLabel(group.modified_at)} />
    </dl>
    <div className="preview-body export-detail">
      <section><h3 className="fact-label">Source path</h3><code className="source-path">{group.path}</code></section>
      <section>
        <h3>Included files</h3>
        {group.manifest && <div className="included-file">
          <img src={documentIcon} alt="" width={17} height={17} /><code>{group.manifest}</code>
        </div>}
        <p>{group.manifest ? 'The inventory identifies this manifest and provides aggregate counts for the export.' :
          'Individual file paths are unavailable from the inventory.'}</p>
      </section>
      <details className="preview-diagnostics">
        <summary>Export metadata</summary>
        <dl className="facts">
          <Fact label="Root" value={group.root} mono />
          <Fact label="Format" value={group.format} />
          <Fact label="Observed allocation" value={sizeLabel(group.size.allocated_bytes)} />
          <Fact label="Created (UTC)" value={dateLabel(group.created_at)} />
          {group.producer && <Fact label="Producer" value={group.producer} />}
          {group.session_id && <Fact label="Declared session" value={group.session_id} mono />}
          {(group.shared_entries > 0 || group.linked_entries > 0) &&
            <Fact label="Observed links" value={`${numberLabel(group.shared_entries)} hardlinks · ${numberLabel(group.linked_entries)} skipped links`} />}
        </dl>
      </details>
    </div>
    <footer className="preview-footer">
      {!group.removal_available && <p className="removal-reason">{group.reason}</p>}
      <GlassButton tone="danger" disabled={!canReview} onClick={onReview}>Review removal</GlassButton>
    </footer>
  </>
}
