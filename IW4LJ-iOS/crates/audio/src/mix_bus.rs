use asset_core::AssetNamespace;

use crate::render_core::{QUANTUM, SAMPLE_RATE};

const BUSES: usize = 4;

#[derive(Clone, Copy)]
struct Compressor {
    threshold_db: f32,
    ratio: f32,
    makeup_db: f32,
    attack_s: f32,
    release_s: f32,
}

#[derive(Clone, Copy)]
struct BusStage {
    trim_db: f32,
    compressor: Option<Compressor>,
}

fn stage(namespace: AssetNamespace) -> BusStage {
    match namespace {
        AssetNamespace::Iw4 => BusStage {
            trim_db: 0.0,
            compressor: None,
        },
        AssetNamespace::Iw5 => BusStage {
            trim_db: -2.5,
            compressor: None,
        },
        AssetNamespace::T5 => BusStage {
            trim_db: 7.0,
            compressor: None,
        },
        AssetNamespace::T6 => BusStage {
            trim_db: 0.0,
            compressor: Some(Compressor {
                threshold_db: -40.0,
                ratio: 4.0,
                makeup_db: 29.5,
                attack_s: 0.002,
                release_s: 0.212,
            }),
        },
    }
}

fn coefficient(seconds: f32) -> f32 {
    1.0 - (-1.0 / (seconds * SAMPLE_RATE as f32)).exp()
}

fn db_to_gain(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

pub(crate) struct MixBuses {
    buffers: [[[f32; 2]; QUANTUM]; BUSES],
    used: [bool; BUSES],
    envelope: [f32; BUSES],
}

impl Default for MixBuses {
    fn default() -> Self {
        Self {
            buffers: [[[0.0; 2]; QUANTUM]; BUSES],
            used: [false; BUSES],
            envelope: [0.0; BUSES],
        }
    }
}

impl MixBuses {
    pub(crate) fn bus(&mut self, namespace: AssetNamespace) -> &mut [[f32; 2]; QUANTUM] {
        let index = namespace as usize;
        if !self.used[index] {
            self.used[index] = true;
            self.buffers[index] = [[0.0; 2]; QUANTUM];
        }
        &mut self.buffers[index]
    }

    pub(crate) fn mix_into(&mut self, output: &mut [[f32; 2]; QUANTUM]) {
        for namespace in AssetNamespace::ALL {
            let index = namespace as usize;
            let stage = stage(namespace);
            let Some(compressor) = stage.compressor else {
                if self.used[index] {
                    let trim = db_to_gain(stage.trim_db);
                    for (target, frame) in output.iter_mut().zip(&self.buffers[index]) {
                        target[0] += frame[0] * trim;
                        target[1] += frame[1] * trim;
                    }
                }
                continue;
            };
            if !self.used[index] {
                if self.envelope[index] < 1e-12 {
                    continue;
                }
                self.buffers[index] = [[0.0; 2]; QUANTUM];
            }
            let attack = coefficient(compressor.attack_s);
            let release = coefficient(compressor.release_s);
            let slope = 1.0 - 1.0 / compressor.ratio;
            let mut envelope = self.envelope[index];
            for (target, frame) in output.iter_mut().zip(&self.buffers[index]) {
                let power = (frame[0] * frame[0] + frame[1] * frame[1]) * 0.5;
                let power = if power.is_finite() { power } else { 0.0 };
                envelope += (power - envelope) * if power > envelope { attack } else { release };
                let level_db = 10.0 * envelope.max(1e-12).log10();
                let over = (level_db - compressor.threshold_db).max(0.0);
                let gain = db_to_gain(stage.trim_db + compressor.makeup_db - over * slope);
                target[0] += frame[0] * gain;
                target[1] += frame[1] * gain;
            }
            self.envelope[index] = envelope;
        }
        self.used = [false; BUSES];
    }
}

pub(crate) struct MasterLimiter {
    gain: f32,
}

impl Default for MasterLimiter {
    fn default() -> Self {
        Self { gain: 1.0 }
    }
}

const LIMITER_CEILING: f32 = 0.95;
const LIMITER_RELEASE_S: f32 = 0.08;

impl MasterLimiter {
    pub(crate) fn apply(&mut self, output: &mut [[f32; 2]; QUANTUM]) {
        let release = coefficient(LIMITER_RELEASE_S);
        for frame in output {
            let peak = frame[0].abs().max(frame[1].abs());
            let target = if peak > LIMITER_CEILING {
                LIMITER_CEILING / peak
            } else {
                1.0
            };
            self.gain = if target < self.gain {
                target
            } else {
                self.gain + (target - self.gain) * release
            };
            frame[0] *= self.gain;
            frame[1] *= self.gain;
        }
    }
}
