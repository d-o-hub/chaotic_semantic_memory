#[cfg(test)]
mod tests {
    use crate::hyperdim::HVec10240;
    use crate::singularity::{Concept, ConceptBuilder, Singularity};

    fn c(id: &str) -> Concept {
        ConceptBuilder::new(id)
            .with_vector(HVec10240::random())
            .build()
            .unwrap()
    }
    #[test]
    fn crud() {
        let mut s = Singularity::new();
        s.inject(c("x")).unwrap();
        assert!(s.get("x").is_some());
        s.delete("x").unwrap();
        assert!(s.get("x").is_none() && s.id_to_index.is_empty());
        assert!(s.delete("m").is_ok());
    }
    #[test]
    fn update() {
        let mut s = Singularity::new();
        s.inject(c("x")).unwrap();
        let v = HVec10240::random();
        s.update("x", v).unwrap();
        assert_eq!(s.get("x").unwrap().vector, v);
        assert!(s.update("m", HVec10240::random()).is_err());
    }
    #[test]
    fn assoc() {
        let mut s = Singularity::new();
        s.inject(c("a")).unwrap();
        s.inject(c("b")).unwrap();
        s.associate("a", "b", 0.5).unwrap();
        assert_eq!(s.get_associations("a"), vec![("b".into(), 0.5)]);
        assert!(s.associate("a", "m", 0.5).is_err());
        assert!(s.associate("a", "b", -1.0).is_err());
        assert!(s.associate("a", "b", f32::NAN).is_err());
    }
    #[test]
    fn similar_empty() {
        assert!(
            Singularity::new()
                .find_similar(&HVec10240::random(), 5)
                .is_empty()
        );
        let mut s = Singularity::new();
        s.inject(c("x")).unwrap();
        assert!(s.find_similar(&HVec10240::random(), 0).is_empty());
    }
    #[test]
    fn clear_all() {
        let mut s = Singularity::new();
        s.inject(c("x")).unwrap();
        s.associate("x", "x", 0.5).unwrap();
        s.clear();
        assert!(s.is_empty() && s.associations.is_empty() && s.concept_indices.is_empty());
    }
}
