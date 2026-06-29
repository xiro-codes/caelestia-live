use bevy::prelude::*;

fn main() {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/tod".to_string());
    let scheme_path = format!("{}/.local/state/caelestia/scheme.json", home);
    let mut bg_color = Color::srgb(0.05, 0.05, 0.1);
    
    if let Ok(content) = std::fs::read_to_string(&scheme_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(hex) = json.get("colours").and_then(|c| c.get("crust")).and_then(|s| s.as_str()) {
                if let Ok(c) = bevy::color::Srgba::hex(hex) {
                    bg_color = Color::from(c);
                }
            }
        }
    }

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: None,
            exit_condition: bevy::window::ExitCondition::DontExit,
            ..default()
        }))
        .add_plugins(bevy_live_wallpaper::LiveWallpaperPlugin::default())
        .insert_resource(ClearColor(bg_color)) // Scheme background
        .add_systems(Startup, setup)
        .add_systems(Update, animate_scene)
        .run();
}

#[derive(Component)]
struct Wave {
    base_phase: f32,
    speed: f32,
}

#[derive(Component)]
struct StarTwinkle {
    base_scale: f32,
    speed: f32,
    phase: f32,
}

#[derive(Component)]
struct PlanetPart {
    base_y: f32,
    base_rotation: f32,
    bob_speed: f32,
    spin_speed: f32,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((Camera2d, bevy_live_wallpaper::LiveWallpaperCamera));

    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/tod".to_string());
    let scheme_path = format!("{}/.local/state/caelestia/scheme.json", home);
    let json = std::fs::read_to_string(&scheme_path)
        .ok()
        .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    let get_col = |key: &str, fallback: Color| -> Color {
        if let Some(hex) = json.get("colours").and_then(|c| c.get(key)).and_then(|s| s.as_str()) {
            if let Ok(color) = bevy::color::Srgba::hex(hex) {
                return Color::from(color);
            }
        }
        fallback
    };

    // Colors mapped to scheme.json
    let deep_violet = get_col("base", Color::srgb(0.1, 0.05, 0.2));
    let royal_blue = get_col("blue", Color::srgb(0.1, 0.15, 0.4));
    let pastel_pink = get_col("pink", Color::srgb(0.8, 0.5, 0.7));
    let vibrant_purple = get_col("mauve", Color::srgb(0.4, 0.1, 0.5));
    let indigo = get_col("sapphire", Color::srgb(0.15, 0.1, 0.3));
    let light_purple = get_col("lavender", Color::srgb(0.5, 0.3, 0.6));
    let star_white = get_col("text", Color::WHITE);
    let shadow_color = Color::srgba(0.0, 0.0, 0.0, 0.6);

    let screen_w = 4000.0;
    let bottom_y = -2000.0;

    // Background Waves (from back to front)
    let wave_data = vec![
        (deep_violet, 800.0, 150.0, 1.5, 0.0, 0.2, -10.0),
        (royal_blue, 400.0, 200.0, 2.0, 1.0, 0.3, -9.0),
        (pastel_pink, 0.0, 100.0, 1.0, 2.0, 0.15, -8.0),
        (vibrant_purple, -300.0, 180.0, 2.5, 0.5, 0.25, -7.0),
        (indigo, -600.0, 250.0, 1.2, 1.5, 0.2, -6.0),
        (light_purple, -900.0, 120.0, 1.8, 0.8, 0.1, -5.0),
    ];

    for (color, base_y, amp, freq, phase, speed, z) in wave_data {
        commands.spawn((
            Mesh2d(meshes.add(create_wave_mesh(screen_w, base_y, amp, freq, phase, bottom_y))),
            MeshMaterial2d(materials.add(ColorMaterial::from(color))),
            Transform::from_xyz(0.0, 0.0, z),
            Wave {
                base_phase: phase,
                speed,
            },
        ));
    }

    // Central Planet Logo
    let planet_z = 0.0;
    
    let top_crescent_mesh = meshes.add(create_crescent_mesh(true));
    let bot_crescent_mesh = meshes.add(create_crescent_mesh(false));
    let ring_mesh = meshes.add(create_custom_ring_mesh());
    
    let shadow_mat = materials.add(ColorMaterial::from(shadow_color));
    let pink_mat = materials.add(ColorMaterial::from(pastel_pink));
    let white_mat = materials.add(ColorMaterial::from(star_white));

