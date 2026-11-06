(()=>{this.nextObservationSequence(),(null!=this.activeScan||null!=this.db.prepare("SELECT 1 FROM local_thread_catalog_scan_checkpoints WHERE host_id = ?").get(this.hostId))&&this.db.prepare(`
          INSERT INTO local_thread_catalog_scan_entries (host_id, thread_id, removed)
          VALUES (?, ?, 1)
          ON CONFLICT(host_id, thread_id) DO UPDATE SET removed = 1
        `).run(this.hostId,e);let t=this.db.prepare(`DELETE FROM local_thread_catalog
           WHERE host_id = ? AND thread_id = ?
           RETURNING missing_candidate`).get(this.hostId,e),r=t?.missing_candidate===0?[e]:[];return r.length>0&&this.bumpCatalogRevision(),{catalogRevision:this.readCatalogRevision(),changedThreadIds:[],removedThreadIds:r}})}pruneEntriesOlderThan(e){return this.runInTransaction(()=>0!==this.db.prepare(`DELETE FROM local_thread_catalog
           WHERE host_id = ? AND source_updated_at < ?`).run(this.hostId,e).changes&&(this.bumpCatalogRevision(),!0))}pruneHost(){if(null!=this.activeScan)throw Error("Cannot prune a thread cata