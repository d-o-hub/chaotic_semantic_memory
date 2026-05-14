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
    assert!(
        framework
            .update_concept_vector(&invalid, HVec10240::random())
            .await
            .is_err()
    );
    assert!(
        framework
            .update_concept_metadata(&invalid, HashMap::new())
            .await
            .is_err()
    );
    assert!(framework.disassociate(&invalid, "v").await.is_err());
    assert!(framework.disassociate("v", &invalid).await.is_err());
    assert!(framework.clear_associations(&invalid).await.is_err());
    assert!(framework.concept_history(&invalid, 1).await.is_err());
}
