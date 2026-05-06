use chaotic_semantic_memory::hyperdim::HVec10240;
use bincode;

fn main() {
    let v = HVec10240::random();
    let encoded = bincode::serialize(&v).unwrap();
    println!("Encoded length: {}", encoded.len());
    let decoded: HVec10240 = bincode::deserialize(&encoded).unwrap();
    assert_eq!(v.to_bytes(), decoded.to_bytes());
    println!("Roundtrip success!");
}
