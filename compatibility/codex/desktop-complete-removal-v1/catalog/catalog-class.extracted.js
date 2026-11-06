class mU{db;hostId;hostKind;activeScan=null;constructor(e,t=mA,r=t===mA?"local":"ssh"){if(this.db=e,this.hostId=t,this.hostKind=r,0===t.length)throw Error("Thread catalog host ID must not be empty");this.db.prepare(`INSERT OR IGNORE INTO local_thread_catalog_sync_state (host_id)
         VALUES (?)`).run(t),this.db.prepare(`INSERT INTO local_thread_catalog_hosts (host_id, host_kind)
         VALUES (?, ?)
         ON CONFLICT (host_id) DO UPDATE SET host_kind = excluded.host_kind`).run(t,r)}listHostIds(e){return this.db.prepare(`SELECT host_id FROM local_thread_catalog_hosts
         WHERE host_kind = ?
         ORDER BY host_id`).all(e).map(e=>e.host_id)}readThreadHostId(e){return this.db.prepare(`SELECT host_id, source_recency_at, source_created_at
         FROM local_thread_catalog
         WHERE thread_id = ?
           AND missing_candidate = 0
           AND source_kind != 'chatgpt'
         ORDER BY source_recency_at DESC, source_created_at DESC
         LIMIT 100`).all(e).sort((e,t)=>t.source_recency_at-e.source_recency_at||t.source_created_at-e.source_created_at||e.host_id.localeCompare(t.host_id))[0]?.host_id}readSyncState(){let e=this.db.prepare(`SELECT
          watermark_updated_at,
          last_full_reconciled_at,
          initial_build_complete
        FROM local_thread_catalog_sync_state
        WHERE host_id = ?`).get(this.hostId),t=mL.safeParse(e);return t.success?{sourceUpdatedAtWatermark:t.data.watermark_updated_at,lastFullReconciliationAt:t.data.last_full_reconciled_at,isComplete:0!==t.data.initial_build_complete,catalogRevision:this.readCatalogRevision()}:{...mB,catalogRevision:this.readCatalogRevision()}}readEntry(e){let t=this.db.prepare(`SELECT * FROM local_thread_catalog
         WHERE host_id = ? AND thread_id = ? AND missing_candidate = 0`).get(this.hostId,e);return null==t?null:mV(t)}readEntries(e){if(e.length>el.iFb)throw Error(`Thread catalog entry request exceeds ${el.iFb}`);if(0===e.length)return[];let t=e.map(()=>"?").join(", "),r=new Map(this.db.prepare(`SELECT * FROM local_thread_catalog
         WHERE host_id = ?
           AND thread_id IN (${t})
           AND missing_candidate = 0`).all(this.hostId,...e).flatMap(e=>{let t=mV(e);return null==t?[]:[[t.threadId,t]]}));return e.flatMap(e=>{let t=r.get(e);return null==t?[]:[t]})}readThreadCount(e){return this.db.prepare(`SELECT (
             SELECT COUNT(*) FROM (
               SELECT 1 FROM local_thread_catalog
               WHERE host_id = ? AND missing_candidate = 0
               LIMIT ?
             )
           ) AS count
           FROM local_thread_catalog_sync_state
           WHERE host_id = ? AND initial_build_complete != 0`).get(e,el.faO,e)?.count??null}readPage({limit:e,cursor:t,filter:r,manualOrder:n,sortKey:i}){if(!Number.isInteger(e)||e<1||e>el.iFb)throw Error(`Thread catalog page limit must be between 1 and ${el.iFb}`);if(null!=n){if(n.threadIds.length>el.iFb)throw Error(`Thread catalog manual order exceeds ${el.iFb}`);if(null!=t)throw Error("Manual thread catalog pages do not use a cursor");return this.readManualPage(n,r,e)}if(null!=t&&t.sortKey!==i)throw Error("Thread catalog cursor uses a different sort order");let s=e+1,a="created_at"===i?"source_created_at":"source_recency_at",o="created_at"===i?"source_updated_at":"source_created_at",l="created_at"===i?t?.sourceCreatedAt:t?.sourceRecencyAt,c="created_at"===i?t?.sourceUpdatedAt:t?.sourceCreatedAt,d="created_at"===i?"local_thread_catalog_cwd_created_idx":"local_thread_catalog_cwd_updated_idx",u=null==t?"":`AND (
             ${a} < ?
             OR (
               ${a} = ?
               AND ${o} < ?
             )
             OR (
               ${a} = ?
               AND ${o} = ?
               AND thread_id > ?
             )
           )`,h=null==t?[]:[l,l,c,l,c,t.threadId],p=(()=>{let e=mH(r);if(null!=e){let t="";return"updated_at"===i&&(r?.projectId!==void 0?t="INDEXED BY local_thread_catalog_project_updated_idx":r?.conversationOrigin==="tpp"&&(t="INDEXED BY local_thread_catalog_origin_updated_idx")),this.db.prepare(`SELECT * FROM local_thread_catalog ${t}
             WHERE host_id = ?
               AND missing_candidate = 0
               ${e.clause}
               AND thread_id NOT IN (SELECT value FROM json_each(?))
               ${u}
             ORDER BY ${a} DESC, ${o} DESC, thread_id
             LIMIT ?`).all(this.hostId,...e.params,JSON.stringify(r?.excludeThreadIds??[]),...h,s)}if(null==r||!0===r.includeAll){let e=JSON.stringify(r?.excludeThreadIds??[]);return this.db.prepare(`SELECT * FROM local_thread_catalog
             WHERE host_id = ?
               AND missing_candidate = 0
               AND thread_id NOT IN (SELECT value FROM json_each(?))
               ${u}
             ORDER BY ${a} DESC, ${o} DESC, thread_id
             LIMIT ?`).all(this.hostId,e,...h,s)}let t=JSON.stringify(r.cwdValues),n=JSON.stringify(m$(r.cwdPrefixes)),l=JSON.stringify(r.excludeThreadIds);return this.db.prepare(`WITH exact_cwd AS (
             SELECT * FROM local_thread_catalog
               INDEXED BY ${d}
             WHERE host_id = ?
               AND missing_candidate = 0
               AND cwd IN (SELECT value FROM json_each(?))
               AND thread_id NOT IN (SELECT value FROM json_each(?))
               ${u}
             ORDER BY ${a} DESC, ${o} DESC, thread_id
             LIMIT ?
           ), prefixed_cwd AS (
             SELECT catalog.* FROM json_each(?) AS prefix
               CROSS JOIN local_thread_catalog AS catalog
               INDEXED BY ${d}
             WHERE catalog.host_id = ?
               AND catalog.missing_candidate = 0
               AND catalog.cwd >= prefix.value
               AND catalog.cwd < prefix.value || char(1114111)
               AND catalog.cwd NOT IN (SELECT value FROM json_each(?))
               AND catalog.thread_id NOT IN (SELECT value FROM json_each(?))
               ${u}
             ORDER BY ${a} DESC, ${o} DESC, thread_id
             LIMIT ?
           ), included_thread AS (
             SELECT * FROM local_thread_catalog
             WHERE host_id = ?
               AND missing_candidate = 0
               AND thread_id IN (SELECT value FROM json_each(?))
               AND cwd NOT IN (SELECT value FROM json_each(?))
               AND NOT EXISTS (
                 SELECT 1 FROM json_each(?) AS prefix
                 WHERE cwd >= prefix.value
                   AND cwd < prefix.value || char(1114111)
               )
               AND thread_id NOT IN (SELECT value FROM json_each(?))
               ${u}
             ORDER BY ${a} DESC, ${o} DESC, thread_id
             LIMIT ?
           ), scoped AS (
             SELECT * FROM exact_cwd
             UNION ALL
             SELECT * FROM prefixed_cwd
             UNION ALL
             SELECT * FROM included_thread
           )
           SELECT * FROM scoped
           ORDER BY ${a} DESC, ${o} DESC, thread_id
           LIMIT ?`).all(this.hostId,t,l,...h,s,n,this.hostId,t,l,...h,s,this.hostId,JSON.stringify(r.includeThreadIds),t,n,l,...h,s,s)})().flatMap(e=>{let t=mV(e);return null==t?[]:[t]}),m=p.slice(0,e),g=m.at(-1);return{entries:m,nextCursor:p.length>e&&null!=g?{sortKey:i,sourceUpdatedAt:g.sourceUpdatedAt,sourceRecencyAt:g.sourceRecencyAt,sourceCreatedAt:g.sourceCreatedAt,threadId:g.threadId}:null}}readManualPage(e,t,r){let n=mH(t,"catalog."),i=null==t||!0===t.includeAll?"":`AND (
             catalog.cwd IN (SELECT value FROM json_each(?))
             OR EXISTS (
               SELECT 1 FROM json_each(?) AS prefix
               WHERE catalog.cwd >= prefix.value
                 AND catalog.cwd < prefix.value || char(1114111)
             )
             OR catalog.thread_id IN (SELECT value FROM json_each(?))
           )`,s=null==t?"":`AND catalog.thread_id NOT IN (SELECT value FROM json_each(?))
           ${i}`,a=null==t?[]:[JSON.stringify(t.excludeThreadIds),...!0===t.includeAll?[]:[JSON.stringify(t.cwdValues),JSON.stringify(m$(t.cwdPrefixes)),JSON.stringify(t.includeThreadIds)]],o=this.db.prepare(`WITH manual AS (
           SELECT CAST(key AS INTEGER) AS position, value AS thread_id
           FROM json_each(?)
           WHERE CAST(key AS INTEGER) >= ?
         )
         SELECT catalog.*, manual.position AS manual_position
         FROM manual
         JOIN local_thread_catalog AS catalog
           ON catalog.host_id = ?
          AND catalog.thread_id = manual.thread_id
         WHERE catalog.missing_candidate = 0
           ${s}
           ${n?.clause??""}
         ORDER BY manual.position
         LIMIT ?`).all(JSON.stringify(e.threadIds),e.startIndex,this.hostId,...a,...n?.params??[],r+1),l=o.slice(0,r).flatMap(e=>{let t=mV(e);return null==t?[]:[t]}),c=o.at(r);return{entries:l,nextManualIndex:null==c?null:Number(c.manual_position),nextCursor:null}}readScanCheckpoint(){let e=this.db.prepare("SELECT checkpoint, failed_at FROM local_thread_catalog_scan_checkpoints WHERE host_id = ?").get(this.hostId);if(null==e)return null;try{let t=mF.safeParse(JSON.parse(e.checkpoint));if(t.success)return{...t.data,failedAt:e.failed_at}}catch{}return this.clearScanCheckpoint(),this.clearSeenEntries(),null}beginScan(e,t=null){if(null!=this.activeScan)throw Error("A local thread catalog scan is already active");let r=t?.attempt.mode===e?t.attempt:this.runInTransaction(()=>(this.clearScanCheckpoint(),this.clearSeenEntries(),{mode:e,observationSequence:this.nextObservationSequence()}));return this.activeScan=r,r}applyScanPage(e,t,r){return this.assertActiveScan(e),this.runInTransaction(()=>{if("full"===e.mode){let e=this.db.prepare(`INSERT OR IGNORE INTO local_thread_catalog_scan_entries (host_id, thread_id)
           VALUES (?, ?)`);for(let r of t)this.assertEntryHost(r),e.run(this.hostId,r.threadId)}let n=this.upsertEntries(t,e.observationSequence,!1);return null!=r&&this.db.prepare(`
          INSERT INTO local_thread_catalog_scan_checkpoints (host_id, checkpoint, failed_at)
          VALUES (?, ?, NULL)
          ON CONFLICT(host_id) DO UPDATE SET checkpoint = excluded.checkpoint, failed_at = NULL
        `).run(this.hostId,JSON.stringify({attempt:e,progress:r})),n})}completeScan(e,t,r=Date.now(),n=null){this.assertActiveScan(e),this.runInTransaction(()=>{if("full"===e.mode){let i=this.mutateThreadCount(`UPDATE local_thread_catalog AS catalog
           SET missing_candidate = 0,
               observation_sequence = ?
           WHERE catalog.host_id = ?
             AND missing_candidate != 0
             AND observation_sequence <= ?
             AND EXISTS (
               SELECT 1 FROM local_thread_catalog_scan_entries AS seen
               WHERE seen.host_id = catalog.host_id
                 AND seen.thread_id = catalog.thread_id
                 AND seen.removed = 0
             )`,e.observationSequence,this.hostId,e.observationSequence),s="local"===this.hostKind||"chatgpt"===this.hostKind?this.mutateThreadCount(`UPDATE local_thread_catalog AS catalog
                 SET missing_candidate = 1,
                     observation_sequence = ?
                 WHERE catalog.host_id = ?
                   AND missing_candidate = 0
                   AND observation_sequence <= ?
                   AND (? IS NULL OR source_updated_at >= ?)
                   AND NOT EXISTS (
                     SELECT 1 FROM local_thread_catalog_scan_entries AS seen
                     WHERE seen.host_id = catalog.host_id
                       AND seen.thread_id = catalog.thread_id
                   )`,e.observationSequence,this.hostId,e.observationSequence,n,n):0;this.db.prepare(`DELETE FROM local_thread_catalog AS catalog
             WHERE catalog.host_id = ?
               AND missing_candidate != 0
               AND observation_sequence < ?
               AND (? IS NULL OR source_updated_at >= ?)
               AND NOT EXISTS (
                 SELECT 1 FROM local_thread_catalog_scan_entries AS seen
                 WHERE seen.host_id = catalog.host_id
                   AND seen.thread_id = catalog.thread_id
               )`).run(this.hostId,e.observationSequence,n,n),(i>0||s>0)&&this.bumpCatalogRevision(),this.db.prepare(`UPDATE local_thread_catalog_sync_state
             SET watermark_updated_at = ?,
                 initial_build_complete = 1,
                 last_full_reconciled_at = ?
             WHERE host_id = ?`).run(t,r,this.hostId)}else this.db.prepare(`UPDATE local_thread_catalog_sync_state
             SET watermark_updated_at = ?
             WHERE host_id = ?`).run(t,this.hostId);this.clearScanCheckpoint(),this.clearSeenEntries()}),this.activeScan=null}abortScan(e){this.assertActiveScan(e),this.clearScanCheckpoint(),this.clearSeenEntries(),this.activeScan=null}pauseScan(e,t){this.assertActiveScan(e),this.db.prepare(`
      UPDATE local_thread_catalog_scan_checkpoints
      SET failed_at = COALESCE(?, failed_at) WHERE host_id = ?
    `).run(t,this.hostId),this.activeScan=null}applyObservedEntry(e){return this.assertEntryHost(e),this.runInTransaction(()=>{let t=this.nextObservationSequence();return this.db.prepare("DELETE FROM local_thread_catalog_scan_entries WHERE host_id = ? AND thread_id = ? AND removed = 1").run(this.hostId,e.threadId),this.upsertEntries([e],t,!0)})}applyObservedTitle(e,t){let r=t.trim();return this.runInTransaction(()=>{let t=this.activeScan?.mode==="full"?this.activeScan.observationSequence:this.nextObservationSequence(),n=this.readStoredEntry(e);return null==n?{catalogRevision:this.readCatalogRevision(),changedThreadIds:[],removedThreadIds:[]}:this.upsertEntries([{...n,displayTitle:0===r.length?n.displayTitle:mq({cwd:"",name:r,threadId:e})}],t,!0,r.length>0)})}applyAuthoritativeRemoval(e){return this.runInTransaction(()=>{this.nextObservationSequence(),(null!=this.activeScan||null!=this.db.prepare("SELECT 1 FROM local_thread_catalog_scan_checkpoints WHERE host_id = ?").get(this.hostId))&&this.db.prepare(`
          INSERT INTO local_thread_catalog_scan_entries (host_id, thread_id, removed)
          VALUES (?, ?, 1)
          ON CONFLICT(host_id, thread_id) DO UPDATE SET removed = 1
        `).run(this.hostId,e);let t=this.db.prepare(`DELETE FROM local_thread_catalog
           WHERE host_id = ? AND thread_id = ?
           RETURNING missing_candidate`).get(this.hostId,e),r=t?.missing_candidate===0?[e]:[];return r.length>0&&this.bumpCatalogRevision(),{catalogRevision:this.readCatalogRevision(),changedThreadIds:[],removedThreadIds:r}})}pruneEntriesOlderThan(e){return this.runInTransaction(()=>0!==this.db.prepare(`DELETE FROM local_thread_catalog
           WHERE host_id = ? AND source_updated_at < ?`).run(this.hostId,e).changes&&(this.bumpCatalogRevision(),!0))}pruneHost(){if(null!=this.activeScan)throw Error("Cannot prune a thread catalog host during an active scan");this.runInTransaction(()=>{this.clearScanCheckpoint(),this.clearSeenEntries();let e=this.db.prepare(`SELECT EXISTS (
             SELECT 1 FROM local_thread_catalog
             WHERE host_id = ? AND missing_candidate = 0
           ) AS has_visible_entries`).get(this.hostId)?.has_visible_entries;this.db.prepare("DELETE FROM local_thread_catalog WHERE host_id = ?").run(this.hostId),this.db.prepare("DELETE FROM local_thread_catalog_sync_state WHERE host_id = ?").run(this.hostId),this.db.prepare("DELETE FROM local_thread_catalog_hosts WHERE host_id = ?").run(this.hostId),1===e&&this.bumpCatalogRevision()})}upsertEntries(e,t,r,n=!1){let i=[],s=this.db.prepare(mW),a=this.db.prepare("SELECT 1 FROM local_thread_catalog_scan_entries WHERE host_id = ? AND thread_id = ? AND removed = 1"),o=+!!n,l=r?this.db.prepare(`UPDATE local_thread_catalog
           SET observation_sequence = ?,
               pending_observed_title = CASE
                 WHEN ? = 1 THEN 1
                 ELSE pending_observed_title
               END
           WHERE host_id = ?
             AND thread_id = ?
             AND observation_sequence < ?`):null;for(let n of e){if(this.assertEntryHost(n),!r&&null!=a.get(this.hostId,n.threadId))continue;let e=+("chatgpt"===n.sourceKind&&void 0!==n.trialConversationType);null!=s.get(this.hostId,n.threadId,n.displayTitle,n.sourceCreatedAt,n.sourceUpdatedAt,n.sourceRecencyAt,"chatgpt"===n.sourceKind?null:n.cwd,n.sourceKind,"chatgpt"===n.sourceKind?null:n.sourceDetail,"chatgpt"===n.sourceKind?null:n.threadSource,"chatgpt"===n.sourceKind?null:n.modelProvider,"chatgpt"===n.sourceKind?null:n.gitBranch,"chatgpt"===n.sourceKind?n.projectId:null,"chatgpt"===n.sourceKind?n.conversationOrigin:null,"chatgpt"===n.sourceKind?n.trialConversationType??null:null,"chatgpt"===n.sourceKind?n.asyncStatus:null,t,o,o,o,e,o,o,e)?i.push(n.threadId):l?.run(t,o,this.hostId,n.threadId,t)}return i.length>0&&this.bumpCatalogRevision(),{catalogRevision:this.readCatalogRevision(),changedThreadIds:i,removedThreadIds:[]}}readStoredEntry(e){let t=this.db.prepare(`SELECT * FROM local_thread_catalog
         WHERE host_id = ? AND thread_id = ?`).get(this.hostId,e);return null==t?null:mV(t)}nextObservationSequence(){let e=this.db.prepare(`UPDATE local_thread_catalog_sync_state
         SET observation_sequence = observation_sequence + 1
         WHERE host_id = ?
         RETURNING observation_sequence`).get(this.hostId);if(null==e||!Number.isInteger(e.observation_sequence))throw Error("Local thread catalog sync state is unavailable");return e.observation_sequence}bumpCatalogRevision(){this.db.prepare(`UPDATE local_thread_catalog_metadata
         SET catalog_revision = catalog_revision + 1
         WHERE id = 1`).run()}readCatalogRevision(){let e=this.db.prepare(`SELECT catalog_revision FROM local_thread_catalog_metadata
         WHERE id = 1`).get();return"number"==typeof e?.catalog_revision?e.catalog_revision:0}mutateThreadCount(e,...t){return this.db.prepare(e).run(...t).changes??0}assertActiveScan(e){if(this.activeScan!==e)throw Error("Local thread catalog scan is not active")}assertEntryHost(e){if(e.hostId!==this.hostId)throw Error(`Thread catalog entry belongs to ${e.hostId}, expected ${this.hostId}`)}clearScanCheckpoint(){this.db.prepare("DELETE FROM local_thread_catalog_scan_checkpoints WHERE host_id = ?").run(this.hostId)}clearSeenEntries(){this.db.prepare("DELETE FROM local_thread_catalog_scan_entries WHERE host_id = ?").run(this.hostId)}runInTransaction(e){this.db.prepare("BEGIN IMMEDIATE").run();try{let t=e();return this.db.prepare("COMMIT").run(),t}catch(e){throw this.db.prepare("ROLLBACK").run(),e}}}function mH(e,t=""){if(e?.projectId===void 0&&e?.conversationOrigin===void 0)return null;let r=[],n=[];return void 0!==e.projectId&&(r.push(`AND ${t}project_id IS ?`),n.push(e.projectId)),"tpp"===e.conversationOrigin?r.push(`AND ${t}conversation_origin = 'tpp'`):"non-tpp"===e.conversationOrigin&&r.push(`AND ${t}conversation_origin IS NOT 'tpp'`),{clause:r.join("\n               "),params:n}}function m$(e){if(e.some(e=>0===e.length))throw Error("Thread catalog CWD prefixes must not be empty");let t=Array.from(new Set(e.map(mz))).sort((e,t)=>e.localeCompare(t));return t.filter((e,r)=>!t.slice(0,r).some(t=>e.startsWith(t)))}function mz(e){return e.endsWith("/")||e.endsWith("\\")?e:`${e}${e.includes("\\")?"\\":"/"}`}