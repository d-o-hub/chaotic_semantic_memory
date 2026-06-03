use chaotic_semantic_memory::prelude::*;
use rand::RngExt;
use std::sync::Arc;
use tokio::task::JoinHandle;

#[tokio::test]
async fn stress_test_high_concurrency_in_memory() {
    let framework = Arc::new(
        ChaoticSemanticFramework::builder()
            .without_persistence()
            .build()
            .await
            .unwrap(),
    );

    let num_tasks = 100;
    let ops_per_task = 50;
    let mut handles: Vec<JoinHandle<()>> = Vec::with_capacity(num_tasks);

    for t in 0..num_tasks {
        let fw = Arc::clone(&framework);
        handles.push(tokio::spawn(async move {
            for i in 0..ops_per_task {
                let op = {
                    let mut rng = rand::rng();
                    rng.random_range(0..5)
                };
                let id = format!("task-{t}-op-{i}");

                match op {
                    0 => {
                        // Inject
                        fw.inject_concept(&id, HVec10240::random()).await.unwrap();
                    }
                    1 => {
                        // Probe
                        let _ = fw.probe(HVec10240::random(), 5).await.unwrap();
                    }
                    2 => {
                        // Associate
                        let to_id = {
                            let mut rng = rand::rng();
                            format!("task-{t}-op-{}", rng.random_range(0..ops_per_task))
                        };
                        let _ = fw.associate(&id, &to_id, 0.5).await;
                    }
                    3 => {
                        // Delete
                        let _ = fw.delete_concept(&id).await;
                    }
                    _ => {
                        // Stats
                        let _ = fw.stats().await.unwrap();
                    }
                }
            }
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let stats = framework.stats().await.unwrap();
    println!("In-memory stress test complete. Final concept count: {}", stats.concept_count);
}

#[cfg(feature = "persistence")]
#[tokio::test]
async fn stress_test_high_concurrency_with_persistence() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("stress.db").to_str().unwrap().to_string();

    let framework = Arc::new(
        ChaoticSemanticFramework::builder()
            .with_local_db(&db_path)
            .build()
            .await
            .unwrap(),
    );

    let num_tasks = 100;
    let ops_per_task = 20; // Fewer ops because persistence is slower
    let mut handles: Vec<JoinHandle<()>> = Vec::with_capacity(num_tasks);

    for t in 0..num_tasks {
        let fw = Arc::clone(&framework);
        handles.push(tokio::spawn(async move {
            for i in 0..ops_per_task {
                let op = {
                    let mut rng = rand::rng();
                    rng.random_range(0..5)
                };
                let id = format!("persist-task-{t}-op-{i}");

                match op {
                    0 => {
                        fw.inject_concept(&id, HVec10240::random()).await.unwrap();
                    }
                    1 => {
                        let _ = fw.probe(HVec10240::random(), 5).await.unwrap();
                    }
                    2 => {
                        let to_id = {
                            let mut rng = rand::rng();
                            format!("persist-task-{t}-op-{}", rng.random_range(0..ops_per_task))
                        };
                        let _ = fw.associate(&id, &to_id, 0.5).await;
                    }
                    3 => {
                        let _ = fw.delete_concept(&id).await;
                    }
                    _ => {
                        let _ = fw.stats().await.unwrap();
                    }
                }
            }
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let stats = framework.stats().await.unwrap();
    println!("Persistence stress test complete. Final concept count: {}, DB size: {:?}",
             stats.concept_count, stats.db_size_bytes);
}
