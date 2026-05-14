use chaotic_semantic_memory::{ChaoticSemanticFramework, HVec10240};
use std::collections::HashMap;

#[tokio::test]
async fn test_api_validates_concept_ids() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let invalid_id = "a".repeat(300); // Exceeds 256 bytes

    // Test update_concept_vector
    let res = framework
        .update_concept_vector(invalid_id.as_str(), HVec10240::random())
        .await;
    assert!(res.is_err());
    assert!(format!("{:?}", res).contains("InvalidInput"));

    // Test update_concept_metadata
    let res = framework
        .update_concept_metadata(invalid_id.as_str(), HashMap::new())
        .await;
    assert!(res.is_err());
    assert!(format!("{:?}", res).contains("InvalidInput"));

    // Test disassociate
    let res = framework
        .disassociate(invalid_id.as_str(), "valid-id")
        .await;
    assert!(res.is_err());
    assert!(format!("{:?}", res).contains("InvalidInput"));

    let res = framework
        .disassociate("valid-id", invalid_id.as_str())
        .await;
    assert!(res.is_err());
    assert!(format!("{:?}", res).contains("InvalidInput"));

    // Test clear_associations
    let res = framework.clear_associations(invalid_id.as_str()).await;
    assert!(res.is_err());
    assert!(format!("{:?}", res).contains("InvalidInput"));

    // Test concept_history
    let res = framework.concept_history(invalid_id.as_str(), 10).await;
    assert!(res.is_err());
    assert!(format!("{:?}", res).contains("InvalidInput"));
}

#[tokio::test]
async fn test_api_rejects_control_characters() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();

    let invalid_id = "test\nid"; // Contains newline

    let res = framework
        .update_concept_vector(invalid_id, HVec10240::random())
        .await;
    assert!(res.is_err());
    assert!(format!("{:?}", res).contains("InvalidInput"));
}
