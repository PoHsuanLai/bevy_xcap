//! bevy_xcap — Bevy plugin for native window pixel capture via `xcap`.
//!
//! Bevy's built-in `Screenshot` API only captures wgpu-rendered content.
//! Plugin editors render via native toolkits (Cocoa/Win32/GTK), so we
//! need `xcap` for actual pixel capture of those windows.
//!
//! # Usage
//!
//! Mirrors Bevy's `Screenshot` API:
//!
//! ```ignore
//! commands
//!     .spawn(NativeScreenshot::window(window_entity))
//!     .observe(|captured: On<NativeScreenshotCaptured>| {
//!         let rgba = &captured.rgba;
//!         let (w, h) = (captured.width, captured.height);
//!         // ... use pixel data
//!     });
//! ```

use bevy::prelude::*;
use bevy::window::RawHandleWrapper;
use raw_window_handle::RawWindowHandle;

/// Requests a native pixel capture of a window.
///
/// Analogous to Bevy's `Screenshot` component. Spawn on a new entity with
/// an `.observe()` callback to receive [`NativeScreenshotCaptured`].
///
/// The entity is automatically despawned after the observer fires.
#[derive(Component)]
pub struct NativeScreenshot {
    pub target: Entity,
}

impl NativeScreenshot {
    /// Capture a specific window entity's native pixels.
    pub fn window(window: Entity) -> Self {
        Self { target: window }
    }
}

/// Marker: capture is in progress (window enumeration + pixel read).
#[derive(Component, Default)]
pub struct Capturing;

/// Marker: capture complete, observer pending.
#[derive(Component, Default)]
pub struct Captured;

/// Entity event fired when native pixel capture completes.
///
/// Analogous to Bevy's `ScreenshotCaptured`. Delivered via observers:
///
/// ```ignore
/// .observe(|captured: On<NativeScreenshotCaptured>| { ... })
/// ```
#[derive(EntityEvent)]
pub struct NativeScreenshotCaptured {
    pub entity: Entity,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub struct XCapPlugin;

impl Plugin for XCapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, capture_system);
    }
}

fn capture_system(
    mut commands: Commands,
    screenshots: Query<(Entity, &NativeScreenshot), Added<NativeScreenshot>>,
    handles: Query<&RawHandleWrapper>,
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

        let window_handle: RawWindowHandle = raw_handle.get_window_handle();

        commands
            .entity(screenshot_entity)
            .insert(Capturing);

        match capture_from_handle(window_handle) {
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

                // Auto-despawn after trigger (matches Bevy Screenshot behavior)
                commands.entity(screenshot_entity).despawn();
            }
            Err(e) => {
                warn!("[bevy_xcap] Failed to capture window: {e}");
                commands.entity(screenshot_entity).despawn();
            }
        }
    }
}

fn capture_from_handle(handle: RawWindowHandle) -> Result<(u32, u32, Vec<u8>), String> {
    let windows = xcap::Window::all().map_err(|e| format!("Failed to enumerate windows: {e}"))?;

    let target = find_matching_window(&windows, handle)
        .ok_or_else(|| "No matching xcap window found for handle".to_string())?;

    let image = target
        .capture_image()
        .map_err(|e| format!("Capture failed: {e}"))?;

    let width = image.width();
    let height = image.height();
    let rgba = image.into_raw();

    Ok((width, height, rgba))
}

fn find_matching_window<'a>(
    windows: &'a [xcap::Window],
    handle: RawWindowHandle,
) -> Option<&'a xcap::Window> {
    let target_id = native_window_id(handle)?;
    windows.iter().find(|w| w.id().ok() == Some(target_id))
}

#[cfg(target_os = "macos")]
fn native_window_id(handle: RawWindowHandle) -> Option<u32> {
    match handle {
        RawWindowHandle::AppKit(h) => {
            // On macOS, xcap Window::id() returns the CGWindowID (u32).
            // The AppKit handle gives us an NSView pointer; we need CGWindowID.
            // TODO: resolve NSView -> NSWindow -> CGWindowID via objc calls
            let _ = h;
            None
        }
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn native_window_id(handle: RawWindowHandle) -> Option<u32> {
    match handle {
        RawWindowHandle::Win32(h) => Some(h.hwnd.get() as u32),
        _ => None,
    }
}

#[cfg(target_os = "linux")]
fn native_window_id(handle: RawWindowHandle) -> Option<u32> {
    match handle {
        RawWindowHandle::Xlib(h) => Some(h.window as u32),
        RawWindowHandle::Xcb(h) => Some(h.window.get()),
        _ => None,
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn native_window_id(_handle: RawWindowHandle) -> Option<u32> {
    None
}
