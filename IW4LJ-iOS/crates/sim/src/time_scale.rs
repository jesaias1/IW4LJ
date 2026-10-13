#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScriptSlowMotion {
    pub from: f32,
    pub to: f32,
    pub start_ms: i32,
    pub duration_ms: i32,
}

impl ScriptSlowMotion {
    pub fn valid(self) -> bool {
        self.from.is_finite()
            && self.to.is_finite()
            && self.from > 0.0
            && self.to > 0.0
            && self.start_ms >= 0
            && self.duration_ms >= 0
    }

    pub fn sample(self, game_ms: f64) -> f32 {
        let elapsed = (game_ms - f64::from(self.start_ms)).max(0.0);
        let from = f64::from(self.from);
        let to = f64::from(self.to);
        let duration = f64::from(self.duration_ms);
        if duration == 0.0 || elapsed >= duration * (from + to) * 0.5 {
            return self.to;
        }
        (from * from + 2.0 * (to - from) * elapsed / duration)
            .max(0.0)
            .sqrt()
            .clamp(from.min(to), from.max(to)) as f32
    }
}
