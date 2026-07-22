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
            // Use byte array for binary formats (bincode compatible).
            // Bincode does not support deserialize_any, so we must use
            // serialize_bytes, which bincode handles natively.
            let bytes = self.to_bytes();
            serializer.serialize_bytes(&bytes)
        }
    }
}

struct HVecVisitor;

impl<'de> Visitor<'de> for HVecVisitor {
    type Value = HVec10240;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a base64-encoded string, byte array, or a sequence of 80 u128 values")
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
        // This could be 1280 bytes (legacy) or 80 u128s (bincode)
        // We'll try to decode as u128s.
        let mut words = Vec::with_capacity(80);
        while let Some(word) = seq.next_element::<u128>()? {
            words.push(word);
            if words.len() > 1280 {
                return Err(de::Error::custom("sequence too long for HVec10240"));
            }
        }

        if words.len() == 80 {
            let mut data = [0u128; 80];
            data.copy_from_slice(&words);
            Ok(HVec10240 { data })
        } else if words.len() == 1280 {
            // Legacy byte sequence
            #[allow(clippy::cast_possible_truncation)] // Intentional: values are guaranteed 0-255
            let bytes: Vec<u8> = words.into_iter().map(|w| w as u8).collect();
            HVec10240::from_bytes(&bytes).map_err(de::Error::custom)
        } else {
            Err(de::Error::custom(format!(
                "expected 80 or 1280 elements, got {}",
                words.len()
            )))
        }
    }
}

impl<'de> Deserialize<'de> for HVec10240 {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Bincode does not support deserialize_any, so we branch on format.
        // Human-readable (JSON): use deserialize_any to support base64 strings,
        // byte arrays, and legacy sequence formats.
        // Binary (bincode): deserialize as Vec<u8> and convert.
        if deserializer.is_human_readable() {
            deserializer.deserialize_any(HVecVisitor)
        } else {
            let bytes = <Vec<u8>>::deserialize(deserializer)?;
            Self::from_bytes(&bytes).map_err(de::Error::custom)
        }
    }
}
