#[cfg(test)]
mod edge_case_tests {
    use chaotic_semantic_memory::retrieval::bm25::Bm25Index;

    #[test]
    fn test_all_oov_query() {
        let mut index = Bm25Index::new();
        index.add_document("doc1", &["hello", "world"]);

        // "rust" and "fast" are OOV
        let results = index.search(&["rust", "fast"], 10);
        assert!(
            results.is_empty(),
            "Query with all OOV terms should return empty results"
        );
    }

    #[test]
    fn test_mixed_oov_query() {
        let mut index = Bm25Index::new();
        index.add_document("doc1", &["hello", "world"]);
        index.add_document("doc2", &["rust", "is", "fast"]);

        // "hello" is in index, "c++" is OOV
        let results = index.search(&["hello", "c++"], 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "doc1");
    }

    #[test]
    fn test_empty_index_query() {
        let index = Bm25Index::new();
        let results = index.search(&["hello"], 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_query_with_duplicates_including_oov() {
        let mut index = Bm25Index::new();
        index.add_document("doc1", &["hello", "world"]);

        // "hello" (index), "rust" (OOV), "hello" (duplicate), "rust" (duplicate)
        let results = index.search(&["hello", "rust", "hello", "rust"], 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "doc1");
    }

    #[test]
    fn test_document_with_no_terms_score() {
        let mut index = Bm25Index::new();
        index.add_document("doc1", &["hello"]);
        index.add_document("empty", &[] as &[&str]);

        let results = index.search(&["hello"], 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "doc1");
    }
}