    // Shadows
    commands.spawn((
        Mesh2d(top_crescent_mesh.clone()),
        MeshMaterial2d(shadow_mat.clone()),
        Transform::from_xyz(15.0, -15.0, planet_z - 0.5),
        PlanetPart { base_y: -15.0, base_rotation: 0.0, bob_speed: 0.8, spin_speed: 0.05 },
    ));
    commands.spawn((
        Mesh2d(bot_crescent_mesh.clone()),
        MeshMaterial2d(shadow_mat.clone()),
        Transform::from_xyz(15.0, -15.0, planet_z - 0.5),
        PlanetPart { base_y: -15.0, base_rotation: 0.0, bob_speed: 0.8, spin_speed: 0.05 },
    ));
    commands.spawn((
        Mesh2d(ring_mesh.clone()),
        MeshMaterial2d(shadow_mat),
        Transform::from_xyz(15.0, -15.0, planet_z - 0.5),
        PlanetPart { base_y: -15.0, base_rotation: 0.0, bob_speed: 0.8, spin_speed: 0.05 },
    ));

    // Top Pink Crescent
    commands.spawn((
        Mesh2d(top_crescent_mesh),
        MeshMaterial2d(pink_mat.clone()),
        Transform::from_xyz(0.0, 0.0, planet_z),
        PlanetPart { base_y: 0.0, base_rotation: 0.0, bob_speed: 0.8, spin_speed: 0.05 },
    ));

    // Bottom Pink Crescent
    commands.spawn((
        Mesh2d(bot_crescent_mesh),
        MeshMaterial2d(pink_mat),
        Transform::from_xyz(0.0, 0.0, planet_z),
        PlanetPart { base_y: 0.0, base_rotation: 0.0, bob_speed: 0.8, spin_speed: 0.05 },
    ));

    // White Ring (Tilted C-shape)
    commands.spawn((
        Mesh2d(ring_mesh),
        MeshMaterial2d(white_mat),
        Transform::from_xyz(0.0, 0.0, planet_z + 1.0),
        PlanetPart { base_y: 0.0, base_rotation: 0.0, bob_speed: 0.8, spin_speed: 0.05 },
    ));

    // Stars
    let stars = vec![
        // Left side
        (star_white, 40.0, -300.0, -200.0, 2.0, 0.0),
        (pastel_pink, 20.0, -500.0, 100.0, 1.5, 1.0),
        (pastel_pink, 25.0, -250.0, 300.0, 3.0, 2.0),
        (pastel_pink, 10.0, -150.0, 150.0, 2.5, 3.0),
        // Right side
        (star_white, 50.0, 350.0, 250.0, 1.8, 0.5),
        (pastel_pink, 15.0, 180.0, 220.0, 4.0, 1.5),
        (pastel_pink, 12.0, 220.0, 280.0, 3.5, 2.5),
        (pastel_pink, 20.0, 450.0, -100.0, 2.2, 0.8),
        (pastel_pink, 18.0, 300.0, -350.0, 1.6, 1.2),
    ];

    let star_shadow_mat = materials.add(ColorMaterial::from(shadow_color));

    for (color, radius, x, y, speed, phase) in stars {
        let star_mesh = meshes.add(create_star_mesh(radius));
        let star_mat = materials.add(ColorMaterial::from(color));
        
        // Star Shadow
        commands.spawn((
            Mesh2d(star_mesh.clone()),
            MeshMaterial2d(star_shadow_mat.clone()),
            Transform::from_xyz(x + 5.0, y - 5.0, planet_z + 1.9),
            StarTwinkle { base_scale: 1.0, speed, phase },
        ));

        // Actual Star
        commands.spawn((
            Mesh2d(star_mesh),
            MeshMaterial2d(star_mat),
            Transform::from_xyz(x, y, planet_z + 2.0),
            StarTwinkle { base_scale: 1.0, speed, phase },
        ));
    }

    // Clouds in the corners
    let cloud_rgba = match light_purple {
        Color::Srgba(c) => c,
        _ => bevy::color::Srgba::new(0.8, 0.5, 0.8, 1.0),
    };
    let cloud_mat = materials.add(ColorMaterial::from(Color::srgba(cloud_rgba.red, cloud_rgba.green, cloud_rgba.blue, 0.15)));

