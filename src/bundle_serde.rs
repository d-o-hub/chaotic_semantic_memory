//! Serde Serialize/Deserialize implementations for BundleAccumulator.

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

use crate::bundle::BundleAccumulator;
use crate::hyperdim::HVec10240;

impl Serialize for BundleAccumulator {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("BundleAccumulator", 2)?;
        // Serialize as a slice since large arrays aren't natively supported by Serde
        state.serialize_field("counts", &self.counts.as_slice())?;
        state.serialize_field("n", &self.n)?;
        state.end()
    }
}

struct BundleVisitor;

impl<'de> Visitor<'de> for BundleVisitor {
    type Value = BundleAccumulator;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("struct BundleAccumulator")
    }

    fn visit_map<V>(self, mut map: V) -> std::result::Result<Self::Value, V::Error>
    where
        V: de::MapAccess<'de>,
    {
        let mut counts: Option<Vec<i32>> = None;
        let mut n: Option<u32> = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "counts" => {
                    counts = Some(map.next_value()?);
                }
                "n" => {
                    n = Some(map.next_value()?);
                }
                _ => {
                    let _: serde::de::IgnoredAny = map.next_value()?;
                }
            }
        }

        let counts_vec = counts.ok_or_else(|| de::Error::missing_field("counts"))?;
        if counts_vec.len() != HVec10240::DIMENSION {
            return Err(de::Error::custom(format!(
                "expected {} counts, got {}",
                HVec10240::DIMENSION,
                counts_vec.len()
            )));
        }

        let mut counts_array = Box::new([0i32; HVec10240::DIMENSION]);
        counts_array.copy_from_slice(&counts_vec);

        let n = n.ok_or_else(|| de::Error::missing_field("n"))?;

        Ok(BundleAccumulator {
            counts: counts_array,
            n,
        })
    }
}

impl<'de> Deserialize<'de> for BundleAccumulator {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_struct("BundleAccumulator", &["counts", "n"], BundleVisitor)
    }
}
