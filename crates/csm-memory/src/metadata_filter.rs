//! Metadata filtering for similarity search queries.
//!
//! Provides predicate-based filtering during similarity search to support
//! RAG patterns like document scoping, per-session memory, and multi-tenant filtering.

use serde_json::Value;
use std::collections::HashMap;
use std::ops;

/// Maximum recursion depth for metadata filters to prevent stack overflow (DoS).
pub(crate) const MAX_FILTER_DEPTH: usize = 32;

/// A predicate for filtering concepts by metadata during similarity search.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
    pub fn eq(key: impl Into<String>, value: Value) -> Self {
        Self::Eq(key.into(), value)
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
    pub const fn and(filters: Vec<MetadataFilter>) -> Self {
        Self::And(filters)
    }

    /// Combine filters with OR.
    pub const fn or(filters: Vec<MetadataFilter>) -> Self {
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

    /// Compute the depth of the filter tree.
    pub(crate) fn depth(&self) -> usize {
        let (mut max, mut stack) = (0, vec![(self, 1)]);
        while let Some((f, d)) = stack.pop() {
            max = max.max(d);
            if d > MAX_FILTER_DEPTH {
                return d;
            }
            match f {
                Self::And(v) | Self::Or(v) => v.iter().for_each(|i| stack.push((i, d + 1))),
                Self::Not(i) => stack.push((i, d + 1)),
                _ => {}
            }
        }
        max
    }

    /// Validate filter parameters.
    pub fn validate(&self) -> csm_core::error::Result<()> {
        let depth = self.depth();
        if depth > MAX_FILTER_DEPTH {
            return Err(csm_core::error::MemoryError::InvalidInput {
                field: "filter".to_string(),
                reason: format!(
                    "metadata filter depth exceeds maximum allowed {MAX_FILTER_DEPTH} (got {depth})"
                ),
            });
        }
        Ok(())
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
        let filter = MetadataFilter::eq("type", json!("document"));
        let metadata = make_metadata(&[("type", json!("document"))]);
        assert!(filter.matches(&metadata));
    }

    #[test]
    fn test_eq_no_match() {
        let filter = MetadataFilter::eq("type", json!("document"));
        let metadata = make_metadata(&[("type", json!("image"))]);
        assert!(!filter.matches(&metadata));
    }

    #[test]
    fn test_eq_missing_key() {
        let filter = MetadataFilter::eq("type", json!("document"));
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
            MetadataFilter::eq("type", json!("document")),
            MetadataFilter::exists("title"),
        ]);
        let metadata = make_metadata(&[("type", json!("document")), ("title", json!("Test"))]);
        assert!(filter.matches(&metadata));
    }

    #[test]
    fn test_and_one_fails() {
        let filter = MetadataFilter::and(vec![
            MetadataFilter::eq("type", json!("document")),
            MetadataFilter::exists("author"),
        ]);
        let metadata = make_metadata(&[("type", json!("document")), ("title", json!("Test"))]);
        assert!(!filter.matches(&metadata));
    }

    #[test]
    fn test_or_any_match() {
        let filter = MetadataFilter::or(vec![
            MetadataFilter::eq("type", json!("document")),
            MetadataFilter::eq("type", json!("image")),
        ]);
        let metadata = make_metadata(&[("type", json!("image"))]);
        assert!(filter.matches(&metadata));
    }

    #[test]
    fn test_or_none_match() {
        let filter = MetadataFilter::or(vec![
            MetadataFilter::eq("type", json!("document")),
            MetadataFilter::eq("type", json!("image")),
        ]);
        let metadata = make_metadata(&[("type", json!("video"))]);
        assert!(!filter.matches(&metadata));
    }

    #[test]
    fn test_not() {
        let filter = !MetadataFilter::eq("private", json!(true));
        let metadata = make_metadata(&[("private", json!(false))]);
        assert!(filter.matches(&metadata));
    }

    #[test]
    fn test_depth_and_rejection() {
        let mut f = MetadataFilter::eq("a", json!(1));
        assert_eq!(f.depth(), 1);
        for i in 0..MAX_FILTER_DEPTH {
            f = MetadataFilter::and(vec![f, MetadataFilter::eq("k", json!(i))]);
        }
        assert!(f.depth() > MAX_FILTER_DEPTH);
        assert!(f.validate().is_err());
    }
}
