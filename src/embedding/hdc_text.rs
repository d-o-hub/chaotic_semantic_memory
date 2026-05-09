//! HDC-native text embedding provider.

use crate::embedding::EmbeddingProvider;
use crate::error::Result;
use crate::encoder::TextEncoder;

pub struct HdcTextProvider {
    encoder: TextEncoder,
}

impl HdcTextProvider {
    pub fn new() -> Self { Self { encoder: TextEncoder::new() } }
}

impl Default for HdcTextProvider {
    fn default() -> Self { Self::new() }
}

#[async_trait::async_trait]
impl EmbeddingProvider for HdcTextProvider {
    fn dimension(&self) -> usize { 10240 }
    fn name(&self) -> &str { "hdc" }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let hv = self.encoder.encode(text);
        Ok(hv.data.to_vec())
    }
}
