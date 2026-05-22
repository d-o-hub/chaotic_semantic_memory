pub const SCHEMA_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS concepts (
    id            VARCHAR PRIMARY KEY,
    text          VARCHAR,
    namespace     VARCHAR,
    created_at_us BIGINT,
    updated_at_us BIGINT,
    expires_at_us BIGINT,
    metadata_json JSON
);

CREATE TABLE IF NOT EXISTS associations (
    src_id   VARCHAR,
    dst_id   VARCHAR,
    strength DOUBLE,
    PRIMARY KEY (src_id, dst_id)
);

CREATE TABLE IF NOT EXISTS concept_versions (
    id          VARCHAR,
    version     INTEGER,
    text        VARCHAR,
    created_us  BIGINT,
    PRIMARY KEY (id, version)
);

CREATE TABLE IF NOT EXISTS benchmarks (
    suite     VARCHAR,
    name      VARCHAR,
    commit    VARCHAR,
    run_at_us BIGINT,
    p50_us    DOUBLE,
    p95_us    DOUBLE,
    p99_us    DOUBLE,
    extras    JSON
);
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use duckdb::Connection;

    #[test]
    fn test_schema_init() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA_DDL).unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();

        assert!(tables.contains(&"concepts".to_string()));
        assert!(tables.contains(&"associations".to_string()));
        assert!(tables.contains(&"concept_versions".to_string()));
        assert!(tables.contains(&"benchmarks".to_string()));
    }
}
