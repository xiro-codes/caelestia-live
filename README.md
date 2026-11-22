# Caelestia Live Wallpaper & SVG Animation Engine

An extensible SVG live wallpaper animation engine built in Rust using [Bevy](https://bevyengine.org/) and [Rhai](https://rhai.rs/).

![demo](./assets/demo.gif)

## Overview

Caelestia Live decouples the animation engine binary from wallpaper assets. Wallpaper packages are stored as modular zip archives in `~/.config/caelestia-live/bgs/<name>.zip`. Each package contains the SVG artwork and a Rhai script driving the animations.

### Package Structure

Each wallpaper archive (`<name>.zip`) contains:
- `bg.svg`: The SVG artwork.
- `main.rhai`: The Rhai script controlling initial styles and runtime animations.

### Features

- **Extensible Rhai Scripting Engine**: Control any part of the SVG live using a simple, Rust-like scripting syntax.
- **`for_each` SVG Hooking API**: Target elements by ID or pattern:
  ```rust
  for_each("Star", |mut transform, mut scale| {
      transform.alpha = 0.5 + (time * 2.0).sin() * 0.5;
      transform.rot = (time * 0.2).sin() * 0.1;
      scale.set(1.0 + (time * 1.5).sin() * 0.15);
  });
  ```
- **Dynamic Color Syncing**: Analyzes an inputted wallpaper image or reads from Caelestia scheme JSON (`~/.local/state/caelestia/scheme.json`), making theme colors available to scripts via `transform.set_theme_color(...)` and the `theme()` object.
- **Resolution Independence**: Automatically scales the SVG to fit your screen resolution or aspect ratio.
- **Low Overhead**: Runs as a lightweight Wayland surface via `bevy_live_wallpaper`.

## Scripting API (Rhai)

Inside `main.rhai`:

```rust
fn init() {
    print("✨ Wallpaper initialized!");

    // Color elements using theme colors: background, color1 .. color7
    for_each("Wave1", |mut transform| transform.set_theme_color("color1"));
    for_each("Wave2", |mut transform| transform.set_theme_color("color2"));
    for_each("Star", |mut transform| transform.set_theme_color("color7"));
}

fn update(time, delta) {
    // Animate matching SVG elements
    for_each("Wave", |mut transform| {
        let i = transform.index as float;
        transform.x = transform.base_x + (time * 0.3 + i * 0.5).sin() * 50.0;
        transform.y = transform.base_y + (time * 0.5 + i * 0.5).cos() * 15.0;
    });

    for_each("Star", |mut transform, mut scale| {
        let i = transform.index as float;
        transform.alpha = 0.5 + (time * 1.5 + i).sin() * 0.5;
        transform.y = transform.base_y + (time * 0.5 + i).sin() * 2.0;
        scale.set(0.95 + (time * 2.0 + i).sin() * 0.1);
    });
}
```

### Element Properties & Methods

- `transform.id`: Element ID from SVG (e.g., `"Star1"`).
- `transform.index`: 0-based index of this element among matched elements.
- `transform.x`, `transform.y`, `transform.z`: Position.
- `transform.base_x`, `transform.base_y`: Initial SVG position.
- `transform.rotation` / `transform.rot`: Rotation in radians.
- `transform.alpha`: Opacity (0.0 to 1.0).
- `transform.set_position(x, y)` / `transform.translate(dx, dy)` / `transform.rotate(angle)`.
- `transform.set_theme_color(name)` / `transform.set_color(hex_or_name)`: Sets color (`"background"`, `"color1"`..`"color7"`, or `#hex`).
- `scale.x`, `scale.y` / `scale.set(uniform_scale)` / `scale.scale(factor)`.
- `theme().background`, `theme().color1`..`theme().color7` or `theme().color(index)`.

## CLI Usage

```bash
# List installed wallpaper packages in ~/.config/caelestia-live/bgs/
caelestia-live --list

# Run with a specific background package
caelestia-live --bg caelestia

# Run with color extraction from an image
caelestia-live --bg caelestia --wallpaper /path/to/image.jpg

# Package a local directory (containing bg.svg and main.rhai) into a wallpaper zip
caelestia-live --pack ./my_theme_folder/
```


## Credits

Special thanks to the creator of the [Caelestia Material Wave Wallpaper](https://www.figma.com/design/SI9495HhPhuREO6vQjDj1h/Caelestia-Material-Wave-Wallpaper?node-id=0-1&t=2DURRhAC8cQnNHwk-1) Figma design, which served as the beautiful underlying SVG artwork for this live wallpaper.

## License
MIT
