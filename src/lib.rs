//! Native window pixel capture for Bevy via [`xcap`](https://github.com/nashaofu/xcap).
//!
//! Bevy's built-in `Screenshot` only captures wgpu-rendered content. This
//! crate captures actual OS window pixels — useful when the window contains
//! native toolkit UI (Cocoa, Win32, GTK) or embedded third-party content.
//!
//! ```ignore
//! use bevy_xcap::prelude::*;
//!
//! commands
//!     .spawn(NativeScreenshot::window(window_entity))
//!     .observe(save_to_disk("screenshot.png"));
//! ```

pub mod prelude {
    pub use crate::{
        Captured, Capturing, NativeScreenshot, NativeScreenshotCaptured, XCapPlugin, save_to_disk,
    };
}

use bevy::prelude::*;
use bevy::window::RawHandleWrapper;
use std::sync::{mpsc, Mutex};

#[derive(Component)]
pub struct NativeScreenshot {
    pub target: Entity,
}

impl NativeScreenshot {
    pub fn window(window: Entity) -> Self {
        Self { target: window }
    }
}

#[derive(Component, Default)]
pub struct Capturing;

#[derive(Component, Default)]
pub struct Captured;

#[derive(EntityEvent)]
pub struct NativeScreenshotCaptured {
    pub entity: Entity,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Observer callback that saves captured pixels to a PNG file.
pub fn save_to_disk(
    path: impl Into<std::path::PathBuf>,
) -> impl FnMut(On<NativeScreenshotCaptured>) {
    let path = path.into();
    move |captured: On<NativeScreenshotCaptured>| {
        let c = &*captured;
        match image::save_buffer(&path, &c.rgba, c.width, c.height, image::ColorType::Rgba8) {
            Ok(()) => info!("[bevy_xcap] Saved {}x{} screenshot to {}", c.width, c.height, path.display()),
            Err(e) => error!("[bevy_xcap] Failed to save screenshot: {e}"),
        }
    }
}

type CaptureResult = (Entity, Result<(u32, u32, Vec<u8>), String>);

#[derive(Resource)]
struct CaptureReceiver(Mutex<mpsc::Receiver<CaptureResult>>);

#[derive(Resource, Clone)]
struct CaptureSender(mpsc::Sender<CaptureResult>);

pub struct XCapPlugin;

impl Plugin for XCapPlugin {
    fn build(&self, app: &mut App) {
        let (tx, rx) = mpsc::channel();
        app.insert_resource(CaptureSender(tx));
        app.insert_resource(CaptureReceiver(Mutex::new(rx)));
        app.add_systems(Update, (dispatch_captures, poll_captures));
    }
}

/// Dispatches new capture requests to background threads.
fn dispatch_captures(
    mut commands: Commands,
    screenshots: Query<(Entity, &NativeScreenshot), Added<NativeScreenshot>>,
    handles: Query<&RawHandleWrapper>,
    windows: Query<&Window>,
    sender: Res<CaptureSender>,
) {
    for (screenshot_entity, screenshot) in &screenshots {
        let Ok(raw_handle) = handles.get(screenshot.target) else {
            warn!(
                "[bevy_xcap] Target entity {:?} has no RawHandleWrapper",
                screenshot.target
            );
            commands.entity(screenshot_entity).despawn();
            continue;
        };

        let window_title = windows
            .get(screenshot.target)
            .map(|w| w.title.clone())
            .ok();

        commands.entity(screenshot_entity).insert(Capturing);

        let raw_handle = raw_handle.clone();
        let tx = sender.0.clone();

        std::thread::spawn(move || {
            let result = capture_window(&raw_handle, window_title.as_deref());
            let _ = tx.send((screenshot_entity, result));
        });
    }
}

/// Collects completed captures and triggers entity events.
fn poll_captures(mut commands: Commands, receiver: Res<CaptureReceiver>) {
    let rx = receiver.0.lock().unwrap();
    while let Ok((screenshot_entity, result)) = rx.try_recv() {
        match result {
            Ok((width, height, rgba)) => {
                commands
                    .entity(screenshot_entity)
                    .remove::<Capturing>()
                    .insert(Captured)
                    .trigger(move |entity| NativeScreenshotCaptured {
                        entity,
                        width,
                        height,
                        rgba,
                    });
                commands.entity(screenshot_entity).despawn();
            }
            Err(e) => {
                warn!("[bevy_xcap] Failed to capture window: {e}");
                commands.entity(screenshot_entity).despawn();
            }
        }
    }
}

fn capture_window(
    raw_handle: &RawHandleWrapper,
    title: Option<&str>,
) -> Result<(u32, u32, Vec<u8>), String> {
    let all_windows =
        xcap::Window::all().map_err(|e| format!("Failed to enumerate windows: {e}"))?;

    let handle = raw_handle.get_window_handle();

    // Match by native window ID (Windows/Linux)
    if let Some(target_id) = native_window_id(handle) {
        if let Some(w) = all_windows.iter().find(|w| w.id().ok() == Some(target_id)) {
            return capture_xcap_window(w);
        }
    }

    // Fallback: match by title (macOS doesn't expose window IDs via raw handles)
    if let Some(title) = title {
        if let Some(w) = all_windows
            .iter()
            .find(|w| w.title().ok().as_deref() == Some(title))
        {
            return capture_xcap_window(w);
        }
    }

    Err("No matching xcap window found".to_string())
}

fn capture_xcap_window(window: &xcap::Window) -> Result<(u32, u32, Vec<u8>), String> {
    let image = window
        .capture_image()
        .map_err(|e| format!("Capture failed: {e}"))?;

    let width = image.width();
    let height = image.height();
    let rgba = image.into_raw();

    Ok((width, height, rgba))
}

fn native_window_id(handle: raw_window_handle::RawWindowHandle) -> Option<u32> {
    #[cfg(target_os = "windows")]
    if let raw_window_handle::RawWindowHandle::Win32(h) = handle {
        return Some(h.hwnd.get() as u32);
    }

    #[cfg(target_os = "linux")]
    match handle {
        raw_window_handle::RawWindowHandle::Xlib(h) => return Some(h.window as u32),
        raw_window_handle::RawWindowHandle::Xcb(h) => return Some(h.window.get()),
        _ => {}
    }

    let _ = handle;
    None
}
