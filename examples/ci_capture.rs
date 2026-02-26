//! CI test: auto-captures a screenshot after a few frames and verifies the PNG.
//!
//! Run with: `xvfb-run cargo run --example ci_capture`

use bevy::prelude::*;
use bevy::color::palettes::css;
use bevy_xcap::{NativeScreenshot, NativeScreenshotCaptured, XCapPlugin};

const OUTPUT_PATH: &str = "./ci_screenshot_test.png";
const WAIT_FRAMES: u32 = 30;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "bevy_xcap CI test".into(),
                resolution: bevy::window::WindowResolution::new(400, 300),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.15, 0.15, 0.2)))
        .add_plugins(XCapPlugin)
        .init_resource::<FrameCounter>()
        .add_systems(Startup, setup)
        .add_systems(Update, auto_capture)
        .run();
}

#[derive(Resource, Default)]
struct FrameCounter(u32);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);

    commands.spawn((
        Mesh2d(meshes.add(Circle::new(60.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(css::CORAL))),
        Transform::from_xyz(-80.0, 30.0, 0.0),
    ));
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(120.0, 80.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(css::DODGER_BLUE))),
        Transform::from_xyz(60.0, -20.0, 0.0),
    ));

    let _ = std::fs::remove_file(OUTPUT_PATH);
}

fn auto_capture(
    mut commands: Commands,
    mut counter: ResMut<FrameCounter>,
    mut exit_writer: bevy::ecs::message::MessageWriter<AppExit>,
    windows: Query<Entity, With<Window>>,
) {
    counter.0 += 1;

    if counter.0 == WAIT_FRAMES {
        let Ok(window_entity) = windows.single() else {
            error!("[CI] No window entity found");
            exit_writer.write(AppExit::Error(1.try_into().unwrap()));
            return;
        };

        info!("[CI] Frame {}: triggering native capture -> {}", counter.0, OUTPUT_PATH);

        let output = OUTPUT_PATH.to_string();
        commands
            .spawn(NativeScreenshot::window(window_entity))
            .observe(
                move |captured: On<NativeScreenshotCaptured>,
                      mut exit_writer: bevy::ecs::message::MessageWriter<AppExit>| {
                    let c = &*captured;
                    info!("[CI] Captured {}x{} pixels", c.width, c.height);

                    match image::save_buffer(
                        &output,
                        &c.rgba,
                        c.width,
                        c.height,
                        image::ColorType::Rgba8,
                    ) {
                        Ok(()) => {
                            info!("[CI] Saved to {output}");

                            let meta = std::fs::metadata(&output).expect("file should exist");
                            assert!(meta.len() > 0, "Screenshot file is empty");
                            assert!(c.width > 0 && c.height > 0, "Screenshot has zero dimensions");

                            info!("[CI] PASS: {}x{}, {} bytes", c.width, c.height, meta.len());
                            exit_writer.write(AppExit::Success);
                        }
                        Err(e) => {
                            error!("[CI] FAIL: could not save PNG: {e}");
                            exit_writer.write(AppExit::Error(1.try_into().unwrap()));
                        }
                    }
                },
            );
    }

    if counter.0 > 120 {
        error!("[CI] FAIL: timed out waiting for capture");
        exit_writer.write(AppExit::Error(1.try_into().unwrap()));
    }
}