    let cloud_clusters = vec![
        (-1200.0, 600.0, vec![(0.0, 0.0, 120.0), (80.0, -40.0, 100.0), (40.0, -80.0, 90.0), (140.0, -20.0, 70.0)]),
        (1200.0, 600.0, vec![(0.0, 0.0, 130.0), (-90.0, -50.0, 110.0), (-40.0, -100.0, 90.0), (-160.0, -30.0, 60.0)]),
        (-1200.0, -600.0, vec![(0.0, 0.0, 140.0), (100.0, 40.0, 110.0), (40.0, 100.0, 95.0), (160.0, 20.0, 75.0)]),
        (1200.0, -600.0, vec![(0.0, 0.0, 150.0), (-110.0, 50.0, 120.0), (-50.0, 110.0, 100.0), (-180.0, 30.0, 80.0)]),
    ];

    for (bx, by, circles) in cloud_clusters {
        for (dx, dy, radius) in circles {
            commands.spawn((
                Mesh2d(meshes.add(create_ellipse_mesh(radius, radius))),
                MeshMaterial2d(cloud_mat.clone()),
                Transform::from_xyz(bx + dx, by + dy, planet_z - 1.0),
            ));
        }
    }
}

fn animate_scene(
    time: Res<Time>,
    mut waves: Query<(&mut Transform, &Wave), (Without<StarTwinkle>, Without<PlanetPart>)>,
    mut stars: Query<(&mut Transform, &StarTwinkle), (Without<Wave>, Without<PlanetPart>)>,
    mut planets: Query<(&mut Transform, &PlanetPart), (Without<Wave>, Without<StarTwinkle>)>,
) {
    let t = time.elapsed_secs();

    // Animate waves by slowly shifting them horizontally
    for (mut transform, wave) in waves.iter_mut() {
        let shift = (t * wave.speed + wave.base_phase).sin() * 50.0;
        transform.translation.x = shift;
    }

    // Twinkle and spin stars
    for (mut transform, star) in stars.iter_mut() {
        let scale = star.base_scale + (t * star.speed + star.phase).sin() * 0.2;
        transform.scale = Vec3::splat(scale);
        transform.rotation = Quat::from_rotation_z(t * 0.5 * star.speed);
    }

    // Bob and spin planet parts
    for (mut transform, planet) in planets.iter_mut() {
        transform.translation.y = planet.base_y + (t * planet.bob_speed).sin() * 15.0;
        transform.rotation = Quat::from_rotation_z(planet.base_rotation + t * planet.spin_speed);
    }
}

// Custom Mesh Generators

fn create_star_mesh(radius: f32) -> Mesh {
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    let segments = 32;

    positions.push([0.0, 0.0, 0.0]); // Center

    for i in 0..segments {
        let t = (i as f32) * std::f32::consts::TAU / (segments as f32);
        let x = radius * t.cos().powi(3);
        let y = radius * t.sin().powi(3);
        positions.push([x, y, 0.0]);
    }

    for i in 0..segments {
        let a = 0;
        let b = i + 1;
        let c = if i == segments - 1 { 1 } else { i + 2 };
        indices.push(a);
        indices.push(b);
        indices.push(c);
    }

    Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_indices(bevy::mesh::Indices::U32(indices))
}

fn create_crescent_mesh(is_top: bool) -> Mesh {
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    let segments = 200;
    let r = 80.0_f32;
    
    let a = 160.0_f32;
    let b = 50.0_f32;
    let alpha = 22.0_f32.to_radians();
    
    let big_a = alpha.sin().powi(2) / a.powi(2) + alpha.cos().powi(2) / b.powi(2);
    
    for i in 0..=segments {
        let x = -r + 2.0 * r * (i as f32) / (segments as f32);
        let y_circ = (r.powi(2) - x.powi(2)).max(0.0).sqrt();
        
        let big_b = 2.0 * x * alpha.sin() * alpha.cos() * (1.0 / a.powi(2) - 1.0 / b.powi(2));
        let big_c = x.powi(2) * alpha.cos().powi(2) / a.powi(2) + x.powi(2) * alpha.sin().powi(2) / b.powi(2) - 1.0;
        
        let disc = big_b.powi(2) - 4.0 * big_a * big_c;
        let mut y_ell_top = 0.0;
        let mut y_ell_bot = 0.0;
        
        if disc >= 0.0 {
            y_ell_top = (-big_b + disc.sqrt()) / (2.0 * big_a);
            y_ell_bot = (-big_b - disc.sqrt()) / (2.0 * big_a);
        }
        
        if is_top {
            if y_ell_top <= y_circ {
                positions.push([x, y_ell_top, 0.0]);
                positions.push([x, y_circ, 0.0]);
            } else {
                positions.push([x, y_circ, 0.0]);
                positions.push([x, y_circ, 0.0]);
            }
        } else {
            if -y_circ <= y_ell_bot {
                positions.push([x, -y_circ, 0.0]);
                positions.push([x, y_ell_bot, 0.0]);
            } else {
                positions.push([x, -y_circ, 0.0]);
                positions.push([x, -y_circ, 0.0]);
            }
        }
    }
    
    for i in 0..segments {
        let p1 = i * 2;
        let p2 = i * 2 + 1;
        let p3 = i * 2 + 2;
        let p4 = i * 2 + 3;

        indices.extend_from_slice(&[p1, p2, p3]);
        indices.extend_from_slice(&[p3, p2, p4]);
    }
    
    Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_indices(bevy::mesh::Indices::U32(indices))
}

