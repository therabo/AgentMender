un(this.hostId),this.db.prepare("DELETE FROM local_thread_catalog_hosts WHERE host_id = ?").run(this.hostId),1===e&&this.bumpCatalogRevision()})}upsertEntries(e,t,r,n=!1){let i=[],s=this.db.prepare(mW),a=this.db.prepare("SELECT 1 FROM local_thread_catalog_scan_entries WHERE host_id = ? AND thread_id = ? AND removed = 1"),o=+!!n,l=r?this.db.prepare(`UPDATE local_thread_catalog
           SET observation_sequence = ?,
               pending_observed_title = CASE
                 WHEN ? = 1 THEN 1
                 ELSE pending_observed_title
               END
           WHERE host_id = ?
       