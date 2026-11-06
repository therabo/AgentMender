CREATE TABLE IF NOT EXISTS codex_schema_migrations (
      id TEXT PRIMARY KEY,
      predecessor_id TEXT,
      checksum TEXT NOT NULL,
      applied_at INTEGER NOT NULL,
      app_version TEXT,
      build_flavor TEXT,
      build_commit TEXT
    )
  ;
