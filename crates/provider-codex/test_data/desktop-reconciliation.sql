CREATE TABLE "local_thread_catalog" (
            host_id TEXT NOT NULL,
            thread_id TEXT NOT NULL,
            display_title TEXT NOT NULL,
            source_created_at REAL NOT NULL,
            source_updated_at REAL NOT NULL,
            cwd TEXT,
            source_kind TEXT NOT NULL,
            source_detail TEXT,
            model_provider TEXT,
            git_branch TEXT,
            observation_sequence INTEGER NOT NULL,
            missing_candidate INTEGER NOT NULL DEFAULT 0
              CHECK (missing_candidate IN (0, 1)),
            thread_source TEXT,
            source_recency_at REAL NOT NULL DEFAULT 0,
            pending_observed_title INTEGER NOT NULL DEFAULT 0,
            project_id TEXT,
            conversation_origin TEXT, trial_conversation_type TEXT, chatgpt_async_status INTEGER,
            PRIMARY KEY (host_id, thread_id)
          );

CREATE TABLE local_thread_catalog_sync_state (
            host_id TEXT PRIMARY KEY,
            watermark_updated_at REAL,
            initial_build_complete INTEGER NOT NULL DEFAULT 0,
            observation_sequence INTEGER NOT NULL DEFAULT 0
          , last_full_reconciled_at INTEGER);

CREATE TABLE local_thread_catalog_scan_entries (
    host_id TEXT NOT NULL, thread_id TEXT NOT NULL, removed INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (host_id, thread_id)
) WITHOUT ROWID;
CREATE TABLE local_thread_catalog_metadata (
    id INTEGER PRIMARY KEY, catalog_revision INTEGER NOT NULL
);
CREATE TABLE local_thread_catalog_scan_checkpoints (
    host_id TEXT PRIMARY KEY, checkpoint TEXT NOT NULL
);
CREATE TABLE local_thread_catalog_hosts (
    host_id TEXT PRIMARY KEY, host_kind TEXT NOT NULL
);

INSERT INTO local_thread_catalog_metadata VALUES (1, 25);
INSERT INTO local_thread_catalog_hosts VALUES ('local', 'local'), ('remote', 'chatgpt');
INSERT INTO local_thread_catalog_scan_checkpoints VALUES ('local', 'in-flight'), ('remote', 'remote checkpoint');
