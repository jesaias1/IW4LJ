use input_iw4::{command_id_lookup, command_name};

use std::collections::BTreeSet;

use bevy::prelude::Resource;

#[derive(Resource, Debug, Default, Clone)]
pub struct ConsoleInputState {
    held: BTreeSet<u32>,
    timed: Vec<(u32, f32)>,

    pending_mouse: Option<(f32, f32)>,

    mouse_rate: Option<(f32, f32)>,
}

pub const PRESS_SECONDS: f32 = 0.15;

pub const PRESS_TICK_DT_MAX: f32 = 0.05;

fn plus_command_id(name: &str) -> Option<u32> {
    let id = command_id_lookup(name)?;
    if id.checked_sub(1).and_then(input_iw4::key_up_command_id) == Some(id) {
        Some(id - 1)
    } else {
        Some(id)
    }
}

impl ConsoleInputState {
    pub fn hold(&mut self, input: &str) -> bool {
        let Some(id) = plus_command_id(input) else {
            return false;
        };
        self.timed.retain(|(held, _)| *held != id);
        self.held.insert(id)
    }

    pub fn press(&mut self, input: &str, seconds: f32) -> bool {
        let Some(id) = plus_command_id(input) else {
            return false;
        };
        self.held.insert(id);
        match self.timed.iter_mut().find(|(held, _)| *held == id) {
            Some(slot) => slot.1 = slot.1.max(seconds),
            None => self.timed.push((id, seconds)),
        }
        true
    }

    pub fn tick(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, PRESS_TICK_DT_MAX);
        for (_, remaining) in self.timed.iter_mut() {
            *remaining -= dt;
        }
        for (id, _) in self.timed.iter().filter(|(_, left)| *left <= 0.0) {
            self.held.remove(id);
        }
        self.timed.retain(|(_, left)| *left > 0.0);
    }

    pub fn release(&mut self, input: &str) -> bool {
        let Some(id) = plus_command_id(input) else {
            return false;
        };
        self.timed.retain(|(held, _)| *held != id);
        self.held.remove(&id)
    }

    pub fn clear(&mut self) {
        self.held.clear();
        self.timed.clear();
        self.pending_mouse = None;
        self.mouse_rate = None;
    }

    pub fn queue_mouse(&mut self, dx: f32, dy: f32) {
        match &mut self.pending_mouse {
            Some((x, y)) => {
                *x += dx;
                *y += dy;
            }
            None => self.pending_mouse = Some((dx, dy)),
        }
    }

    pub fn take_mouse(&mut self) -> (f32, f32) {
        self.pending_mouse.take().unwrap_or((0.0, 0.0))
    }

    pub fn mouse_rate(&self) -> Option<(f32, f32)> {
        self.mouse_rate
    }

    pub fn set_mouse_rate(&mut self, dx: f32, dy: f32) {
        self.mouse_rate = if dx == 0.0 && dy == 0.0 {
            None
        } else {
            Some((dx, dy))
        };
    }

    pub fn held(&self, input: &str) -> bool {
        plus_command_id(input).is_some_and(|id| self.held.contains(&id))
    }

    pub fn ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.held.iter().copied()
    }

    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.held.iter().filter_map(|id| command_name(*id))
    }
}

pub(crate) fn is_bind_command(word: &str) -> bool {
    command_id_lookup(word).is_some()
}
