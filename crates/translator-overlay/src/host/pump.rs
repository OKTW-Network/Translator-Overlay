//! Overlay thread message pump.

use tokio::sync::mpsc;
use tracing::warn;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage, WM_APP, WM_QUIT, WaitMessage,
};

use crate::{
    command::OverlayCommand,
    host::{OverlayHost, follow::FOLLOW_SYNC},
    picker,
};

impl OverlayHost {
    pub fn run(&mut self, mut rx: mpsc::UnboundedReceiver<OverlayCommand>) {
        self.follow_hooks = crate::host::follow::install_follow_hooks();

        loop {
            let mut sync = false;
            loop {
                match rx.try_recv() {
                    Ok(OverlayCommand::Shutdown) => {
                        self.teardown();
                        return;
                    }
                    Ok(cmd) => {
                        self.handle(cmd);
                        sync = true;
                    }
                    Err(mpsc::error::TryRecvError::Empty) => break,
                    Err(mpsc::error::TryRecvError::Disconnected) => {
                        self.teardown();
                        return;
                    }
                }
            }

            if self.drain_thread_messages(&mut sync) {
                self.teardown();
                return;
            }

            if FOLLOW_SYNC.swap(false, std::sync::atomic::Ordering::AcqRel) || sync {
                self.tick();
            }

            if unsafe { WaitMessage() }.is_err() {
                warn!("WaitMessage failed");
                self.teardown();
                return;
            }
        }
    }

    /// Returns `true` when the thread should exit (`WM_QUIT`).
    fn drain_thread_messages(&mut self, sync: &mut bool) -> bool {
        let mut msg = MSG::default();
        while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
            if msg.message == WM_QUIT {
                return true;
            }
            if msg.hwnd.is_invalid() && msg.message == WM_APP {
                continue;
            }
            if self.picker.is_some() && msg.hwnd == self.hwnd && picker::is_picker_message(msg.message) {
                self.dispatch_picker_msg(&msg);
                *sync = true;
                continue;
            }
            let _ = unsafe { TranslateMessage(&msg) };
            unsafe { DispatchMessageW(&msg) };
        }
        false
    }
}
