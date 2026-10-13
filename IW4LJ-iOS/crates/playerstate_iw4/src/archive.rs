use crate::PlayerState;

impl PlayerState {
    pub const fn is_live_frame(&self) -> bool {
        self.delta_time == 0
    }
}
