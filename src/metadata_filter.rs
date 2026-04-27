//! Metadata filtering for similarity search queries.
//!
//! Provides predicate-based filtering during similarity search to support
//! RAG patterns like document scoping, per-session memory, and multi-tenant filtering.

use serde_json::Value;
use std::collections::HashMap;
use std::ops;

/// A predicate for filtering concepts by metadata during similarity search.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum MetadataFilter {
    /// Key equals value: `key == value`
    Eq(String, Value),
    /// Key is in set of values: `key in [values]`
    In(String, Vec<Value>),
    /// Key exists in metadata
    Exists(String),
    /// All filters must match
    And(Vec<MetadataFilter>),
    /// Any filter must match
    Or(Vec<MetadataFilter>),
    /// Negation of filter
    Not(Box<MetadataFilter>),
}

impl MetadataFilter {
    /// Create an equality filter: `key == value`.
    pub fn eq(key: impl Into<String>, value: impl Into<Value>) -> Self {
        Self::Eq(key.into(), value.into())
    }

    /// Create an "in" filter: `key in [values]`.
    pub fn in_(key: impl Into<String>, values: Vec<Value>) -> Self {
        Self::In(key.into(), values)
    }

    /// Create an existence filter: `key exists`.
    pub fn exists(key: impl Into<String>) -> Self {
        Self::Exists(key.into())
    }

    /// Combine filters with AND.
    pub fn and(filters: Vec<MetadataFilter>) -> Self {
        Self::And(filters)
    }

    /// Combine filters with OR.
    pub fn or(filters: Vec<MetadataFilter>) -> Self {
        Self::Or(filters)
    }

    /// Evaluate the filter against metadata.
    pub fn matches(&self, metadata: &HashMap<String, Value>) -> bool {
        match self {
            Self::Eq(key, value) => metadata.get(key) == Some(value),
            Self::In(key, values) => metadata.get(key).is_some_and(|v| values.contains(v)),
            Self::Exists(key) => metadata.contains_key(key),
            Self::And(filters) => filters.iter().all(|f| f.matches(metadata)),
            Self::Or(filters) => filters.iter().any(|f| f.matches(metadata)),
            Self::Not(filter) => !filter.matches(metadata),
        }
    }
}

