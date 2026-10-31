// SPDX-License-Identifier: MPL-2.0

use audio_engine::{AlgorithmicReverb, ReverbError, ReverbMode, ReverbParameters};

fn engine(rate: f32) -> AlgorithmicReverb {
    let mut engine = AlgorithmicReverb::new(rate).unwrap();
    engine
        .set_parameters(ReverbParameters {
            mix: 1.0,
            ..Default::default()
        })
        .unwrap();
    engine.reset();
    engine
}

#[test]
fn impulse_has_finite_decaying_stereo_tail() {
    for rate in [8_000.0, 44_100.0, 48_000.0, 96_000.0, 192_000.0] {
        let mut e = engine(rate);
        let len = (rate * 5.0) as usize;
        let mut l = vec![0.0; len];
        let mut r = l.clone();
        l[0] = 1.0;
        r[0] = 1.0;
        e.process_stereo(&mut l, &mut r).unwrap();
        assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() < 2.0));
        let energy = |start: usize, end: usize| {
            l[start..end]
                .iter()
                .chain(&r[start..end])
                .map(|v| f64::from(*v).powi(2))
                .sum::<f64>()
        };
        let initial = energy(0, rate as usize);
        assert!(initial > 0.0);
        assert!(energy(len - rate as usize, len) < initial * 0.01);
        assert!(l.iter().zip(&r).any(|(a, b)| (a - b).abs() > 1e-6));
    }
}

#[test]
fn block_partition_does_not_change_output() {
    let mut a = engine(48_000.0);
    let mut b = engine(48_000.0);
    let mut l = vec![0.0; 24_000];
    l[0] = 1.0;
    let mut r = l.clone();
    let mut x = l.clone();
    let mut y = r.clone();
    a.process_stereo(&mut l, &mut r).unwrap();
    for (left, right) in x.chunks_mut(127).zip(y.chunks_mut(127)) {
        b.process_stereo(left, right).unwrap();
    }
    assert_eq!(l, x);
    assert_eq!(r, y);
}

#[test]
fn reset_clears_tail_and_zero_width_is_mono() {
    let mut e = engine(48_000.0);
    e.set_parameters(ReverbParameters {
        mix: 1.0,
        width: 0.0,
        ..Default::default()
    })
    .unwrap();
    e.reset();
    let mut l = vec![0.0; 12_000];
    let mut r = l.clone();
    l[0] = 1.0;
    e.process_stereo(&mut l, &mut r).unwrap();
    assert_eq!(l, r);
    e.reset();
    l.fill(0.0);
    r.fill(0.0);
    e.process_stereo(&mut l, &mut r).unwrap();
    assert!(l.iter().chain(&r).all(|v| *v == 0.0));
}

#[test]
fn rejects_invalid_arguments_and_unsupported_modes() {
    assert!(matches!(
        AlgorithmicReverb::new(f32::NAN),
        Err(ReverbError::InvalidSampleRate)
    ));
    let mut e = engine(48_000.0);
    assert_eq!(
        e.set_parameters(ReverbParameters {
            mix: f32::NAN,
            ..Default::default()
        }),
        Err(ReverbError::InvalidParameters)
    );
    assert_eq!(
        e.set_parameters(ReverbParameters {
            mode: ReverbMode::Convolution,
            ..Default::default()
        }),
        Err(ReverbError::UnsupportedMode)
    );
    let mut left = [1.0, 2.0];
    assert_eq!(
        e.process_stereo(&mut left, &mut [0.0]),
        Err(ReverbError::BufferLengthMismatch)
    );
    assert_eq!(left, [1.0, 2.0]);
}

#[test]
fn dry_bypass_and_nonfinite_input_are_safe() {
    let mut e = engine(48_000.0);
    e.set_parameters(ReverbParameters {
        mix: 0.0,
        ..Default::default()
    })
    .unwrap();
    e.reset();
    let mut l = [0.25, -0.5, f32::NAN];
    let mut r = [0.5, -0.25, f32::INFINITY];
    e.process_stereo(&mut l, &mut r).unwrap();
    assert_eq!(l, [0.25, -0.5, 0.0]);
    assert_eq!(r, [0.5, -0.25, 0.0]);
}

#[test]
fn automation_extremes_remain_finite() {
    let mut e = engine(48_000.0);
    let mut l = [0.0; 256];
    let mut r = l;
    for block in 0..400 {
        let extreme = if block % 2 == 0 { 1.0 } else { 0.0 };
        e.set_parameters(ReverbParameters {
            mix: extreme,
            size: extreme,
            damping: extreme,
            diffusion: extreme,
            pan: extreme * 2.0 - 1.0,
            pre_delay_ms: extreme * 500.0,
            decay_seconds: 0.1 + extreme * 29.9,
            ..Default::default()
        })
        .unwrap();
        l.fill(0.1);
        r.fill(-0.1);
        e.process_stereo(&mut l, &mut r).unwrap();
        assert!(l.iter().chain(&r).all(|v| v.is_finite()));
    }
}

#[test]
fn wet_pan_moves_channel_levels_without_touching_dry_path() {
    let render = |pan, mix| {
        let mut e = engine(48000.0);
        e.set_parameters(ReverbParameters {
            pan,
            mix,
            width: 0.0,
            ..Default::default()
        })
        .unwrap();
        e.reset();
        let mut l = vec![0.0; 24000];
        let mut r = l.clone();
        l[0] = 0.5;
        r[0] = 0.5;
        e.process_stereo(&mut l, &mut r).unwrap();
        (l, r)
    };
    let (center_l, center_r) = render(0.0, 1.0);
    assert_eq!(center_l, center_r);
    let (left_l, left_r) = render(-1.0, 1.0);
    let (right_l, right_r) = render(1.0, 1.0);
    let energy = |x: &[f32]| x.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>();
    assert!(energy(&left_l) > 1e-5 && energy(&left_r) < 1e-10);
    assert!(energy(&right_r) > 1e-5 && energy(&right_l) < 1e-10);
    assert!((energy(&left_l) - energy(&right_r)).abs() < 1e-8);
    let (dry_l, dry_r) = render(1.0, 0.0);
    assert_eq!(dry_l[0], 0.5);
    assert_eq!(dry_r[0], 0.5);
    assert!(dry_l[1..].iter().chain(&dry_r[1..]).all(|x| *x == 0.0));
    let mut e = engine(48000.0);
    assert_eq!(
        e.set_parameters(ReverbParameters {
            pan: 1.1,
            ..Default::default()
        }),
        Err(ReverbError::InvalidParameters)
    );
}
