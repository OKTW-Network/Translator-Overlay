//! In-process WinUI checks for the controlled wrappers and the preview image.
//!
//! Ignored by default: they need Windows App Runtime 2.4 and a desktop session. Run with
//! `cargo test -p translator-app -- --ignored`. A watchdog ends the process if WinUI hangs.

use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use bytes::Bytes;
use parking_lot::Mutex;
use translator_core::NormRect;
use translator_winui::{Application, HSTRING, Result, Visibility, Window, post, set_ui_thread};

use crate::{
    children,
    ui::{
        app::quietly,
        pages::dashboard::preview::{Preview, PreviewInput},
        widgets::{
            column,
            fields::{Combo, SliderNumber, TextField, Toggle},
        },
    },
};

type Log = Arc<Mutex<Vec<String>>>;

fn note(log: &Log) -> impl Fn(String) + Clone + Send + 'static {
    let log = Arc::clone(log);
    move |s| log.lock().push(s)
}

/// Runs `f` on the UI thread after a few frames, so async change events have arrived.
fn later(f: impl FnOnce() + Send + 'static) {
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(500));
        post(f);
    });
}

fn build(events: &Log, failures: &Log) -> Result<()> {
    set_ui_thread()?;
    let window = Window::new()?;
    let panel = column(8.0)?;
    window.SetContent(&panel)?;

    let ev = note(events);
    let mut toggle = Toggle::new({
        let ev = ev.clone();
        move |on| ev(format!("toggle {on}"))
    })?;
    let mut combo = Combo::new("pick", {
        let ev = ev.clone();
        move |i| ev(format!("combo {i:?}"))
    })?;
    let mut pair = SliderNumber::new((0.0, 10.0, 1.0), {
        let ev = ev.clone();
        move |v| ev(format!("slider {v}"))
    })?;
    let text = TextField::new("", move |t| ev(format!("text {t}")))?;
    let mut preview = Preview::new()?;
    // A rebuilt dashboard gets a new `Preview` for the same, unchanged frame.
    let mut rebuilt = Preview::new()?;
    children!(panel; toggle.ts, combo.cb, pair.slider, pair.number, text.tb, preview.none, preview.image, preview.unavailable);
    window.Activate()?;

    let rgba = Bytes::from(vec![200u8; 4 * 4 * 2]);
    let frame = PreviewInput {
        sequence: 1,
        width: 4,
        height: 2,
        rgba: Some(&rgba),
        regions: &[NormRect {
            x: 0.25,
            y: 0.0,
            width: 0.5,
            height: 1.0,
        }],
    };
    // Programmatic writes (a sync) must not look like user edits.
    quietly(|| -> Result<()> {
        toggle.set(true, true)?;
        combo.set(vec!["a".into(), "b".into(), "c".into()], Some(1), true)?;
        pair.set(5.0, true)?;
        text.set("abc")?;
        preview.sync(&frame, 1.0)?;
        rebuilt.sync(&frame, 1.0)
    })?;
    if preview.image.Visibility()? != Visibility::Visible {
        failures.lock().push("preview image is not shown for a valid frame".into());
    }
    if rebuilt.image.Source().is_err() {
        failures
            .lock()
            .push("a second preview of the same frame has no image source".into());
    }

    let (events, failures) = (Arc::clone(events), Arc::clone(failures));
    later(move || {
        let fail = |s: &str| failures.lock().push(s.into());
        if !events.lock().is_empty() {
            fail(&format!("sync raised callbacks: {:?}", events.lock()));
        }
        events.lock().clear();
        // Same changes made outside a sync are user edits.
        let edits = (|| -> Result<()> {
            toggle.ts.SetIsOn(false)?;
            combo.cb.SetSelectedIndex(2)?;
            pair.slider.SetValue(7.0)?;
            text.tb.SetText(&HSTRING::from("abcd"))
        })();
        if let Err(e) = edits {
            fail(&format!("edit failed: {e}"));
        }
        later(move || {
            let got = events.lock().clone();
            for want in ["toggle false", "combo Some(2)", "slider 7", "text abcd"] {
                if !got.iter().any(|e| e == want) {
                    failures.lock().push(format!("missing `{want}` in {got:?}"));
                }
            }
            // A later sync with an unchanged value leaves the user's edit alone.
            if quietly(|| text.set("abc")).is_err() || text.tb.Text().map(|t| t.to_string_lossy()).ok().as_deref() != Some("abcd") {
                failures.lock().push("unchanged state overwrote the user's text".into());
            }
            drop(window);
            if let Ok(app) = Application::Current() {
                let _ = app.Exit();
            }
        });
    });
    Ok(())
}

#[test]
#[ignore = "needs Windows App Runtime 2.4 and a desktop session"]
fn controlled_widgets_and_preview() {
    thread::spawn(|| {
        let start = Instant::now();
        thread::sleep(Duration::from_secs(60));
        eprintln!("WinUI test hung for {:?}; aborting", start.elapsed());
        std::process::exit(101);
    });
    let events: Log = Arc::default();
    let failures: Log = Arc::default();
    let (e, f) = (Arc::clone(&events), Arc::clone(&failures));
    translator_winui::run(move || build(&e, &f)).expect("WinUI run");
    let failures = failures.lock();
    assert!(failures.is_empty(), "{failures:#?}");
}
