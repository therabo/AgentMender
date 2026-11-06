let o={id:"20260902130908_create_live_visualization_suggestions",operations:[{kind:"exec",sql:`CREATE TABLE IF NOT EXISTS live_visualization_suggestions (
          account_id TEXT NOT NULL,
          user_id TEXT NOT NULL,
          host_id TEXT NOT NULL,
          thread_id TEXT NOT NULL,
          id TEXT NOT NULL,
          title TEXT NOT NULL,
          description TEXT NOT NULL,
          status TEXT NOT NULL CHECK (status IN ('pending', 'accepted', 'dismissed')),
          submission TEXT CHECK (submission IN ('submitted', 'accepted')),
          PRIMARY KEY (account_id, user_id, host_id, thread_id)
        ) WITHOUT ROWID`}]};function l(e,t,n){return{kind:"addColumnIfMissing",table:e,column:t,columnDefinition:n}}let c={id:"20260906120000_add_chatgpt_async_status_to_catalog",operations:[l("local_thread_catalog","chatgpt_async_status","INTEGER"),{kind:"exec",sql:`UPDATE local_thread_catalog_sync_state
          SET last_full_reconciled_at = NULL
          WHERE host_id IN (
            SELECT host_id FROM local_thread_catalog_hosts
            WHERE host_kind = 'chatgpt'
          );
          DELETE FROM local_thread_catalog_scan_checkpoints
          WHERE host_id IN (
            SELECT host_id FROM local_thread_catalog_hosts
            WHERE host_kind = 'chatgpt'
          )`}]},u={id:"20260914140000_add_local_thread_catalog_trial_conversation_type",operations:[l("local_thread_catalog","trial_conversation_type","TEXT"),{kind:"exec",sql:`UPDATE local_thread_catalog_sync_state
          SET last_full_reconciled_at = NULL
          WHERE host_id IN (
            SELECT host_id FROM local_thread_catalog_hosts
            WHERE host_kind = 'chatgpt'
          );
          DELETE FROM local_thread_catalog_scan_checkpoints
          WHERE host_id IN (
            SELECT host_id FROM local_thread_catalog_hosts
            WHERE host_kind = 'chatgpt'
          )`}]},d={id:"20260924180000_add_automation_auto_archive",operations:[l("automations","auto_archive","INTEGER NOT NULL DEFAULT 0")]},p={id:"20261002020000_add_automation_next_run_nominal_at",operations:[l("automations","next_run_nominal_at","INTEGER")]},h={id:"20261013185532_add_live_visualization_suggestion_turn",operations:[...o.operations,{kind:"addColumnIfMissing",table:"live_visualization_suggestions",column:"turn_id",columnDefinition:"TEXT"}]},m={id:"20261014044905_add_live_visualization_creation_turn",operations:[{kind:"addColumnIfMissing",table:"live_visualization_suggestions",column:"creation_turn_id",columnDefinition:"TEXT"}]},f={id:"20261028065000_create_active_chat_turns",operations:[{kind:"exec",sql:`CREATE TABLE IF NOT EXISTS active_chat_turns (
      profile TEXT NOT NULL,
      account_id TEXT NOT NULL,
      user_id TEXT NOT NULL,
      thread_id TEXT NOT NULL,
      launch_id TEXT NOT NULL,
      observed_at_ms INTEGER NOT NULL,
      PRIMARY KEY (profile, account_id, user_id, thread_id)
    ) WITHOUT ROWID`}]},g={id:"legacy_0001_create_inbox_items",operations:[{kind:"exec",sql:`CREATE TABLE IF NOT EXISTS inbox_items (
          id TEXT PRIMARY KEY,
          title TEXT
        )`}]},_={id:"legacy_0002_add_inbox_item_fields",operations:[l("inbox_items","description","TEXT"),l("inbox_items","thread_id","TEXT"),l("inbox_items","read_at","INTEGER"),l("inbox_items","created_at","INTEGER")]},E={id:"legacy_0003_repair_inbox_item_fields",operations:[l("inbox_items","description","TEXT"),l("inbox_items","thread_id","TEXT"),l("inbox_items","read_at","INTEGER"),l("inbox_items","created_at","INTEGER")]},S={id:"legacy_0007_create_automations",operations:[{kind:"exec",sql:`CREATE TABLE IF NOT EXISTS automations (
          id TEXT PRIMARY KEY,
          name TEXT NOT NULL,
          prompt TEXT NOT NULL,
          status TEXT NOT NULL DEFAULT 'ACTIVE',
          next_run_at INTEGER,
          last_run_at INTEGER,
          cwds TEXT NOT NULL DEFAULT '[]',
          rrule TEXT NOT NULL DEFAULT 'FREQ=HOURLY;INTERVAL=24;BYMINUTE=0',
          model TEXT,
          reasoning_effort TEXT,
          created_at INTEGER NOT NULL,
          updated_at INTEGER NOT NULL
        )`},l("automations","status","TEXT NOT NULL DEFAULT 'ACTIVE'"),l("automations","last_run_at","INTEGER"),l("automations","cwds","TEXT NOT NULL DEFAULT '[]'")]},T={id:"legacy_0008_add_automation_next_run_at",operations:[l("automations","next_run_at","INTEGER")]},y={id:"legacy_0009_create_automation_runs",operations:[{kind:"exec",sql:`CREATE TABLE IF NOT EXISTS automation_runs (
          thread_id TEXT PRIMARY KEY,
          automation_id TEXT NOT NULL,
          status TEXT NOT NULL,
          read_at INTEGER,
          thread_title TEXT,
          source_cwd TEXT,
          inbox_title TEXT,
          inbox_summary TEXT,
          created_at INTEGER NOT NULL,
          updated_at INTEGER NOT NULL
        )`}]},A={id:"legacy_0013_add_automation_rrule",operations:[l("automations","rrule","TEXT"),{kind:"run",sql:"UPDATE automations SET rrule = ? WHERE rrule IS NULL OR rrule = ''",parameters:["FREQ=HOURLY;INTERVAL=24;BYMINUTE=0"]}]},R={id:"legacy_0014_add_archived_automation_run_messages",operations:[l("automation_runs","archived_user_message","TEXT"),l("automation_runs","archived_assistant_message","TEXT")]},v={id:"legacy_0015_add_automation_run_read_at",operations:[l("automation_runs","read_at","INTEGER")]},w={id:"legacy_0016_add_automation_run_thread_title",operations:[l("automation_runs","thread_title","TEXT")]},b={id:"legacy_0017_add_automation_run_source_cwd",operations:[l("automation_runs","source_cwd","TEXT")]},I={id:"legacy_0018_add_automation_run_inbox_fields",operations:[l("automation_runs","inbox_title","TEXT"),l("automation_runs","inbox_summary","TEXT")]},C={id:"legacy_0019_add_automation_run_archived_reason",operations:[l("automation_runs","archived_reason","TEXT")]},O={id:"legacy_0020_add_automation_model_fields",operations:[l("automations","model","TEXT"),l("automations","reasoning_effort","TEXT")]},N={id:"legacy_0021_create_local_app_server_feature_enablement",operations:[{kind:"exec",sql:`CREATE TABLE IF NOT EXISTS local_app_server_feature_enablement (
          feature_name TEXT PRIMARY KEY,
          enabled INTEGER NOT NULL,
          updated_at INTEGER NOT NULL
        )`}]},M={id:"legacy_0022_create_local_thread_catalog",operations:[{kind:"exec",sql:`
          CREATE TABLE IF NOT EXISTS local_thread_catalog (
            host_id TEXT NOT NULL,
            thread_id TEXT NOT NULL,
            display_title TEXT NOT NULL,
            source_created_at REAL NOT NULL,
            source_updated_at REAL NOT NULL,
            cwd TEXT NOT NULL,
            source_kind TEXT NOT NULL,
            source_detail TEXT,
            model_provider TEXT NOT NULL,
            git_branch TEXT,
            observation_sequence INTEGER NOT NULL,
            missing_candidate INTEGER NOT NULL DEFAULT 0
              CHECK (missing_candidate IN (0, 1)),
            PRIMARY KEY (host_id, thread_id)
          );
          CREATE INDEX IF NOT EXISTS local_thread_catalog_updated_idx
            ON local_thread_catalog (
              host_id,
              source_updated_at DESC,
              source_created_at DESC,
              thread_id
            ) WHERE missing_candidate = 0;
          CREATE TABLE IF NOT EXISTS local_thread_catalog_sync_state (
            host_id TEXT PRIMARY KEY,
            watermark_updated_at REAL,
            initial_build_complete INTEGER NOT NULL DEFAULT 0,
            observation_sequence INTEGER NOT NULL DEFAULT 0
          );
          CREATE TABLE IF NOT EXISTS local_thread_catalog_hosts (
            host_id TEXT PRIMARY KEY,
            host_kind TEXT NOT NULL CHECK (
              host_kind IN ('local', 'ssh', 'wsl', 'remote-control')
            )
          );
          CREATE TABLE IF NOT EXISTS local_thread_catalog_metadata (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            catalog_revision INTEGER NOT NULL DEFAULT 0
          );
          INSERT OR IGNORE INTO local_thread_catalog_sync_state (host_id)
            VALUES ('local');
          INSERT OR IGNORE INTO local_thread_catalog_hosts (host_id, host_kind)
            VALUES ('local', 'local');
          INSERT OR IGNORE INTO local_thread_catalog_metadata (id) VALUES (1);
        `}]},P={id:"legacy_0023_add_automation_targets",operations:[l("automations","target_type","TEXT"),l("automations","project_id","TEXT")]},L={id:"legacy_0024_add_local_thread_catalog_last_full_reconciled_at",operations:[l("local_thread_catalog_sync_state","last_full_reconciled_at","INTEGER")]},D={id:"legacy_0025_add_local_thread_catalog_indexes",operations:[{kind:"exec",sql:`
          CREATE INDEX IF NOT EXISTS local_thread_catalog_created_idx
            ON local_thread_catalog (
              host_id,
              source_created_at DESC,
              source_updated_at DESC,
              thread_id
            ) WHERE missing_candidate = 0;
          CREATE INDEX IF NOT EXISTS local_thread_catalog_cwd_updated_idx
            ON local_thread_catalog (
              host_id,
              cwd,
              source_updated_at DESC,
              source_created_at DESC,
              thread_id
            ) WHERE missing_candidate = 0;
          CREATE INDEX IF NOT EXISTS local_thread_catalog_cwd_created_idx
            ON local_thread_catalog (
              host_id,
              cwd,
              source_created_at DESC,
              source_updated_at DESC,
              thread_id
            ) WHERE missing_candidate = 0;
        `}]},k={id:"legacy_0026_create_thread_timeline_ledger",operations:[{kind:"exec",sql:`
          CREATE TABLE IF NOT EXISTS thread_timeline_ledger (
            host_id TEXT NOT NULL,
            thread_id TEXT NOT NULL,
            sequence INTEGER NOT NULL,
            record_id TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            PRIMARY KEY (host_id, thread_id, sequence),
            UNIQUE (host_id, thread_id, record_id)
          ) WITHOUT ROWID;
        `}]},U={id:"legacy_0027_add_local_thread_catalog_thread_source",operations:[l("local_thread_catalog","thread_source","TEXT"),{kind:"run",sql:`UPDATE local_thread_catalog_sync_state
          SET last_full_reconciled_at = NULL`,parameters:[]}]},x={id:"legacy_0028_schedule_full_catalog_backfill_for_title_projection",operations:[{kind:"run",sql:`UPDATE local_thread_catalog_sync_state
          SET last_full_reconciled_at = NULL`,parameters:[]}]},V={id:"legacy_0029_add_local_thread_catalog_source_recency",operations:[l("local_thread_catalog","source_recency_at","REAL NOT NULL DEFAULT 0"),{kind:"exec",sql:`
          UPDATE local_thread_catalog
          SET source_recency_at = source_updated_at
          WHERE source_recency_at = 0;
          DROP INDEX IF EXISTS local_thread_catalog_updated_idx;
          DROP INDEX IF EXISTS local_thread_catalog_cwd_updated_idx;
          CREATE INDEX IF NOT EXISTS local_thread_catalog_updated_idx
            ON local_thread_catalog (
              host_id,
              source_recency_at DESC,
              source_created_at DESC,
              thread_id
            ) WHERE missing_candidate = 0;
          CREATE INDEX IF NOT EXISTS local_thread_catalog_cwd_updated_idx
            ON local_thread_catalog (
              host_id,
              cwd,
              source_recency_at DESC,
              source_created_at DESC,
              thread_id
            ) WHERE missing_candidate = 0;
          UPDATE local_thread_catalog_sync_state
          SET last_full_reconciled_at = NULL;
        `}]},B={id:"legacy_0030_add_local_thread_catalog_pending_observed_title",operations:[l("local_thread_catalog","pending_observed_title","INTEGER NOT NULL DEFAULT 0")]},F={id:"legacy_0031_add_local_thread_catalog_thread_lookup_index",operations:[{kind:"exec",sql:`
          CREATE INDEX IF NOT EXISTS local_thread_catalog_thread_lookup_idx
            ON local_thread_catalog
              (thread_id, source_recency_at DESC, source_created_at DESC, host_id)
            WHERE missing_candidate = 0;
        `}]},G={id:"legacy_0032_expand_local_thread_catalog_for_projects",operations:[l("local_thread_catalog","project_id","TEXT"),l("local_thread_catalog","conversation_origin","TEXT"),{kind:"exec",sql:`
          CREATE TABLE local_thread_catalog_new (
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
            conversation_origin TEXT,
            PRIMARY KEY (host_id, thread_id)
          );
          INSERT INTO local_thread_catalog_new (
            host_id, thread_id, display_title, source_created_at,
            source_updated_at, cwd, source_kind, source_detail,
            model_provider, git_branch, observation_sequence,
            missing_candidate, thread_source, source_recency_at,
            pending_observed_title, project_id, conversation_origin
          )
          SELECT
            host_id, thread_id, display_title, source_created_at,
            source_updated_at, cwd, source_kind, source_detail,
            model_provider, git_branch, observation_sequence,
            missing_candidate, thread_source, source_recency_at,
            pending_observed_title, project_id, conversation_origin
          FROM local_thread_catalog;
          DROP TABLE local_thread_catalog;
          ALTER TABLE local_thread_catalog_new RENAME TO local_thread_catalog;
          CREATE INDEX local_thread_catalog_updated_idx
            ON local_thread_catalog (
              host_id, source_recency_at DESC, source_created_at DESC, thread_id
            ) WHERE missing_candidate = 0;
          CREATE INDEX local_thread_catalog_created_idx
            ON local_thread_catalog (
              host_id, source_created_at DESC, source_updated_at DESC, thread_id
            ) WHERE missing_candidate = 0;
          CREATE INDEX local_thread_catalog_cwd_updated_idx
            ON local_thread_catalog (
              host_id, cwd, source_recency_at DESC, source_created_at DESC,
              thread_id
            ) WHERE missing_candidate = 0;
          CREATE INDEX local_thread_catalog_cwd_created_idx
            ON local_thread_catalog (
              host_id, cwd, source_created_at DESC, source_updated_at DESC,
              thread_id
            ) WHERE missing_candidate = 0;
          CREATE INDEX local_thread_catalog_thread_lookup_idx
            ON local_thread_catalog (
              thread_id, source_recency_at DESC, source_created_at DESC, host_id
            ) WHERE missing_candidate = 0;
          CREATE INDEX local_thread_catalog_project_updated_idx
            ON local_thread_catalog (
              host_id, project_id, source_recency_at DESC,
              source_created_at DESC, thread_id
            ) WHERE missing_candidate = 0;
          CREATE INDEX local_thread_catalog_origin_updated_idx
            ON local_thread_catalog (
              host_id, conversation_origin, source_recency_at DESC,
              source_created_at DESC, thread_id
            ) WHERE missing_candidate = 0;
          CREATE TABLE local_thread_catalog_hosts_new (
            host_id TEXT PRIMARY KEY,
            host_kind TEXT NOT NULL CHECK (
              host_kind IN ('local', 'ssh', 'wsl', 'remote-control', 'chatgpt')
            )
          );
          INSERT INTO local_thread_catalog_hosts_new (host_id, host_kind)
            SELECT host_id, host_kind FROM local_thread_catalog_hosts;
          DROP TABLE local_thread_catalog_hosts;
          ALTER TABLE local_thread_catalog_hosts_new
            RENAME TO local_thread_catalog_hosts;
        `}]},j={id:"legacy_0033_expand_automations",operations:[{kind:"exec",sql:`CREATE TABLE IF NOT EXISTS automations (
          id TEXT PRIMARY KEY,
          name TEXT NOT NULL,
          prompt TEXT NOT NULL,
          status TEXT NOT NULL DEFAULT 'ACTIVE',
          next_run_at INTEGER,
          last_run_at INTEGER,
          cwds TEXT NOT NULL DEFAULT '[]',
          target_type TEXT,
          project_id TEXT,
          rrule TEXT NOT NULL DEFAULT 'FREQ=HOURLY;INTERVAL=24;BYMINUTE=0',
          model TEXT,
          reasoning_effort TEXT,
          created_at INTEGER NOT NULL,
          updated_at INTEGER NOT NULL
        )`},l("automations","kind","TEXT NOT NULL DEFAULT 'cron'"),l("automations","target_thread_id","TEXT"),l("automations","execution_environment","TEXT"),l("automations","local_environment_config_path","TEXT"),l("automations","plugin_template_id","TEXT"),l("automations","notification_policy","TEXT"),l("automations","account_id","TEXT"),l("automations","user_id","TEXT"),l("automations","installation_id","TEXT"),l("automations","legacy_automation_id","TEXT"),{kind:"exec",sql:`CREATE INDEX IF NOT EXISTS automations_owner_idx
        ON automations (account_id, user_id, installation_id)`}]},q={id:"legacy_0034_create_local_thread_catalog_scan_tables",operations:[{kind:"exec",sql:`CREATE TABLE IF NOT EXISTS local_thread_catalog_scan_checkpoints (
          host_id TEXT PRIMARY KEY,
          checkpoint TEXT NOT NULL,
          failed_at INTEGER
        );
        CREATE TABLE IF NOT EXISTS local_thread_catalog_scan_entries (
          host_id TEXT NOT NULL,
          thread_id TEXT NOT NULL,
          removed INTEGER NOT NULL DEFAULT 0,
          PRIMARY KEY (host_id, thread_id)
        ) WITHOUT ROWID`}]},H=[{migration:g,predecessorId:null},{migration:_,predecessorId:g.id},{migration:E,predecessorId:_.id},{migration:S,predecessorId:E.id},{migration:T,predecessorId:S.id},{migration:y,predecessorId:T.id},{migration:A,predecessorId:y.id},{migration:R,predecessorId:A.id},{migration:v,predecessorId:R.id},{migration:w,predecessorId:v.id},{migration:b,predecessorId:w.id},{migration:I,predecessorId:b.id},{migration:C,predecessorId:I.id},{migration:O,predecessorId:C.id},{migration:N,predecessorId:O.id},{migration:M,predecessorId:N.id},{migration:P,predecessorId:M.id},{migration:L,predecessorId:P.id},{migration:D,predecessorId:L.id},{migration:k,predecessorId:D.id},{migration:U,predecessorId:k.id},{migration:x,predecessorId:U.id},{migration:V,predecessorId:x.id},{migration:B,predecessorId:V.id},{migration:F,predecessorId:B.id},{migration:G,predecessorId:F.id},{migration:j,predecessorId:G.id},{migration:q,predecessorId:j.id},{migration:o,predecessorId:q.id},{migration:u,predecessorId:o.id},{migration:d,predecessorId:u.id},{migration:c,predecessorId:d.id},{migration:p,predecessorId:c.id},{migration:h,predecessorId:p.id},{migration:m,predecessorId:h.id},{migration:f,predecessorId:m.id}],$=new Set,Y=null;for(let e of H){let{id:t}=e.migration;if($.has(t))throw Error(`Duplicate SQLite migration ID: ${t}`);if(e.predecessorId!==Y)throw Error(`SQLite migration ${t} must follow ${Y??"the manifest root"}; found predecessor ${e.predecessorId??"the manifest root"}`);$.add(t),Y=t}let K=H.map(e=>{var t;return{...e,checksum:(t=e.migration,(0,s.createHash)("sha256").update(JSON.stringify({checksumVersion:1,operations:t.operations})).digest("hex"))}})