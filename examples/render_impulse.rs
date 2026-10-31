// SPDX-License-Identifier: MPL-2.0
//! Offline example only. No file I/O is performed by the reverb processing loop.
use audio_engine::{AlgorithmicReverb, ReverbParameters};
use std::{
    fs::File,
    io::{BufWriter, Write},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "impulse.wav".into());
    let rate = 48_000_u32;
    let mut reverb = AlgorithmicReverb::new(rate as f32).map_err(|e| format!("{e:?}"))?;
    reverb
        .set_parameters(ReverbParameters {
            mix: 1.0,
            ..Default::default()
        })
        .map_err(|e| format!("{e:?}"))?;
    reverb.reset();
    let mut left = vec![0.0_f32; rate as usize * 8];
    let mut right = left.clone();
    left[0] = 0.5;
    right[0] = 0.5;
    for (l, r) in left.chunks_mut(256).zip(right.chunks_mut(256)) {
        reverb.process_stereo(l, r).map_err(|e| format!("{e:?}"))?;
    }
    let mut file = BufWriter::new(File::create(&path)?);
    let data_bytes = (left.len() * 4) as u32;
    file.write_all(b"RIFF")?;
    file.write_all(&(36 + data_bytes).to_le_bytes())?;
    file.write_all(b"WAVEfmt ")?;
    file.write_all(&16_u32.to_le_bytes())?;
    file.write_all(&1_u16.to_le_bytes())?;
    file.write_all(&2_u16.to_le_bytes())?;
    file.write_all(&rate.to_le_bytes())?;
    file.write_all(&(rate * 4).to_le_bytes())?;
    file.write_all(&4_u16.to_le_bytes())?;
    file.write_all(&16_u16.to_le_bytes())?;
    file.write_all(b"data")?;
    file.write_all(&data_bytes.to_le_bytes())?;
    for (l, r) in left.iter().zip(&right) {
        for sample in [l, r] {
            file.write_all(&((sample.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes())?;
        }
    }
    file.flush()?;
    println!("Wrote {path}: 8 s, 48 kHz, stereo PCM16.");
    Ok(())
}
