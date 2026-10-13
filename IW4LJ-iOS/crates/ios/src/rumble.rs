//! Controller vibration, through GameController's haptics.
//!
//! The game asks for vibration the way it does of any pad (`touch::take_rumble`:
//! device, heavy motor, light motor, milliseconds). A pad's two handles are its
//! two motors; each gets a continuous haptic event of the level asked for.

use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Duration;

use dispatch2::DispatchQueue;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_core_haptics::{
    CHHapticEngine, CHHapticEvent, CHHapticEventParameter, CHHapticEventParameterIDHapticIntensity,
    CHHapticEventParameterIDHapticSharpness, CHHapticEventTypeHapticContinuous, CHHapticPattern,
    CHHapticPatternPlayer,
};
use objc2_foundation::NSArray;
use objc2_game_controller::{
    GCController, GCHapticsLocality, GCHapticsLocalityDefault, GCHapticsLocalityLeftHandle,
    GCHapticsLocalityRightHandle,
};

type Player = Retained<ProtocolObject<dyn CHHapticPatternPlayer>>;

thread_local! {
    /// Main thread only: the pads, their engines, and what is playing now.
    static PADS: RefCell<HashMap<i32, Retained<GCController>>> = RefCell::new(HashMap::new());
    static ENGINES: RefCell<HashMap<(i32, bool), Retained<CHHapticEngine>>> =
        RefCell::new(HashMap::new());
    static PLAYING: RefCell<HashMap<(i32, bool), Player>> = RefCell::new(HashMap::new());
}

/// A pad appeared. Main thread.
pub fn register(device: i32, controller: &Retained<GCController>) {
    PADS.with(|pads| pads.borrow_mut().insert(device, controller.clone()));
}

/// A pad went away. Main thread.
pub fn unregister(device: i32) {
    PADS.with(|pads| pads.borrow_mut().remove(&device));
    ENGINES.with(|engines| engines.borrow_mut().retain(|(owner, _), _| *owner != device));
    PLAYING.with(|playing| playing.borrow_mut().retain(|(owner, _), _| *owner != device));
}

/// Hand the game's vibration requests to the pads.
pub fn start() {
    let spawned = std::thread::Builder::new()
        .name("iw4l-rumble".into())
        .spawn(|| {
            loop {
                std::thread::sleep(Duration::from_millis(10));
                if let Some((device, strong, weak, millis)) = touch::take_rumble() {
                    DispatchQueue::main().exec_async(move || {
                        play(device, true, strong, millis);
                        play(device, false, weak, millis);
                    });
                }
            }
        });
    if let Err(error) = spawned {
        eprintln!("rumble thread: {error}");
    }
}

fn engine_for(device: i32, left: bool) -> Option<Retained<CHHapticEngine>> {
    if let Some(engine) = ENGINES.with(|engines| engines.borrow().get(&(device, left)).cloned()) {
        return Some(engine);
    }
    let controller = PADS.with(|pads| pads.borrow().get(&device).cloned())?;
    // SAFETY: plain getters and a factory on live objects, on the main thread.
    let engine = unsafe {
        let haptics = controller.haptics()?;
        let wanted: &GCHapticsLocality = if left {
            GCHapticsLocalityLeftHandle
        } else {
            GCHapticsLocalityRightHandle
        };
        // A pad with one motor has only the default locality; it is driven by
        // the heavy request.
        let engine = haptics.createEngineWithLocality(wanted).or_else(|| {
            left.then(|| haptics.createEngineWithLocality(GCHapticsLocalityDefault))
                .flatten()
        })?;
        engine.startAndReturnError().ok()?;
        engine
    };
    ENGINES.with(|engines| engines.borrow_mut().insert((device, left), engine.clone()));
    Some(engine)
}

fn play(device: i32, left: bool, level: f32, millis: u32) {
    // What was playing is replaced, or stopped by a request of nothing.
    if let Some(player) = PLAYING.with(|playing| playing.borrow_mut().remove(&(device, left))) {
        // SAFETY: cancelling a player that may already have finished is allowed.
        unsafe {
            let _ = player.cancelAndReturnError();
        }
    }
    if level <= 0.0 || millis == 0 {
        return;
    }
    let Some(engine) = engine_for(device, left) else {
        return;
    };
    // SAFETY: the event, pattern and player are built from live values and the
    // engine is running.
    let player = unsafe {
        let intensity = CHHapticEventParameter::initWithParameterID_value(
            objc2::AnyThread::alloc(),
            CHHapticEventParameterIDHapticIntensity,
            level.clamp(0.0, 1.0),
        );
        let sharpness = CHHapticEventParameter::initWithParameterID_value(
            objc2::AnyThread::alloc(),
            CHHapticEventParameterIDHapticSharpness,
            if left { 0.3 } else { 0.8 },
        );
        let event = CHHapticEvent::initWithEventType_parameters_relativeTime_duration(
            objc2::AnyThread::alloc(),
            CHHapticEventTypeHapticContinuous,
            &NSArray::from_retained_slice(&[intensity, sharpness]),
            0.0,
            f64::from(millis) / 1000.0,
        );
        let pattern = CHHapticPattern::initWithEvents_parameters_error(
            objc2::AnyThread::alloc(),
            &NSArray::from_retained_slice(&[event]),
            &NSArray::new(),
        );
        let player = pattern
            .ok()
            .and_then(|pattern| engine.createPlayerWithPattern_error(&pattern).ok());
        match player {
            Some(player) if player.startAtTime_error(0.0).is_ok() => player,
            _ => {
                // The engine stopped under us (the app was away); the next
                // request makes a new one.
                ENGINES.with(|engines| engines.borrow_mut().remove(&(device, left)));
                return;
            }
        }
    };
    PLAYING.with(|playing| playing.borrow_mut().insert((device, left), player));
}
