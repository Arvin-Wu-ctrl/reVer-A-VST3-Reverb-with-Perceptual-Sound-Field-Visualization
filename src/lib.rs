// SPDX-License-Identifier: MPL-2.0

//! Real-time audio boundary.
//!
//! The first implementation target is a preallocated early-reflection stage
//! followed by a frequency-dependent feedback delay network.

mod algorithmic;
pub use algorithmic::{AlgorithmicReverb, ReverbError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReverbMode {
    Algorithmic,
    Convolution,
    Hybrid,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReverbParameters {
    pub mode: ReverbMode,
    pub mix: f32,
    pub pre_delay_ms: f32,
    pub decay_seconds: f32,
    pub size: f32,
    pub damping: f32,
    pub diffusion: f32,
    pub early_late_balance: f32,
    pub width: f32,
    /// Stereo wet balance, -1 left to +1 right. Dry path is unchanged.
    pub pan: f32,
}

impl Default for ReverbParameters {
    fn default() -> Self {
        Self {
            mode: ReverbMode::Algorithmic,
            mix: 0.2,
            pre_delay_ms: 20.0,
            decay_seconds: 1.8,
            size: 0.5,
            damping: 0.5,
            diffusion: 0.7,
            early_late_balance: 0.4,
            width: 0.8,
            pan: 0.0,
        }
    }
}
