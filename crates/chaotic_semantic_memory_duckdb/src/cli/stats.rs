use crate::Analytics;
use crate::cli::CliOutputFormat;
use crate::error::Result;

pub async fn run(analytics: &Analytics, format: &CliOutputFormat) -> Result<()> {
    let concept_summary = analytics.concept_summary()?;
    let benchmark_summary = analytics.benchmark_summary()?;

    if matches!(format, CliOutputFormat::Json) {
        let out = serde_json::json!({
            "concepts": concept_summary,
            "benchmarks": benchmark_summary,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("Concept Summary");
        println!("---------------");
        println!("Total Concepts:     {}", concept_summary.total_concepts);
        println!("Total Associations: {}", concept_summary.total_associations);
        println!(
            "Namespaces:         {}",
            concept_summary.namespaces.join(", ")
        );
        println!();
        println!("Benchmark Summary");
        println!("-----------------");
        println!("Total Runs:  {}", benchmark_summary.total_runs);
        println!("Avg P50 (us): {:.2}", benchmark_summary.avg_p50_us);
        println!("Suites:      {}", benchmark_summary.suites.join(", "));
    }

    Ok(())
}
