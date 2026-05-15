use crate::Analytics;
use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ConceptSummary {
    pub total_concepts: usize,
    pub total_associations: usize,
    pub namespaces: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BenchmarkSummary {
    pub total_runs: usize,
    pub avg_p50_us: f64,
    pub suites: Vec<String>,
}

impl Analytics {
    pub fn concept_summary(&self) -> Result<ConceptSummary> {
        let total_concepts = self
            .conn
            .query_row("SELECT count(*) FROM concepts", [], |row| {
                row.get::<_, usize>(0)
            })?;

        let total_associations =
            self.conn
                .query_row("SELECT count(*) FROM associations", [], |row| {
                    row.get::<_, usize>(0)
                })?;

        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT namespace FROM concepts")?;
        let namespaces = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(ConceptSummary {
            total_concepts,
            total_associations,
            namespaces,
        })
    }

    pub fn benchmark_summary(&self) -> Result<BenchmarkSummary> {
        let total_runs = self
            .conn
            .query_row("SELECT count(*) FROM benchmarks", [], |row| {
                row.get::<_, usize>(0)
            })?;

        let avg_p50_us = self.conn.query_row(
            "SELECT COALESCE(avg(p50_us), 0.0) FROM benchmarks",
            [],
            |row| row.get::<_, f64>(0),
        )?;

        let mut stmt = self.conn.prepare("SELECT DISTINCT suite FROM benchmarks")?;
        let suites = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(BenchmarkSummary {
            total_runs,
            avg_p50_us,
            suites,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::SCHEMA_DDL;
    use duckdb::Connection;

    #[test]
    fn test_stats_minimal() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA_DDL).unwrap();
        let analytics = Analytics { conn };

        let c_summary = analytics.concept_summary().unwrap();
        assert_eq!(c_summary.total_concepts, 0);

        let b_summary = analytics.benchmark_summary().unwrap();
        assert_eq!(b_summary.total_runs, 0);
    }
}
