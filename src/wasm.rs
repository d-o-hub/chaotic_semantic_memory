use wasm_bindgen::prelude::*;

use crate::{ChaoticSemanticFramework, HVec10240};

const DEFAULT_WASM_RESERVOIR_SIZE: usize = 50_000;

#[wasm_bindgen]
pub struct WasmFramework {
    inner: Option<ChaoticSemanticFramework>,
    reservoir_size: usize,
}

#[wasm_bindgen]
impl WasmFramework {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmFramework {
        Self {
            inner: None,
            reservoir_size: DEFAULT_WASM_RESERVOIR_SIZE,
        }
    }

    #[wasm_bindgen(js_name = withReservoirSize)]
    pub fn with_reservoir_size(mut self, reservoir_size: usize) -> WasmFramework {
        self.reservoir_size = reservoir_size;
        self
    }

    pub async fn init(&mut self) -> Result<(), JsValue> {
        let builder =
            ChaoticSemanticFramework::singularity().with_reservoir_size(self.reservoir_size);
        let inner = builder
            .build()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.inner = Some(inner);
        Ok(())
    }

    pub fn inject_seeded(&self, name: String, seed: u64) -> f64 {
        if let Some(inner) = &self.inner {
            inner.inject_concept(&name, HVec10240::from_seed(seed))
        } else {
            0.0
        }
    }

    pub async fn probe_seeded(&self, seed: u64, top_k: usize) -> js_sys::Array {
        if let Some(inner) = &self.inner {
            let items = inner
                .retrieve_parallel(HVec10240::from_seed(seed), top_k)
                .await;
            items
                .into_iter()
                .map(|(name, score)| JsValue::from_str(&format!("{name}:{score:.4}")))
                .collect()
        } else {
            js_sys::Array::new()
        }
    }
}
