//! Press Space to capture a native screenshot. Saves to `./native_screenshot_N.png`.
//!
//! On macOS, grant Screen Recording permission to your terminal
//! (System Settings > Privacy & Security > Screen Recording).

use bevy::prelude::*;
use bevy::color::palettes::css;
use bevy::text::{TextColor, TextFont};
use bevy_xcap::{NativeScreenshot, XCapPlugin, save_to_disk};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "bevy_xcap example".into(),
                resolution: bevy::window::WindowResolution::new(800, 600),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.15, 0.15, 0.2)))
        .add_plugins(XCapPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (screenshot_on_space, rotate_shapes))
        .run();
}

#[derive(Component)]
struct Rotating;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);

    commands.spawn((
        Mesh2d(meshes.add(Circle::new(80.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(css::CORAL))),
        Transform::from_xyz(-150.0, 50.0, 0.0),
        Rotating,
    ));

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(200.0, 120.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(css::DODGER_BLUE))),
        Transform::from_xyz(100.0, -30.0, 0.0),
        Rotating,
    ));

    commands.spawn((
        Mesh2d(meshes.add(Circle::new(40.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(css::LIMEGREEN))),
        Transform::from_xyz(0.0, 150.0, 0.0),
        Rotating,
    ));

    commands.spawn((
        Text2d::new("Press Space to capture"),
        TextFont {
            font_size: 30.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_xyz(0.0, -200.0, 1.0),
    ));
}

fn rotate_shapes(time: Res<Time>, mut query: Query<&mut Transform, With<Rotating>>) {
    for mut transform in &mut query {
        transform.rotate_z(time.delta_secs() * 0.5);
    }
}

fn screenshot_on_space(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    windows: Query<Entity, With<Window>>,
    mut counter: Local<u32>,
) {
    if !input.just_pressed(KeyCode::Space) {
        return;
    }

    let Ok(window_entity) = windows.single() else {
        warn!("No window found");
        return;
    };

    let path = format!("./native_screenshot_{}.png", *counter);
    *counter += 1;

    info!("Capturing native screenshot -> {path}");

    commands
        .spawn(NativeScreenshot::window(window_entity))
        .observe(save_to_disk(path));
}
