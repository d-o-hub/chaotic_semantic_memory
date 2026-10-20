#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use proptest::prelude::*;
use std::collections::HashSet;

proptest! {
    #[test]
    fn search_is_unique_bounded_and_consistent_with_rebuild(
        term_ids in prop::collection::vec(0u8..16, 1..80),
        top_k in 1usize..80,
    ) {
        let mut index = Bm25Index::new();
        for (doc_index, term_id) in term_ids.iter().enumerate() {
            let terms = ["common".to_string(), format!("term-{term_id}")];
            index.add_document(&format!("doc-{doc_index}"), &terms);
        }

        let cold = index.search(&["common"], top_k);
        let warm = index.search(&["common"], top_k);
        prop_assert_eq!(&cold, &warm);
        prop_assert!(cold.len() <= top_k);
        let unique_ids: HashSet<&str> = cold.iter().map(|(id, _)| id.as_str()).collect();
        prop_assert_eq!(unique_ids.len(), cold.len());

        let mut rebuilt = Bm25Index::new();
        for (doc_index, term_id) in term_ids.iter().enumerate() {
            let terms = ["common".to_string(), format!("term-{term_id}")];
            rebuilt.add_document(&format!("doc-{doc_index}"), &terms);
        }
        prop_assert_eq!(&cold, &rebuilt.search(&["common"], top_k));

        index.add_document("late-added", &["common", "late"]);
        rebuilt.add_document("late-added", &["common", "late"]);
        let after_mutation = index.search(&["common"], top_k);
        prop_assert_eq!(
            &after_mutation,
            &rebuilt.search(&["common"], top_k)
        );
        prop_assert!(after_mutation.len() <= top_k);
        let unique_after_mutation: HashSet<&str> =
            after_mutation.iter().map(|(id, _)| id.as_str()).collect();
        prop_assert_eq!(unique_after_mutation.len(), after_mutation.len());
    }
}
