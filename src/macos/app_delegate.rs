
//! Implementing `NSApplicationDelegate` for a custom class.
#![deny(unsafe_op_in_unsafe_fn)]
use std::path::PathBuf;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSApplication, NSApplicationDelegate};
use objc2_foundation::{NSArray, NSObject, NSObjectProtocol, NSString};

define_class!(
    // SAFETY:
    // - The superclass NSObject does not have any subclassing requirements.
    // - `AppDelegate` does not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    pub struct AppDelegate;

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[unsafe(method(application:openFiles:))]
        fn application_open_files(&self, _app: &NSApplication, files: &NSArray<NSString>) {
            let paths: Vec<PathBuf> = files.iter().map(|s| PathBuf::from(s.to_string())).collect();

            // enqueue → channel → egui update
            println!("open files: {paths:?}");
        }
    }
);

impl AppDelegate {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        unsafe { msg_send![super(mtm.alloc().set_ivars(())), init] }
    }
}

pub fn install_app_delegate() {
    let mtm = MainThreadMarker::new().unwrap();
    let delegate = AppDelegate::new(mtm);

    let app = NSApplication::sharedApplication(mtm);
    app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));

    // keep alive for app lifetime
    std::mem::forget(delegate);
}