fn create_custom_ring_mesh() -> Mesh {
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    let segments = 200;
    
    let a_in = 130.0;
    let b_in = 20.0;
    let a_out = 150.0;
    let b_out = 40.0;
    let alpha = 22.0_f32.to_radians();
    
    let t_start = 20.0_f32.to_radians();
    let t_end = 340.0_f32.to_radians();
    
    for i in 0..=segments {
        let t = t_start + (t_end - t_start) * (i as f32) / (segments as f32);
        
        let u_in = a_in * t.cos();
        let v_in = b_in * t.sin();
        let mut u_out = a_out * t.cos();
        let v_out = b_out * t.sin();
        
        let dist_to_pi = (t - std::f32::consts::PI).abs();
        if dist_to_pi < 0.5 {
            let wing = (1.0 - dist_to_pi / 0.5).powi(2) * 80.0;
            u_out -= wing;
        }
        
        let x_in = u_in * alpha.cos() - v_in * alpha.sin();
        let y_in = u_in * alpha.sin() + v_in * alpha.cos();
        let x_out = u_out * alpha.cos() - v_out * alpha.sin();
        let y_out = u_out * alpha.sin() + v_out * alpha.cos();
        
        positions.push([x_out, y_out, 0.0]);
        positions.push([x_in, y_in, 0.0]);
    }
    
    for i in 0..segments {
        let p1 = i * 2;
        let p2 = i * 2 + 1;
        let p3 = i * 2 + 2;
        let p4 = i * 2 + 3;

        indices.extend_from_slice(&[p1, p2, p3]);
        indices.extend_from_slice(&[p3, p2, p4]);
    }
    
    Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_indices(bevy::mesh::Indices::U32(indices))
}

fn create_wave_mesh(width: f32, base_y: f32, amplitude: f32, frequency: f32, phase: f32, bottom_y: f32) -> Mesh {
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    let segments = 100;
    
    for i in 0..=segments {
        let normalized_x = (i as f32) / (segments as f32);
        let x = -width / 2.0 + normalized_x * width;
        let wave_y = base_y + amplitude * (frequency * normalized_x * std::f32::consts::TAU + phase).sin();
        
        positions.push([x, wave_y, 0.0]);
        positions.push([x, bottom_y, 0.0]);
    }

    for i in 0..segments {
        let p1 = i * 2;
        let p2 = i * 2 + 1;
        let p3 = i * 2 + 2;
        let p4 = i * 2 + 3;

        indices.extend_from_slice(&[p1, p2, p3]);
        indices.extend_from_slice(&[p3, p2, p4]);
    }

    Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_indices(bevy::mesh::Indices::U32(indices))
}

fn create_ellipse_mesh(rx: f32, ry: f32) -> Mesh {
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    let segments = 32;

    positions.push([0.0, 0.0, 0.0]); // Center

    for i in 0..segments {
        let t = (i as f32) * std::f32::consts::TAU / (segments as f32);
        positions.push([rx * t.cos(), ry * t.sin(), 0.0]);
    }

    for i in 0..segments {
        let a = 0;
        let b = i + 1;
        let c = if i == segments - 1 { 1 } else { i + 2 };
        indices.push(a);
        indices.push(b);
        indices.push(c);
    }

    Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_indices(bevy::mesh::Indices::U32(indices))
}
