//! Implementing `NSApplicationDelegate` for a custom class.
#![deny(unsafe_op_in_unsafe_fn)]

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSApplication, NSApplicationDelegate, NSApplicationDelegateReply};
use objc2_foundation::{NSArray, NSObject, NSObjectProtocol, NSString};

use std::io::Write;

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
        fn application_open_files(&self, app: &NSApplication, files: &NSArray<NSString>) {
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/gem_player_open_with.log")
                .unwrap();

            writeln!(f, "openFiles fired:").ok();

            for file in files.iter() {
                writeln!(f, "  {}", file).ok();

                // 🔜 enqueue PathBuf::from(file.to_string())
            }

            unsafe {
                app.replyToOpenOrPrint(NSApplicationDelegateReply::Success);
            }
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

    println!("done install_app_delegate");
}
