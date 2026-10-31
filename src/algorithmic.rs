// SPDX-License-Identifier: MPL-2.0

//! Standalone algorithmic prototype. Construction allocates; processing does not.
//! Convolution, dynamics and perceptual analysis are deliberately not implemented here.
use crate::{ReverbMode, ReverbParameters};

const TIMES: [f32; 4] = [0.0297, 0.0371, 0.0411, 0.0437];

struct Delay {
    samples: Vec<f32>,
    position: usize,
}

impl Delay {
    fn new(length: usize) -> Self {
        Self {
            samples: vec![0.0; length],
            position: 0,
        }
    }

    fn read(&self, delay: f32) -> f32 {
        let distance = delay.clamp(1.0, (self.samples.len() - 2) as f32);
        let whole = distance as usize;
        let fraction = distance - whole as f32;
        let a = (self.position + self.samples.len() - whole) % self.samples.len();
        let b = (a + self.samples.len() - 1) % self.samples.len();
        self.samples[a] * (1.0 - fraction) + self.samples[b] * fraction
    }

    fn push(&mut self, value: f32) {
        self.samples[self.position] = value;
        self.position = (self.position + 1) % self.samples.len();
    }

    fn clear(&mut self) {
        self.samples.fill(0.0);
        self.position = 0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReverbError {
    InvalidSampleRate,
    InvalidParameters,
    UnsupportedMode,
    BufferLengthMismatch,
}

/// Four-line orthogonal FDN with stereo early taps and one-pole feedback damping.
/// Recreate outside the audio thread when sample rate changes. Parameter changes
/// are smoothed over roughly 20 ms; changing delay lengths produces pitch glide.
/// Decay is nominal mid/low-band RT60, not a measured room property.
pub struct AlgorithmicReverb {
    sample_rate: f32,
    pre_delay: [Delay; 2],
    lines: [Delay; 4],
    filtered: [f32; 4],
    target: ReverbParameters,
    current: ReverbParameters,
    gains: [f32; 4],
    target_gains: [f32; 4],
    smoothing: f32,
}

impl AlgorithmicReverb {
    pub fn new(sample_rate: f32) -> Result<Self, ReverbError> {
        if !sample_rate.is_finite() || !(8_000.0..=192_000.0).contains(&sample_rate) {
            return Err(ReverbError::InvalidSampleRate);
        }
        let parameters = ReverbParameters::default();
        let gains = feedback_gains(parameters);
        Ok(Self {
            sample_rate,
            pre_delay: std::array::from_fn(|_| Delay::new((sample_rate * 0.6).ceil() as usize + 4)),
            lines: std::array::from_fn(|i| {
                Delay::new((sample_rate * TIMES[i] * 1.5).ceil() as usize + 4)
            }),
            filtered: [0.0; 4],
            target: parameters,
            current: parameters,
            gains,
            target_gains: gains,
            smoothing: 1.0 - (-1.0 / (sample_rate * 0.02)).exp(),
        })
    }

    /// Validates atomically: an error leaves all prior parameters intact.
    pub fn set_parameters(&mut self, p: ReverbParameters) -> Result<(), ReverbError> {
        if p.mode != ReverbMode::Algorithmic {
            return Err(ReverbError::UnsupportedMode);
        }
        let values = [
            p.mix,
            p.pre_delay_ms,
            p.decay_seconds,
            p.size,
            p.damping,
            p.diffusion,
            p.early_late_balance,
            p.width,
            p.pan,
        ];
        if values.iter().any(|v| !v.is_finite())
            || !(0.0..=500.0).contains(&p.pre_delay_ms)
            || !(0.1..=30.0).contains(&p.decay_seconds)
            || !(-1.0..=1.0).contains(&p.pan)
            || [
                p.mix,
                p.size,
                p.damping,
                p.diffusion,
                p.early_late_balance,
                p.width,
            ]
            .iter()
            .any(|v| !(0.0..=1.0).contains(v))
        {
            return Err(ReverbError::InvalidParameters);
        }
        self.target = p;
        self.target_gains = feedback_gains(p);
        Ok(())
    }

    pub fn reset(&mut self) {
        for delay in self.pre_delay.iter_mut().chain(self.lines.iter_mut()) {
            delay.clear();
        }
        self.filtered = [0.0; 4];
        self.current = self.target;
        self.gains = self.target_gains;
    }

    /// No allocations, locks, I/O or logging. Mismatched buffers are unchanged.
    pub fn process_stereo(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
    ) -> Result<(), ReverbError> {
        self.process_stereo_observed(left, right, |_, _, _, _, _, _| {})
    }

    /// Observes actual stages without changing numerical processing order.
    /// early/late are weighted, pre-width; wet is post-width and pre-Mix.
    pub fn process_stereo_observed(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        mut observe: impl FnMut([f32; 2], [f32; 2], [f32; 2], [f32; 2], [f32; 2], [f32; 9]),
    ) -> Result<(), ReverbError> {
        if left.len() != right.len() {
            return Err(ReverbError::BufferLengthMismatch);
        }
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            self.smooth();
            let p = self.current;
            let dry = [finite(*l), finite(*r)];
            let predelay = p.pre_delay_ms * self.sample_rate * 0.001;
            // A zero pre-delay is a direct feed; delays >= one sample use the ring.
            let input = std::array::from_fn::<_, 2, _>(|i| {
                if predelay < 1.0 {
                    dry[i] * (1.0 - predelay) + self.pre_delay[i].read(1.0) * predelay
                } else {
                    self.pre_delay[i].read(predelay)
                }
            });
            let scale = 0.5 + p.size;
            let early = [
                0.6 * self.pre_delay[0].read(predelay + self.sample_rate * 0.011 * scale)
                    + 0.3 * self.pre_delay[1].read(predelay + self.sample_rate * 0.023 * scale),
                0.6 * self.pre_delay[1].read(predelay + self.sample_rate * 0.013 * scale)
                    + 0.3 * self.pre_delay[0].read(predelay + self.sample_rate * 0.019 * scale),
            ];
            for (delay, value) in self.pre_delay.iter_mut().zip(dry) {
                delay.push(value);
            }
            let mut taps = [0.0; 4];
            for (i, tap) in taps.iter_mut().enumerate() {
                *tap = self.lines[i].read(self.sample_rate * TIMES[i] * scale);
                self.filtered[i] =
                    flush((1.0 - 0.85 * p.damping) * *tap + 0.85 * p.damping * self.filtered[i]);
            }
            let feedback = diffusion_mix(self.filtered, p.diffusion);
            let injection = [input[0], input[1], -input[1], input[0]];
            for i in 0..4 {
                self.lines[i].push(flush(injection[i] * 0.25 + feedback[i] * self.gains[i]));
            }
            let late = [(taps[0] + taps[2]) * 0.5, (taps[1] + taps[3]) * 0.5];
            let wet: [f32; 2] = std::array::from_fn(|i| {
                early[i] * p.early_late_balance + late[i] * (1.0 - p.early_late_balance)
            });
            let mid = (wet[0] + wet[1]) * 0.5;
            let side = (wet[0] - wet[1]) * 0.5 * p.width;
            // Center is exactly the previous signal path. Away from center,
            // equal-power stereo BALANCE changes wet channel gains only.
            // This is not a binaural delay or a Haas/localization estimator.
            let gains = if p.pan == 0.0 {
                [1.0, 1.0]
            } else {
                let angle = (p.pan + 1.0) * std::f32::consts::FRAC_PI_4;
                [
                    angle.cos() * std::f32::consts::SQRT_2,
                    angle.sin() * std::f32::consts::SQRT_2,
                ]
            };
            let wet_panned = [(mid + side) * gains[0], (mid - side) * gains[1]];
            *l = dry[0] * (1.0 - p.mix) + wet_panned[0] * p.mix;
            *r = dry[1] * (1.0 - p.mix) + wet_panned[1] * p.mix;
            observe(
                dry,
                [*l, *r],
                wet_panned,
                early.map(|x| x * p.early_late_balance),
                late.map(|x| x * (1.0 - p.early_late_balance)),
                [
                    p.mix,
                    p.pre_delay_ms,
                    p.decay_seconds,
                    p.size,
                    p.damping,
                    p.diffusion,
                    p.early_late_balance,
                    p.width,
                    p.pan,
                ],
            );
        }
        Ok(())
    }

    fn smooth(&mut self) {
        let c = &mut self.current;
        let t = self.target;
        for (value, target) in [
            (&mut c.mix, t.mix),
            (&mut c.pre_delay_ms, t.pre_delay_ms),
            (&mut c.decay_seconds, t.decay_seconds),
            (&mut c.size, t.size),
            (&mut c.damping, t.damping),
            (&mut c.diffusion, t.diffusion),
            (&mut c.early_late_balance, t.early_late_balance),
            (&mut c.width, t.width),
            (&mut c.pan, t.pan),
        ] {
            *value += (target - *value) * self.smoothing;
        }
        for (gain, target) in self.gains.iter_mut().zip(self.target_gains) {
            *gain += (target - *gain) * self.smoothing;
        }
    }
}

// Two layers of plane rotations remain orthogonal at every diffusion setting.
// A linear identity/Hadamard blend instead attenuates some modes between endpoints.
fn diffusion_mix([a, b, c, d]: [f32; 4], diffusion: f32) -> [f32; 4] {
    let s = diffusion * std::f32::consts::FRAC_1_SQRT_2;
    let k = (1.0 - s * s).sqrt();
    let (a, b, c, d) = (k * a + s * b, k * b - s * a, k * c + s * d, k * d - s * c);
    [k * a + s * c, k * b + s * d, k * c - s * a, k * d - s * b]
}

#[cfg(test)]
mod tests {
    use super::diffusion_mix;

    #[test]
    fn diffusion_preserves_energy_and_inner_products() {
        for step in 0..=100 {
            let amount = step as f32 / 100.0;
            let basis: [[f32; 4]; 4] = std::array::from_fn(|i| {
                let mut v = [0.0; 4];
                v[i] = 1.0;
                diffusion_mix(v, amount)
            });
            for i in 0..4 {
                for j in 0..4 {
                    let dot: f32 = basis[i].iter().zip(basis[j]).map(|(a, b)| a * b).sum();
                    assert!((dot - if i == j { 1.0 } else { 0.0 }).abs() < 1e-6);
                }
            }
        }
        let v = [0.25, -0.7, 0.2, 1.0];
        assert_eq!(diffusion_mix(v, 0.0), v);
    }
}

fn feedback_gains(p: ReverbParameters) -> [f32; 4] {
    TIMES.map(|time| 10.0_f32.powf(-3.0 * time * (0.5 + p.size) / p.decay_seconds))
}

fn finite(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}
fn flush(value: f32) -> f32 {
    if value.abs() < 1.0e-20 { 0.0 } else { value }
}
