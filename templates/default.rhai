// Caelestia Live Wallpaper - Rhai Script
// Matches and exceeds the original built-in procedural animation.

fn init() {
    print("✨ Caelestia Live Wallpaper Engine initialized with Rhai!");

    // Color theme assignments using simple color names: color1 .. color7, background
    for_each("Star", |mut transform| {
        transform.set_theme_color("color7");
    });

    for_each("Logo", |mut transform| {
        transform.set_theme_color("color7");
    });

    for_each("Wave1", |mut transform| transform.set_theme_color("color1"));
    for_each("Wave2", |mut transform| transform.set_theme_color("color2"));
    for_each("Wave3", |mut transform| transform.set_theme_color("color5"));
    for_each("Wave4", |mut transform| transform.set_theme_color("color6"));

    for_each("Cloud", |mut transform| {
        if transform.index % 2 == 0 {
            transform.set_theme_color("color3");
        } else {
            transform.set_theme_color("color4");
        }
    });
}

fn update(
    time,
    delta,
) {
    // 1. Waves ripple: shifting X and Y smoothly
    for_each("Wave", |mut transform| {
        let i = transform.index as float;
        let base_phase = i * 0.5;
        let speed = 0.2 + (i % 0.3);

        transform.x = transform.base_x + (time * speed + base_phase).sin() * 50.0;
        transform.y = transform.base_y + (time * speed * 1.5 + base_phase).cos() * 15.0;
    });

    // 2. Stars twinkle (opacity), rotate, and bob
    for_each("Star", |mut transform, mut scale| {
        let i = transform.index as float;
        let speed = 1.5 + (i % 2.0);
        let phase = i;

        transform.alpha = 0.5 + (time * speed * 0.4 + phase).sin() * 0.5;
        transform.rot = (time * speed * 0.2 + phase).sin() * 0.1;
        transform.y = transform.base_y + (time * speed * 0.3 + phase).sin() * 1.5;
        scale.set(0.95 + (time * speed * 0.5 + phase).sin() * 0.1);
    });

    // 3. Logo bobs and wobbles
    for_each("Logo", |mut transform, _, _| {
        let bob_speed = 0.8;
        transform.y = transform.base_y + (time * bob_speed).sin() * 8.0;
        transform.rot = (time * bob_speed * 0.5).sin() * 0.05;
    });

    // 4. Clouds rotate gently
    for_each("Cloud", |mut transform, _, _| {
        let i = transform.index as float;
        let speed = 0.1 + (i % 0.1);
        let phase = i;
        transform.rot = (time * speed + phase).sin() * 0.02;
    });
}
