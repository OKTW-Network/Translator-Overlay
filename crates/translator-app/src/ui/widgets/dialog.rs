//! `ContentDialog` driven by an `open` flag in state.
//!
//! WinUI shows one dialog per window at a time; a dialog that cannot show yet retries on the
//! next sync (the open one's `Closed` triggers it). Closing because state went false does not
//! call `on_closed`.

use std::sync::Arc;

use parking_lot::Mutex;
use translator_winui::{ContentDialog, ContentDialogResult, Result, boxed, keep};

use crate::ui::{
    app::{refresh, xaml_root},
    widgets::{Prop, hs, text},
};

#[derive(Default)]
struct Showing {
    open: bool,
    /// Hidden by a sync; swallow the resulting `Closed`.
    hidden: bool,
}

pub struct Dialog {
    pub cd: ContentDialog,
    showing: Arc<Mutex<Showing>>,
    title: Prop<String>,
    primary: Prop<String>,
    close: Prop<String>,
    body: Prop<String>,
}

impl Dialog {
    /// `on_closed(primary)` runs when the user closes the dialog.
    pub fn new(on_closed: impl Fn(bool) + Send + 'static) -> Result<Self> {
        let cd = ContentDialog::new()?;
        let showing = Arc::new(Mutex::new(Showing::default()));
        let state = Arc::clone(&showing);
        keep(cd.Closed(move |_, args| {
            let primary = args.as_ref().and_then(|a| a.Result().ok()) == Some(ContentDialogResult::Primary);
            let hidden = {
                let mut s = state.lock();
                s.open = false;
                std::mem::take(&mut s.hidden)
            };
            if !hidden {
                on_closed(primary);
            }
            refresh();
        }))?;
        Ok(Self {
            cd,
            showing,
            title: Prop::default(),
            primary: Prop::default(),
            close: Prop::default(),
            body: Prop::default(),
        })
    }

    /// Title and button labels.
    pub fn labels(&mut self, title: &str, primary: &str, close: &str) -> Result<()> {
        let cd = &self.cd;
        self.title.set(title.to_string(), |t| cd.SetTitle(&boxed(t)))?;
        self.primary.set(primary.to_string(), |t| cd.SetPrimaryButtonText(&hs(t)))?;
        self.close.set(close.to_string(), |t| cd.SetCloseButtonText(&hs(t)))
    }

    /// Plain text body.
    pub fn body(&mut self, body: &str) -> Result<()> {
        let cd = &self.cd;
        self.body.set(body.to_string(), |t| cd.SetContent(&text(t)?))
    }

    pub fn show(&self, open: bool) -> Result<()> {
        let mut s = self.showing.lock();
        if open && !s.open {
            let Some(root) = xaml_root() else {
                return Ok(());
            };
            self.cd.SetXamlRoot(&root)?;
            // Fails while another dialog is open; the next sync retries. WinUI may also return
            // S_OK without an async handle, which still means shown.
            if self.cd.ShowAsync().map_or_else(|e| e.code().is_ok(), |_| true) {
                s.open = true;
            }
        } else if !open && s.open {
            s.hidden = true;
            drop(s);
            self.cd.Hide()?;
        }
        Ok(())
    }
}

impl Drop for Dialog {
    // The page that owns this dialog is gone; close it without telling the page.
    fn drop(&mut self) {
        let _ = self.show(false);
    }
}
