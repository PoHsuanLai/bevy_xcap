# bevy_xcap

[![CI](https://github.com/PoHsuanLai/bevy_xcap/actions/workflows/ci.yml/badge.svg)](https://github.com/PoHsuanLai/bevy_xcap/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/bevy_xcap.svg)](https://crates.io/crates/bevy_xcap)
[![docs.rs](https://docs.rs/bevy_xcap/badge.svg)](https://docs.rs/bevy_xcap)
[![License](https://img.shields.io/crates/l/bevy_xcap.svg)](https://github.com/PoHsuanLai/bevy_xcap#license)

Bevy plugin for native window pixel capture via [xcap](https://github.com/nashaofu/xcap).

Bevy's built-in `Screenshot` API only captures wgpu-rendered content. If your window contains native toolkit UI (Cocoa, Win32, GTK) or embedded third-party content, you need actual pixel capture of the OS window. `bevy_xcap` provides this with an API that mirrors Bevy's `Screenshot`.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
bevy_xcap = "0.1"
```

Capture a window:

```rust
use bevy::prelude::*;
use bevy_xcap::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(XCapPlugin)
        .add_systems(Update, capture_on_space)
        .run();
}

fn capture_on_space(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    windows: Query<Entity, With<Window>>,
) {
    if input.just_pressed(KeyCode::Space) {
        let window = windows.single().unwrap();
        commands
            .spawn(NativeScreenshot::window(window))
            .observe(save_to_disk("screenshot.png"));
    }
}
```

Or handle the pixels yourself:

```rust
commands
    .spawn(NativeScreenshot::window(window))
    .observe(|captured: On<NativeScreenshotCaptured>| {
        let c = &*captured;
        println!("Captured {}x{} ({} bytes)", c.width, c.height, c.rgba.len());
    });
```

Capture runs on a background thread — your app won't block.

## Platform notes

### macOS

You must grant **Screen Recording** permission to your terminal or app (System Settings > Privacy & Security > Screen Recording).

### Linux

The following system dependencies are required to compile:

**Debian/Ubuntu:**
```bash
apt-get install pkg-config libclang-dev libxcb1-dev libxrandr-dev libdbus-1-dev libpipewire-0.3-dev libwayland-dev libegl-dev
```

**Alpine:**
```bash
apk add pkgconf llvm19-dev clang19-dev libxcb-dev libxrandr-dev dbus-dev pipewire-dev wayland-dev mesa-dev
```

**Arch Linux:**
```bash
pacman -S base-devel clang libxcb libxrandr dbus libpipewire
```

## Bevy compatibility

| bevy_xcap | Bevy |
|-----------|------|
| 0.1       | 0.17 |

## Credits

Built on top of [xcap](https://github.com/nashaofu/xcap) by [@nashaofu](https://github.com/nashaofu) — cross-platform screen capture library for Rust.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
