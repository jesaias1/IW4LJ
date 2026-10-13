//! Controllers, through Apple's GameController framework.
//!
//! The runtime plays with a controller the way it does on a desktop, and the
//! touch crate already turns what Android's activity reports into that
//! (`touch::BridgeEvent`, in Android's key codes and with `y` downward). So a
//! controller here is read, converted to those same events, and queued; the
//! bridge replays them into Bevy each frame.

use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::Mutex;
use std::sync::atomic::{AtomicI32, Ordering};

use block2::RcBlock;
use dispatch2::DispatchQueue;
use objc2::rc::Retained;
use objc2_foundation::{NSNotification, NSNotificationCenter, NSOperationQueue};
use objc2_game_controller::{
    GCController, GCControllerButtonInput, GCControllerDidConnectNotification,
    GCControllerDidDisconnectNotification, GCControllerElement, GCExtendedGamepad,
};

/// What one pad last said, so only a change is sent.
#[derive(Default, Clone, PartialEq)]
struct Report {
    /// Left stick, right stick, triggers, hat — the order the bridge reads.
    axes: [f32; 8],
    buttons: [bool; 15],
}

/// Android's `KEYCODE_*` for each button read below, in the same order as
/// [`Report::buttons`].
const KEYCODES: [i32; 15] = [
    96,  // A, south
    97,  // B, east
    99,  // X, west
    100, // Y, north
    102, // left shoulder
    103, // right shoulder
    106, // left stick click
    107, // right stick click
    108, // menu, start
    109, // options, select
    110, // home
    19,  // d-pad up
    20,  // d-pad down
    21,  // d-pad left
    22,  // d-pad right
];

static NEXT_DEVICE: AtomicI32 = AtomicI32::new(1);

/// Controller object address to the device number the bridge knows it by.
static DEVICES: Mutex<Option<HashMap<usize, i32>>> = Mutex::new(None);

fn pressed(button: Option<Retained<GCControllerButtonInput>>) -> bool {
    // SAFETY: a plain getter on a live element.
    button.is_some_and(|button| unsafe { button.isPressed() })
}

fn read(pad: &GCExtendedGamepad) -> Report {
    // SAFETY: plain getters on live elements of a live profile.
    unsafe {
        let (left, right, dpad) = (pad.leftThumbstick(), pad.rightThumbstick(), pad.dpad());
        Report {
            // The bridge takes Android's sign, `y` downward.
            axes: [
                left.xAxis().value(),
                -left.yAxis().value(),
                right.xAxis().value(),
                -right.yAxis().value(),
                pad.leftTrigger().value(),
                pad.rightTrigger().value(),
                0.0,
                0.0,
            ],
            buttons: [
                pad.buttonA().isPressed(),
                pad.buttonB().isPressed(),
                pad.buttonX().isPressed(),
                pad.buttonY().isPressed(),
                pad.leftShoulder().isPressed(),
                pad.rightShoulder().isPressed(),
                pressed(pad.leftThumbstickButton()),
                pressed(pad.rightThumbstickButton()),
                pad.buttonMenu().isPressed(),
                pressed(pad.buttonOptions()),
                pressed(pad.buttonHome()),
                dpad.yAxis().value() > 0.5,
                dpad.yAxis().value() < -0.5,
                dpad.xAxis().value() < -0.5,
                dpad.xAxis().value() > 0.5,
            ],
        }
    }
}

fn attach(controller: &Retained<GCController>) {
    // SAFETY: plain getter; the profile is nil for a pad with no standard layout.
    let Some(pad) = (unsafe { controller.extendedGamepad() }) else {
        return;
    };
    let key = Retained::as_ptr(controller) as usize;
    let device = {
        let Ok(mut devices) = DEVICES.lock() else {
            return;
        };
        let devices = devices.get_or_insert_with(HashMap::new);
        if devices.contains_key(&key) {
            return;
        }
        let device = NEXT_DEVICE.fetch_add(1, Ordering::Relaxed);
        devices.insert(key, device);
        device
    };
    crate::rumble::register(device, controller);
    let last = Mutex::new(Report::default());
    let handler = RcBlock::new(
        move |pad: NonNull<GCExtendedGamepad>, _element: NonNull<GCControllerElement>| {
            // SAFETY: GameController passes the live profile.
            let now = read(unsafe { pad.as_ref() });
            let Ok(mut last) = last.lock() else {
                return;
            };
            if *last == now {
                return;
            }
            if last.axes != now.axes {
                touch::push(touch::BridgeEvent::PadAxes {
                    device,
                    axes: now.axes,
                });
            }
            for (index, &down) in now.buttons.iter().enumerate() {
                if down != last.buttons[index] {
                    touch::push(touch::BridgeEvent::PadButton {
                        device,
                        keycode: KEYCODES[index],
                        down,
                    });
                }
            }
            *last = now;
        },
    );
    // SAFETY: the handler is a valid block, which GameController copies.
    unsafe {
        pad.setValueChangedHandler(&*handler as *const _ as *mut _);
    }
}

fn detach(controller: &GCController) {
    let key = controller as *const GCController as usize;
    let device = DEVICES
        .lock()
        .ok()
        .and_then(|mut devices| devices.as_mut()?.remove(&key));
    if let Some(device) = device {
        crate::rumble::unregister(device);
        touch::push(touch::BridgeEvent::PadRemoved { device });
    }
}

fn controller_of(note: &NSNotification) -> Option<Retained<GCController>> {
    note.object()?.downcast::<GCController>().ok()
}

/// Watch for controllers, and pick up any already connected.
pub fn start() {
    DispatchQueue::main().exec_async(|| {
        let center = NSNotificationCenter::defaultCenter();
        let connected = RcBlock::new(|note: NonNull<NSNotification>| {
            // SAFETY: the center passes a live notification.
            if let Some(controller) = controller_of(unsafe { note.as_ref() }) {
                attach(&controller);
            }
        });
        let disconnected = RcBlock::new(|note: NonNull<NSNotification>| {
            // SAFETY: the center passes a live notification.
            if let Some(controller) = controller_of(unsafe { note.as_ref() }) {
                detach(&controller);
            }
        });
        // SAFETY: the names are the framework's own; the observers live as long
        // as the process, so they are never removed.
        unsafe {
            let observers = [
                center.addObserverForName_object_queue_usingBlock(
                    Some(GCControllerDidConnectNotification),
                    None,
                    Some(&NSOperationQueue::mainQueue()),
                    &connected,
                ),
                center.addObserverForName_object_queue_usingBlock(
                    Some(GCControllerDidDisconnectNotification),
                    None,
                    Some(&NSOperationQueue::mainQueue()),
                    &disconnected,
                ),
            ];
            std::mem::forget(observers);
            for controller in GCController::controllers().iter() {
                attach(&controller);
            }
        }
    });
}
