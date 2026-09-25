pub struct ComplexQuadraticMap {
    pub real_z: f64,
    pub imag_z: f64,
    pub real_c: f64,
    pub imag_c: f64,
}

impl ComplexQuadraticMap {
    pub const fn new(real_z: f64, imag_z: f64, real_c: f64, imag_c: f64) -> Self {
        Self { real_z, imag_z, real_c, imag_c }
    }

    pub fn next(&mut self) {
        let r2 = self.real_z * self.real_z;
        let i2 = self.imag_z * self.imag_z;

        let next_r = r2 - i2 + self.real_c;
        let next_i = 2.0 * self.real_z * self.imag_z + self.imag_c;

        self.real_z = next_r;
        self.imag_z = next_i;
    }
}

fn main() {
    let mut map1 = ComplexQuadraticMap::new(0.1, 0.2, -0.75, 0.1);
    let mut map2 = ComplexQuadraticMap::new(0.1000000001, 0.2, -0.75, 0.1);

    for i in 0..100 {
        map1.next();
        map2.next();
        let dr = (map1.real_z - map2.real_z).abs();
        let di = (map1.imag_z - map2.imag_z).abs();
        if dr > 1e-5 || di > 1e-5 {
            println!("{}: diff=({},{})", i, dr, di);
        }
    }
}
