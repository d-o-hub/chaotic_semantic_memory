use chaotic_semantic_memory::{ChaoticSemanticFramework, HVec10240};
use std::collections::HashMap;

#[tokio::test]
async fn test_security_input_validation() {
    let framework = ChaoticSemanticFramework::builder()
        .without_persistence()
        .build()
        .await
        .unwrap();
    let invalid = "a".repeat(300);

    let res = framework
        .update_concept_vector(&invalid, HVec10240::random())
        .await;
    assert!(format!("{:?}", res).contains("InvalidInput"));

    let res = framework
        .update_concept_metadata(&invalid, HashMap::new())
        .await;
    assert!(format!("{:?}", res).contains("InvalidInput"));

    let res = framework.disassociate(&invalid, "v").await;
    assert!(format!("{:?}", res).contains("InvalidInput"));

    let res = framework.disassociate("v", &invalid).await;
    assert!(format!("{:?}", res).contains("InvalidInput"));

    let res = framework.clear_associations(&invalid).await;
    assert!(format!("{:?}", res).contains("InvalidInput"));

    let res = framework.concept_history(&invalid, 1).await;
    assert!(format!("{:?}", res).contains("InvalidInput"));
}
