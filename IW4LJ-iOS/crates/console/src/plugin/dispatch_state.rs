use bevy::prelude::*;

use crate::ConsoleCommand;

#[derive(Resource, Default)]
pub struct ConsoleCommandQueue(pub std::collections::VecDeque<ConsoleCommand>);

impl ConsoleCommandQueue {
    pub fn push_script(&mut self, script: &str) {
        self.0.extend(ConsoleCommand::parse_script(script));
    }
}

#[derive(Resource, Default)]
pub struct ConsoleDispatch {
    pub paused: bool,

    pub wait_remaining: f32,

    pub wait_progression: bool,
    pub wait_progression_elapsed: f32,

    pub wait_world: bool,
    pub wait_world_elapsed: f32,

    pub wait_spawn: bool,
    pub wait_spawn_elapsed: f32,

    pub wait_spawn_admit: bool,

    pub pending_spawn_class: Option<String>,

    pub wait_torn: bool,
    pub wait_torn_elapsed: f32,

    pub wait_ambient: bool,
    pub wait_ambient_elapsed: f32,

    pub wait_move: Option<WaitMovePose>,
    pub wait_move_elapsed: f32,

    pub wait_playing: bool,
    pub wait_playing_elapsed: f32,

    pub wait_tick: Option<u32>,
    pub wait_tick_elapsed: f32,

    pub wait_alive: Option<(sim::ClientId, sim::LifeSequence)>,
    pub wait_alive_elapsed: f32,

    pub quit_jumps: u64,

    pub fifo_jumps: u64,
}

impl ConsoleDispatch {
    pub fn release(&mut self) {
        self.paused = false;
        self.wait_remaining = 0.0;
        self.wait_world = false;
        self.wait_progression = false;
        self.wait_spawn = false;
        self.wait_spawn_admit = false;
        self.pending_spawn_class = None;
        self.wait_torn = false;
        self.wait_ambient = false;
        self.wait_move = None;
        self.wait_playing = false;
        self.wait_tick = None;
        self.wait_alive = None;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WaitMovePose {
    pub client: sim::ClientId,
    pub origin: [f32; 3],
    pub angles: [f32; 3],
}

pub(super) const WAIT_WORLD_TIMEOUT_SECS: f32 = 120.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum WaitKind {
    Seconds(f32),
    Ticks(u32),
    World,
    Progression,
    Spawn,
    Torn,
    Ambient,
}

pub(crate) fn parse_wait_args(args: &[String]) -> WaitKind {
    match args.first().map(String::as_str) {
        Some(s) if s.eq_ignore_ascii_case("progression") => WaitKind::Progression,
        Some(s) if s.eq_ignore_ascii_case("world") => WaitKind::World,
        Some(s) if s.eq_ignore_ascii_case("spawn") => WaitKind::Spawn,
        Some(s) if s.eq_ignore_ascii_case("torn") => WaitKind::Torn,
        Some(s) if s.eq_ignore_ascii_case("ambient") => WaitKind::Ambient,
        Some(s) if s.ends_with(['t', 'T']) && s[..s.len() - 1].parse::<u32>().is_ok() => {
            WaitKind::Ticks(s[..s.len() - 1].parse().unwrap_or(0))
        }
        other => {
            let secs = other
                .and_then(|s| s.trim_end_matches(['s', 'S']).parse::<f32>().ok())
                .unwrap_or(1.0)
                .clamp(0.0, WAIT_WORLD_TIMEOUT_SECS);
            WaitKind::Seconds(secs)
        }
    }
}
