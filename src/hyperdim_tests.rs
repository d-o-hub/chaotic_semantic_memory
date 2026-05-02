#[cfg(test)]
mod tests {
    use super::HVec10240;

    #[test]
    fn test_hvec_creation() {
        let vec = HVec10240::zero();
        assert_eq!(vec.data.iter().sum::<u128>(), 0);
    }

    #[test]
    fn test_random_generation() {
        let vec1 = HVec10240::random();
        let vec2 = HVec10240::random();
        assert_ne!(vec1.data, vec2.data);
    }

    #[test]
    fn test_self_similarity() {
        let vec = HVec10240::random();
        let similarity = vec.cosine_similarity(&vec);
        assert!(similarity > 0.99);
    }

    #[test]
    fn test_binding() {
        let a = HVec10240::random();
        let b = HVec10240::random();
        let bound = a.bind(&b);
        let recovered = bound.bind(&b);
        let similarity = a.cosine_similarity(&recovered);
        assert!(similarity > 0.95);
    }

    #[test]
    fn test_serialization() {
        let v = HVec10240::random();
        let bytes = v.to_bytes();
        assert_eq!(v.data, HVec10240::from_bytes(&bytes).unwrap().data);
    }

    #[test]
    fn test_bundle() {
        let v: Vec<_> = (0..10).map(|_| HVec10240::random()).collect();
        assert_eq!(HVec10240::bundle(&v).unwrap().data.len(), 80);
    }

    #[test]
    fn test_permute() {
        let v = HVec10240::random();
        assert_eq!(v, v.permute(0));
        let s = v.permute(128);
        for i in 0..80 {
            assert_eq!(s.data[i], v.data[(i + 1) % 80]);
        }
    }

    #[test]
    fn test_json_serialize_is_base64() {
        let v = HVec10240::random();
        let json = serde_json::to_string(&v).unwrap();
        // Should be a base64 string, not an array
        assert!(json.starts_with('"'), "Expected string, got: {}", json);
        assert!(
            !json.starts_with('['),
            "Expected base64 string, not array: {}",
            json
        );
        // Verify roundtrip
        let decoded: HVec10240 = serde_json::from_str(&json).unwrap();
        assert_eq!(v.data, decoded.data);
    }

    #[test]
    fn test_json_array_deserialize_fallback() {
        // Legacy format: array of bytes (for backward compatibility)
        let v = HVec10240::random();
        let bytes = v.to_bytes();
        let array_json: String = serde_json::to_string(&bytes).unwrap();
        let decoded: HVec10240 = serde_json::from_str(&array_json).unwrap();
        assert_eq!(v.data, decoded.data);
    }
}
