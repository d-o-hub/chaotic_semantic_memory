use ndarray::Array1;
use rayon::prelude::*;

pub const DEFAULT_RECURRENT_GAIN: f32 = 1.131_313;

#[derive(Clone)]
pub struct EchoStateReservoir {
    pub size: usize,
    pub state: Vec<f32>,
    pub spectral_radius: f32,
    recurrent_gain: f32,
}

impl EchoStateReservoir {
    pub fn new(size: usize, spectral_radius: f32) -> Self {
        Self::with_gain(size, spectral_radius, DEFAULT_RECURRENT_GAIN)
    }

    pub fn with_gain(size: usize, spectral_radius: f32, recurrent_gain: f32) -> Self {
        let clamped = spectral_radius.clamp(0.9, 1.1);
        Self {
            size,
            state: vec![0.0; size],
            spectral_radius: clamped,
            recurrent_gain,
        }
    }

    pub fn recurrent_step(&mut self, input: &Array1<f32>) -> f64 {
        let input_len = input.len().max(1);
        self.state.par_iter_mut().enumerate().for_each(|(i, s)| {
            let x = input[i % input_len];
            let recurrent = ((*s * self.recurrent_gain).sin() + x * self.spectral_radius).tanh();
            *s = recurrent;
        });
        self.energy()
    }

    pub fn energy(&self) -> f64 {
        self.state
            .par_iter()
            .map(|x| (*x as f64).abs())
            .sum::<f64>()
            / self.size.max(1) as f64
    }
}
