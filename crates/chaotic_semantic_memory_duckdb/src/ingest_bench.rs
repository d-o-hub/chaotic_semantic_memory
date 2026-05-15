use crate::Analytics;
use crate::error::Result;
use crate::ingest_export::IngestReport;
use serde::Deserialize;
use std::path::Path;

impl Analytics {
    pub fn load_benchmarks_dir<P: AsRef<Path>>(&mut self, dir: P) -> Result<IngestReport> {
        let mut benchmarks_loaded = 0;

        if !dir.as_ref().exists() {
            return Ok(IngestReport::default());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "jsonl") {
                let file = std::fs::read_to_string(&path)?;
                for line in file.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    let res: serde_json::Value = serde_json::from_str(line)?;

                    let suite = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown");
                    let name = res["query_id"].as_str().unwrap_or("unknown");
                    let p50_us = res["latency_ms"].as_f64().unwrap_or(0.0) * 1000.0;
                    let extras = serde_json::to_string(&res)?;

                    self.conn.execute(
                        "INSERT INTO benchmarks (suite, name, run_at_us, p50_us, extras)
                         VALUES (?, ?, ?, ?, ?)",
                        duckdb::params![suite, name, 0, p50_us, extras],
                    )?;
                    benchmarks_loaded += 1;
                }
            }
        }

        Ok(IngestReport {
            benchmarks_loaded,
            ..Default::default()
        })
    }
}
