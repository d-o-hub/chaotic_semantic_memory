#![cfg(feature = "persistence")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Integration tests for bridge persistence (canonical concept graph storage).

use chaotic_semantic_memory::persistence::Persistence;
use chaotic_semantic_memory::semantic_bridge::{CanonicalConcept, ConceptGraph};
use tempfile::NamedTempFile;

async fn temp_persistence() -> (Persistence, NamedTempFile) {
    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_str().unwrap().to_owned();
    let p = Persistence::new_local(&path).await.unwrap();
    (p, temp)
}

#[tokio::test]
async fn round_trip_single_concept() {
    let (p, _tmp) = temp_persistence().await;

    let concept = CanonicalConcept::new("animal")
        .with_label("cat")
        .with_label("feline")
        .with_related("pet");

    p.save_canonical_concept("ns1", &concept).await.unwrap();
    let loaded = p
        .load_canonical_concept("ns1", "animal")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(loaded.id, "animal");
    assert_eq!(loaded.version, 1);
    assert_eq!(loaded.labels, vec!["cat", "feline"]);
    assert_eq!(loaded.related, vec!["pet"]);
}

#[tokio::test]
async fn update_concept_overwrites() {
    let (p, _tmp) = temp_persistence().await;

    let v1 = CanonicalConcept::new("evolving").with_label("old");
    p.save_canonical_concept("ns", &v1).await.unwrap();

    // Overwrite with new version/labels
    let v2 = CanonicalConcept {
        id: "evolving".into(),
        version: 2,
        labels: vec!["new".into(), "updated".into()],
        related: vec!["dep".into()],
    };
    p.save_canonical_concept("ns", &v2).await.unwrap();

    let loaded = p
        .load_canonical_concept("ns", "evolving")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.version, 2);
    assert_eq!(loaded.labels, vec!["new", "updated"]);
    assert_eq!(loaded.related, vec!["dep"]);
}

#[tokio::test]
async fn delete_concept_removes_it() {
    let (p, _tmp) = temp_persistence().await;

    let concept = CanonicalConcept::new("ephemeral");
    p.save_canonical_concept("ns", &concept).await.unwrap();

    p.delete_canonical_concept("ns", "ephemeral").await.unwrap();

    let loaded = p.load_canonical_concept("ns", "ephemeral").await.unwrap();
    assert!(loaded.is_none());
}

#[tokio::test]
async fn load_all_concepts_for_namespace() {
    let (p, _tmp) = temp_persistence().await;

    for i in 0..5 {
        let c = CanonicalConcept::new(format!("c{i}")).with_label(format!("label{i}"));
        p.save_canonical_concept("bulk", &c).await.unwrap();
    }

    let all = p.load_all_canonical_concepts("bulk").await.unwrap();
    assert_eq!(all.len(), 5);

    let ids: Vec<&str> = all.iter().map(|c| c.id.as_str()).collect();
    for i in 0..5 {
        assert!(ids.contains(&format!("c{i}").as_str()));
    }
}

#[tokio::test]
async fn save_and_load_concept_graph() {
    let (p, _tmp) = temp_persistence().await;

    let mut graph = ConceptGraph::new();
    graph.add_concept(
        CanonicalConcept::new("alpha")
            .with_label("a")
            .with_related("beta"),
    );
    graph.add_concept(
        CanonicalConcept::new("beta")
            .with_label("b")
            .with_related("alpha"),
    );

    p.save_concept_graph("g", &graph).await.unwrap();

    let loaded = p.load_concept_graph("g").await.unwrap();
    assert_eq!(loaded.concept_count(), 2);
    assert_eq!(loaded.label_count(), 2);
}

#[tokio::test]
async fn namespace_isolation() {
    let (p, _tmp) = temp_persistence().await;

    let c1 = CanonicalConcept::new("shared-id").with_label("in-ns1");
    let c2 = CanonicalConcept::new("shared-id").with_label("in-ns2");

    p.save_canonical_concept("ns1", &c1).await.unwrap();
    p.save_canonical_concept("ns2", &c2).await.unwrap();

    let from_ns1 = p
        .load_canonical_concept("ns1", "shared-id")
        .await
        .unwrap()
        .unwrap();
    let from_ns2 = p
        .load_canonical_concept("ns2", "shared-id")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(from_ns1.labels, vec!["in-ns1"]);
    assert_eq!(from_ns2.labels, vec!["in-ns2"]);

    // Delete from ns1 does not affect ns2
    p.delete_canonical_concept("ns1", "shared-id")
        .await
        .unwrap();
    assert!(
        p.load_canonical_concept("ns1", "shared-id")
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        p.load_canonical_concept("ns2", "shared-id")
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn load_nonexistent_returns_none() {
    let (p, _tmp) = temp_persistence().await;

    let result = p
        .load_canonical_concept("ns", "does-not-exist")
        .await
        .unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn labels_with_special_characters() {
    let (p, _tmp) = temp_persistence().await;

    let concept = CanonicalConcept::new("unicode")
        .with_label("日本語")
        .with_label("émojis 🎉")
        .with_label("")
        .with_label("has \"quotes\" and \\slashes");

    p.save_canonical_concept("ns", &concept).await.unwrap();

    let loaded = p
        .load_canonical_concept("ns", "unicode")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        loaded.labels,
        vec!["日本語", "émojis 🎉", "", "has \"quotes\" and \\slashes",]
    );
}

#[tokio::test]
async fn related_ids_round_trip() {
    let (p, _tmp) = temp_persistence().await;

    let concept = CanonicalConcept::new("hub")
        .with_related("spoke-1")
        .with_related("spoke-2")
        .with_related("spoke-3");

    p.save_canonical_concept("ns", &concept).await.unwrap();

    let loaded = p
        .load_canonical_concept("ns", "hub")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.related, vec!["spoke-1", "spoke-2", "spoke-3"]);
}
