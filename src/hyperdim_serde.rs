//! Serde Serialize/Deserialize implementations for HVec10240.
//!
//! Extracted from hyperdim.rs to satisfy the 500 LOC gate.

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

use crate::hyperdim::HVec10240;

impl Serialize for HVec10240 {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            // Use base64 for JSON and other human-readable formats
            use base64::Engine;
            use base64::engine::general_purpose::STANDARD;
            let bytes = self.to_bytes();
            let b64 = STANDARD.encode(&bytes);
            serializer.serialize_str(&b64)
        } else {
            // Use fixed-size array for binary formats (bincode compatible)
            let bytes = self.to_bytes();
            serializer.serialize_bytes(&bytes)
        }
    }
}

struct HVecVisitor;

impl<'de> Visitor<'de> for HVecVisitor {
    type Value = HVec10240;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a base64-encoded string or byte array of length 1280")
    }

    fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD;
        let bytes = STANDARD.decode(v).map_err(de::Error::custom)?;
        HVec10240::from_bytes(&bytes).map_err(de::Error::custom)
    }

    fn visit_bytes<E>(self, v: &[u8]) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        HVec10240::from_bytes(v).map_err(de::Error::custom)
    }

    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        // Handle JSON array of numbers (legacy format)
        let mut bytes = Vec::with_capacity(1280);
        while let Some(byte) = seq.next_element::<u8>()? {
            bytes.push(byte);
        }
        if bytes.len() != 1280 {
            return Err(de::Error::custom(format!(
                "expected 1280 bytes, got {}",
                bytes.len()
            )));
        }
        HVec10240::from_bytes(&bytes).map_err(de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for HVec10240 {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Use deserialize_any to handle both string (base64) and bytes formats
        deserializer.deserialize_any(HVecVisitor)
    }
}
