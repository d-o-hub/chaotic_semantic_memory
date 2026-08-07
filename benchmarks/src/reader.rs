#![allow(clippy::cast_possible_truncation)]
// Token estimates intentionally truncate character counts to u32.

use crate::types::QueryCase;
use anyhow::Result;

pub struct Reader;

impl Reader {
    pub const fn new() -> Self {
        Self
    }

    pub async fn predict(
        &self,
        query_case: &QueryCase,
        retrieved_texts: &[String],
    ) -> Result<(String, u32, u32)> {
        // Mock reader logic for reader-lite mode.
        // In a real implementation, this would call a small LLM or use a heuristic.
        // Here we simulate a "span-match" heuristic: if any retrieved text contains
        // a key token from the query, we might "predict" something.

        let context = retrieved_texts.join(" ");
        let prediction = if query_case.should_abstain {
            "I don't have enough information to answer that.".to_string()
        } else if let Some(gold) = &query_case.expected_answer {
            // If we have an expected answer, check if it's in the context
            if context.contains(gold) {
                gold.clone()
            } else {
                "Not found in context.".to_string()
            }
        } else {
            "Sample answer based on context.".to_string()
        };

        let prompt_tokens = (context.len() / 4) as u32;
        let completion_tokens = (prediction.len() / 4) as u32;

        Ok((prediction, prompt_tokens, completion_tokens))
    }

    pub fn score_exact_match(&self, predicted: &str, expected: Option<&String>) -> bool {
        match expected {
            Some(exp) => predicted.trim().to_lowercase() == exp.trim().to_lowercase(),
            None => false,
        }
    }
}
