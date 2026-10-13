#![allow(deprecated)]
//! Telling the player what went wrong.
//!
//! A phone has no console and a launch error would otherwise be a window that
//! closes by itself. Before UIKit is running, the error is shown by a tiny
//! application of its own; once the game is up, as an alert over it.

use std::cell::RefCell;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};

use block2::RcBlock;
use dispatch2::DispatchQueue;
use objc2::rc::Retained;
use objc2::{ClassType, MainThreadMarker, MainThreadOnly, define_class};
use objc2_foundation::{NSDictionary, NSObjectProtocol, NSString};
use objc2_ui_kit::{
    NSTextAlignment, UIAlertAction, UIAlertActionStyle, UIAlertController, UIAlertControllerStyle,
    UIApplication, UIApplicationDelegate, UIColor, UILabel, UIResponder, UIScreen,
    UIViewController, UIWindow,
};

use crate::platform::app_running;

static MESSAGE: Mutex<String> = Mutex::new(String::new());
static SHOWN: AtomicBool = AtomicBool::new(false);

thread_local! {
    static WINDOW: RefCell<Option<Retained<UIWindow>>> = const { RefCell::new(None) };
}

pub fn install() {
    diag::set_launch_error_hook(show);
}

const FOOTER: &str = "\n\nFix this, then close the app and open it again.";

fn show(message: &str) {
    if SHOWN.swap(true, Ordering::AcqRel) {
        return;
    }
    let text = format!("{message}{FOOTER}");
    if app_running() {
        alert_and_wait(text);
    } else if let Some(mtm) = MainThreadMarker::new() {
        if let Ok(mut slot) = MESSAGE.lock() {
            *slot = text;
        }
        // Make sure the delegate class exists before UIKit looks it up by name.
        let _ = NoticeDelegate::class();
        // UIKit takes over this thread and does not give it back.
        UIApplication::main(
            None,
            Some(&NSString::from_str("IW4LJNoticeDelegate")),
            mtm,
        );
    }
}

/// Show an alert over the running game and return once it has been read.
fn alert_and_wait(text: String) {
    let done = Arc::new((Mutex::new(false), Condvar::new()));
    let signal = Arc::clone(&done);
    DispatchQueue::main().exec_async(move || {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        let alert = UIAlertController::alertControllerWithTitle_message_preferredStyle(
            Some(&NSString::from_str("IW4LJ")),
            Some(&NSString::from_str(&text)),
            UIAlertControllerStyle::Alert,
            mtm,
        );
        let finished = Arc::clone(&signal);
        let close = RcBlock::new(move |_action: NonNull<UIAlertAction>| {
            let (flag, condvar) = &*finished;
            if let Ok(mut flag) = flag.lock() {
                *flag = true;
            }
            condvar.notify_all();
        });
        alert.addAction(&UIAlertAction::actionWithTitle_style_handler(
            Some(&NSString::from_str("Close")),
            UIAlertActionStyle::Default,
            Some(&close),
            mtm,
        ));
        let windows = UIApplication::sharedApplication(mtm).windows();
        if let Some(window) = windows.firstObject()
            && let Some(mut controller) = window.rootViewController()
        {
            while let Some(next) = controller.presentedViewController() {
                controller = next;
            }
            controller.presentViewController_animated_completion(&alert, true, None);
        }
    });
    // On the main thread this would wait for an alert that cannot be drawn, so
    // give up after a moment there; anywhere else wait until it is read.
    let limit = if MainThreadMarker::new().is_some() {
        std::time::Duration::from_secs(1)
    } else {
        std::time::Duration::from_secs(600)
    };
    let (flag, condvar) = &*done;
    let Ok(mut closed) = flag.lock() else {
        return;
    };
    while !*closed {
        match condvar.wait_timeout(closed, limit) {
            Ok((guard, result)) => {
                closed = guard;
                if result.timed_out() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

define_class!(
    #[unsafe(super(UIResponder))]
    #[thread_kind = MainThreadOnly]
    #[name = "IW4LJNoticeDelegate"]
    struct NoticeDelegate;

    unsafe impl NSObjectProtocol for NoticeDelegate {}

    unsafe impl UIApplicationDelegate for NoticeDelegate {
        #[unsafe(method(application:didFinishLaunchingWithOptions:))]
        fn did_finish_launching(
            &self,
            _application: &UIApplication,
            _options: Option<&NSDictionary>,
        ) -> bool {
            let mtm = self.mtm();
            let bounds = UIScreen::mainScreen(mtm).bounds();
            let window = UIWindow::initWithFrame(mtm.alloc(), bounds);
            let controller = UIViewController::new(mtm);
            if let Some(view) = controller.view() {
                view.setBackgroundColor(Some(&UIColor::blackColor()));
                let label = UILabel::initWithFrame(mtm.alloc(), bounds);
                label.setNumberOfLines(0);
                label.setTextAlignment(NSTextAlignment::Center);
                // SAFETY: a plain setter with a live colour.
                unsafe { label.setTextColor(Some(&UIColor::whiteColor())) };
                let text = MESSAGE.lock().map(|text| text.clone()).unwrap_or_default();
                label.setText(Some(&NSString::from_str(&text)));
                view.addSubview(&label);
            }
            window.setRootViewController(Some(&controller));
            window.makeKeyAndVisible();
            WINDOW.with(|slot| *slot.borrow_mut() = Some(window));
            true
        }
    }
);
