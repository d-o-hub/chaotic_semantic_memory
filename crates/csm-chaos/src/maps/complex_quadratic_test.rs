use csm_chaos::maps::complex_quadratic::ComplexQuadraticMap;
fn main() {
    let mut map1 = ComplexQuadraticMap::new(0.1, 0.2, -0.75, 0.1);
    let mut map2 = ComplexQuadraticMap::new(0.1000000001, 0.2, -0.75, 0.1);

    for i in 0..100 {
        map1.next();
        map2.next();
        println!("{}: m1=({},{}) m2=({},{})", i, map1.real_z, map1.imag_z, map2.real_z, map2.imag_z);
    }
}
