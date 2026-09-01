//! Overlay thread message pump.

use tokio::sync::mpsc;
use tracing::warn;
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{
        DispatchMessageW, EVENT_OBJECT_LOCATIONCHANGE, MSG, PM_REMOVE, PeekMessageW, TranslateMessage, WM_APP, WM_MOUSEMOVE, WM_QUIT,
        WaitMessage,
    },
};

use crate::{
    command::OverlayCommand,
    host::{
        OverlayHost,
        follow::{FOLLOW_EVENT_MESSAGE, install_follow_hooks},
    },
    picker::is_picker_message,
};

impl OverlayHost {
    pub fn run(&mut self, mut rx: mpsc::UnboundedReceiver<OverlayCommand>) {
        self.follow_hooks = install_follow_hooks();

        loop {
            let mut apply = false;
            let mut allow_restack = false;
            let mut command_wake = false;
            loop {
                match rx.try_recv() {
                    Ok(OverlayCommand::Shutdown) => {
                        self.teardown();
                        return;
                    }
                    Ok(cmd) => {
                        self.handle(cmd);
                        apply = true;
                        allow_restack = true;
                    }
                    Err(mpsc::error::TryRecvError::Empty) => break,
                    Err(mpsc::error::TryRecvError::Disconnected) => {
                        self.teardown();
                        return;
                    }
                }
            }

            if self.drain_thread_messages(&mut apply, &mut allow_restack, &mut command_wake) {
                self.teardown();
                return;
            }

            if apply {
                self.apply_overlay(allow_restack);
                if self.replay_present {
                    self.apply_overlay(allow_restack);
                }
            }

            if command_wake {
                continue;
            }

            if unsafe { WaitMessage() }.is_err() {
                warn!("WaitMessage failed");
                self.teardown();
                return;
            }
        }
    }

    /// Returns `true` when the thread should exit (`WM_QUIT`).
    fn drain_thread_messages(&mut self, apply: &mut bool, allow_restack: &mut bool, command_wake: &mut bool) -> bool {
        let mut msg = MSG::default();
        while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
            if msg.message == WM_QUIT {
                return true;
            }
            if msg.hwnd.is_invalid() {
                if msg.message == WM_APP {
                    *command_wake = true;
                    continue;
                }
                if msg.message == FOLLOW_EVENT_MESSAGE {
                    if self.on_follow_event(msg.wParam.0 as u32, HWND(msg.lParam.0 as *mut _)) {
                        *apply = true;
                        if msg.wParam.0 as u32 != EVENT_OBJECT_LOCATIONCHANGE {
                            *allow_restack = true;
                        }
                    }
                    continue;
                }
            }
            if self.picker.is_some() && msg.hwnd == self.hwnd && is_picker_message(msg.message) {
                self.dispatch_picker_msg(&msg);
                *apply = true;
                if msg.message != WM_MOUSEMOVE {
                    *allow_restack = true;
                }
                continue;
            }
            let _ = unsafe { TranslateMessage(&msg) };
            unsafe { DispatchMessageW(&msg) };
        }
        false
    }
}
