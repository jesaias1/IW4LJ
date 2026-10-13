#![allow(deprecated)]
//! Text prompts: the game has no field a phone keyboard can type into, so when
//! its menu wants text it raises a flag, and this shows an alert with a text
//! field and sends the answer back a character at a time. The same arrangement
//! as Android's dialog.

use std::ptr::NonNull;
use std::time::Duration;

use block2::RcBlock;
use dispatch2::DispatchQueue;
use objc2::MainThreadMarker;
use objc2::rc::Weak;
use objc2_foundation::NSString;
use objc2_ui_kit::{
    UIAlertAction, UIAlertActionStyle, UIAlertController, UIAlertControllerStyle, UIApplication,
    UIKeyboardType, UITextAutocorrectionType, UITextField, UITextInputTraits, UIViewController,
};

const NAME: u32 = 1;
const HOST_IP: u32 = 2;

pub fn start() {
    let spawned = std::thread::Builder::new()
        .name("iw4l-prompt".into())
        .spawn(|| {
            loop {
                std::thread::sleep(Duration::from_millis(300));
                let kind = touch::take_prompt();
                if kind != 0 {
                    DispatchQueue::main().exec_async(move || show(kind));
                }
            }
        });
    if let Err(error) = spawned {
        eprintln!("prompt thread: {error}");
    }
}

/// The view controller a new alert has to be presented from: the one on top.
fn top(mtm: MainThreadMarker) -> Option<objc2::rc::Retained<UIViewController>> {
    let windows = UIApplication::sharedApplication(mtm).windows();
    let window = windows.firstObject()?;
    let mut controller = window.rootViewController()?;
    while let Some(next) = controller.presentedViewController() {
        controller = next;
    }
    Some(controller)
}

fn show(kind: u32) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let (title, max, address) = match kind {
        NAME => ("Player name", 15, false),
        HOST_IP => ("Host IP address (shown on the host's LAN tab)", 21, true),
        _ => return,
    };
    let Some(presenter) = top(mtm) else {
        return;
    };
    let alert = UIAlertController::alertControllerWithTitle_message_preferredStyle(
        Some(&NSString::from_str(title)),
        None,
        UIAlertControllerStyle::Alert,
        mtm,
    );
    let configure = RcBlock::new(move |field: NonNull<UITextField>| {
        // SAFETY: UIKit passes a live text field.
        let field = unsafe { field.as_ref() };
        field.setAutocorrectionType(UITextAutocorrectionType::No);
        field.setKeyboardType(if address {
            UIKeyboardType::NumbersAndPunctuation
        } else {
            UIKeyboardType::ASCIICapable
        });
    });
    alert.addTextFieldWithConfigurationHandler(Some(&configure));

    // The alert holds its actions; an action holding the alert would never let
    // either go.
    let weak = Weak::from_retained(&alert);
    let accept = RcBlock::new(move |_action: NonNull<UIAlertAction>| {
        let Some(alert) = weak.load() else {
            return;
        };
        let text = alert
            .textFields()
            .and_then(|fields| fields.firstObject())
            .and_then(|field| field.text())
            .map(|text| text.to_string())
            .unwrap_or_default();
        for character in text.chars().take(max) {
            touch::typed_char(character as u32);
        }
        touch::typed_done(kind);
    });
    alert.addAction(&UIAlertAction::actionWithTitle_style_handler(
        Some(&NSString::from_str("OK")),
        UIAlertActionStyle::Default,
        Some(&accept),
        mtm,
    ));
    alert.addAction(&UIAlertAction::actionWithTitle_style_handler(
        Some(&NSString::from_str("Cancel")),
        UIAlertActionStyle::Cancel,
        None,
        mtm,
    ));
    presenter.presentViewController_animated_completion(&alert, true, None);
}
