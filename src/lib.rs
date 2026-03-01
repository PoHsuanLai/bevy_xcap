//! Native window pixel capture for Bevy via [`async-xcap`].
//!
//! Bevy's built-in `Screenshot` only captures wgpu-rendered content. This
//! crate captures actual OS window pixels — useful when the window contains
//! native toolkit UI (Cocoa, Win32, GTK) or embedded third-party content.
//!
//! On macOS, uses ScreenCaptureKit for truly async capture.
//! On Windows/Linux, wraps xcap.
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

/// Dispatches new capture requests to the async task pool.
fn dispatch_captures(
    mut commands: Commands,
    screenshots: Query<(Entity, &NativeScreenshot), Added<NativeScreenshot>>,
    windows: Query<&Window>,
    sender: Res<CaptureSender>,
) {
    for (screenshot_entity, screenshot) in &screenshots {
        let window_title = windows
            .get(screenshot.target)
            .map(|w| w.title.clone())
            .ok();

        commands.entity(screenshot_entity).insert(Capturing);

        let tx = sender.0.clone();

        bevy::tasks::AsyncComputeTaskPool::get()
            .spawn(async move {
                let result = async_capture(window_title.as_deref()).await;
                let _ = tx.send((screenshot_entity, result));
            })
            .detach();
    }
}

async fn async_capture(title: Option<&str>) -> Result<(u32, u32, Vec<u8>), String> {
    let windows = async_xcap::Window::all()
        .await
        .map_err(|e| e.to_string())?;

    let window = if let Some(title) = title {
        windows
            .iter()
            .find(|w| w.title().ok().as_deref() == Some(title))
            .ok_or_else(|| format!("No window with title '{title}'"))?
    } else {
        return Err("No window title provided".into());
    };

    let image = window.capture_image().await.map_err(|e| e.to_string())?;
    Ok((image.width(), image.height(), image.into_raw()))
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