impl ops::Not for MetadataFilter {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self::Not(Box::new(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_metadata(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    #[test]
    fn test_eq_match() {
        let filter = MetadataFilter::eq("type", "document");
        let metadata = make_metadata(&[("type", json!("document"))]);
        assert!(filter.matches(&metadata));
    }

    #[test]
    fn test_eq_no_match() {
        let filter = MetadataFilter::eq("type", "document");
        let metadata = make_metadata(&[("type", json!("image"))]);
        assert!(!filter.matches(&metadata));
    }

    #[test]
    fn test_eq_missing_key() {
        let filter = MetadataFilter::eq("type", "document");
        let metadata = make_metadata(&[]);
        assert!(!filter.matches(&metadata));
    }

    #[test]
    fn test_in_match() {
        let filter = MetadataFilter::in_("tag", vec![json!("rust"), json!("python")]);
        let metadata = make_metadata(&[("tag", json!("rust"))]);
        assert!(filter.matches(&metadata));
    }

    #[test]
    fn test_in_no_match() {
        let filter = MetadataFilter::in_("tag", vec![json!("rust"), json!("python")]);
        let metadata = make_metadata(&[("tag", json!("go"))]);
        assert!(!filter.matches(&metadata));
    }

    #[test]
    fn test_exists_present() {
        let filter = MetadataFilter::exists("title");
        let metadata = make_metadata(&[("title", json!("Hello"))]);
        assert!(filter.matches(&metadata));
    }

    #[test]
    fn test_exists_missing() {
        let filter = MetadataFilter::exists("title");
        let metadata = make_metadata(&[]);
        assert!(!filter.matches(&metadata));
    }

    #[test]
    fn test_and_all_match() {
        let filter = MetadataFilter::and(vec![
            MetadataFilter::eq("type", "document"),
            MetadataFilter::exists("title"),
        ]);
        let metadata = make_metadata(&[("type", json!("document")), ("title", json!("Test"))]);
        assert!(filter.matches(&metadata));
    }

    #[test]
    fn test_and_one_fails() {
        let filter = MetadataFilter::and(vec![
            MetadataFilter::eq("type", "document"),
            MetadataFilter::exists("author"),
        ]);
        let metadata = make_metadata(&[("type", json!("document")), ("title", json!("Test"))]);
        assert!(!filter.matches(&metadata));
    }

    #[test]
    fn test_or_any_match() {
        let filter = MetadataFilter::or(vec![
            MetadataFilter::eq("type", "document"),
            MetadataFilter::eq("type", "image"),
        ]);
        let metadata = make_metadata(&[("type", json!("image"))]);
        assert!(filter.matches(&metadata));
    }

    #[test]
    fn test_or_none_match() {
        let filter = MetadataFilter::or(vec![
            MetadataFilter::eq("type", "document"),
            MetadataFilter::eq("type", "image"),
        ]);
        let metadata = make_metadata(&[("type", json!("video"))]);
        assert!(!filter.matches(&metadata));
    }

    #[test]
    fn test_not() {
        let filter = !MetadataFilter::eq("private", true);
        let metadata = make_metadata(&[("private", json!(false))]);
        assert!(filter.matches(&metadata));
    }

    #[test]
    fn test_nested_complex() {
        // (type == "document" AND (tag == "rust" OR tag == "python")) AND NOT private
        let filter = MetadataFilter::and(vec![
            MetadataFilter::eq("type", "document"),
            MetadataFilter::or(vec![
                MetadataFilter::eq("tag", "rust"),
                MetadataFilter::eq("tag", "python"),
            ]),
            !MetadataFilter::eq("private", true),
        ]);
        let metadata = make_metadata(&[
            ("type", json!("document")),
            ("tag", json!("rust")),
            ("private", json!(false)),
        ]);
        assert!(filter.matches(&metadata));
    }
    #[test]
    fn test_and_empty() {
        let filter = MetadataFilter::and(vec![]);
        let metadata = make_metadata(&[("any", json!("value"))]);
        assert!(
            filter.matches(&metadata),
            "Empty AND should evaluate to true"
        );
    }

    #[test]
    fn test_or_empty() {
        let filter = MetadataFilter::or(vec![]);
        let metadata = make_metadata(&[("any", json!("value"))]);
        assert!(
            !filter.matches(&metadata),
            "Empty OR should evaluate to false"
        );
    }

    #[test]
    fn test_not_nested() {
        let filter = !MetadataFilter::and(vec![
            MetadataFilter::eq("type", "document"),
            MetadataFilter::exists("secure"),
        ]);

        // Matches the AND criteria -> NOT should be false
        let metadata_match = make_metadata(&[("type", json!("document")), ("secure", json!(true))]);
        assert!(!filter.matches(&metadata_match));

        // Fails the AND criteria -> NOT should be true
        let metadata_fail = make_metadata(&[("type", json!("image")), ("secure", json!(true))]);
        assert!(filter.matches(&metadata_fail));
    }

    #[test]
    fn test_nested_complex_negative_paths() {
        // (type == "document" AND (tag == "rust" OR tag == "python")) AND NOT private
        let filter = MetadataFilter::and(vec![
            MetadataFilter::eq("type", "document"),
            MetadataFilter::or(vec![
                MetadataFilter::eq("tag", "rust"),
                MetadataFilter::eq("tag", "python"),
            ]),
            !MetadataFilter::eq("private", true),
        ]);

        // Fail condition 1: type is not document
        let metadata_bad_type = make_metadata(&[
            ("type", json!("image")),
            ("tag", json!("rust")),
            ("private", json!(false)),
        ]);
        assert!(!filter.matches(&metadata_bad_type));

        // Fail condition 2: tag is neither rust nor python
        let metadata_bad_tag = make_metadata(&[
            ("type", json!("document")),
            ("tag", json!("go")),
            ("private", json!(false)),
        ]);
        assert!(!filter.matches(&metadata_bad_tag));

        // Fail condition 3: private is true
        let metadata_is_private = make_metadata(&[
            ("type", json!("document")),
            ("tag", json!("rust")),
            ("private", json!(true)),
        ]);
        assert!(!filter.matches(&metadata_is_private));
    }

    #[test]
    fn test_deep_nesting() {
        // NOT (OR (AND (k1==v1, k2==v2), AND (k3==v3, k4==v4)))
        let filter = !MetadataFilter::or(vec![
            MetadataFilter::and(vec![
                MetadataFilter::eq("k1", "v1"),
                MetadataFilter::eq("k2", "v2"),
            ]),
            MetadataFilter::and(vec![
                MetadataFilter::eq("k3", "v3"),
                MetadataFilter::eq("k4", "v4"),
            ]),
        ]);

        // Fails the OR (neither AND branch is fully satisfied) -> NOT makes it true
        let metadata_neither = make_metadata(&[
            ("k1", json!("v1")),
            ("k2", json!("wrong")),
            ("k3", json!("v3")),
            ("k4", json!("wrong")),
        ]);
        assert!(filter.matches(&metadata_neither));

        // Satisfies first AND branch -> OR is true -> NOT makes it false
        let metadata_first_branch = make_metadata(&[("k1", json!("v1")), ("k2", json!("v2"))]);
        assert!(!filter.matches(&metadata_first_branch));

        // Satisfies second AND branch -> OR is true -> NOT makes it false
        let metadata_second_branch = make_metadata(&[("k3", json!("v3")), ("k4", json!("v4"))]);
        assert!(!filter.matches(&metadata_second_branch));
    }
}
