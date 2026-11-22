mod script;
mod wallpaper;

use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "caelestia-live",
    author,
    version,
    about = "SVG Live Wallpaper Animation Engine",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Option<CliCommand>,

    /// Background name or zip file path (checks ~/.config/caelestia-live/bgs/ by default)
    #[arg(short = 'b', long = "bg")]
    bg: Option<String>,

    /// Path to an image to extract an 8-color theme palette from
    #[arg(short = 'w', long = "wallpaper")]
    wallpaper: Option<String>,

    /// List all available wallpaper packages in ~/.config/caelestia-live/bgs/
    #[arg(short = 'l', long = "list")]
    list: bool,

    /// Pack a folder containing bg.svg into ~/.config/caelestia-live/bgs/<name>.zip
    #[arg(long = "pack")]
    pack: Option<std::path::PathBuf>,

    /// Output the visual SVG element tree of a wallpaper package
    #[arg(short = 't', long = "tree")]
    tree: Option<String>,

    /// Edit a wallpaper script with side-by-side readonly SVG tree, rezipping on save and quit
    #[arg(short = 'e', long = "edit")]
    edit: Option<String>,

    /// Install or re-initialize sample caelestia.zip in ~/.config/caelestia-live/bgs/
    #[arg(long = "install-default")]
    install_default: bool,
}

#[derive(clap::Subcommand, Debug)]
enum CliCommand {
    /// Output the visual SVG element tree of a wallpaper package
    Tree {
        /// Wallpaper name or path
        name: Option<String>,
    },
    /// Edit wallpaper script with a side-by-side readonly SVG tree reference, rezipping on save & quit
    Edit {
        /// Wallpaper name or path to edit
        name: Option<String>,
    },
    /// List all installed wallpaper packages in ~/.config/caelestia-live/bgs/
    List,
    /// Pack a folder containing bg.svg into ~/.config/caelestia-live/bgs/<name>.zip
    Pack {
        /// Directory containing bg.svg and optional main.rhai
        folder: std::path::PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    let bgs_dir = wallpaper::get_bgs_dir();

    if cli.list || matches!(cli.command, Some(CliCommand::List)) {
        let bgs = wallpaper::list_wallpapers();
        println!("Available wallpapers in {:?}:", bgs_dir);
        if bgs.is_empty() {
            println!("  (no .zip wallpaper packages found)");
        } else {
            for bg in bgs {
                let stem = bg.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
                println!("  - {}", stem);
            }
        }
        return;
    }

    let tree_target = match &cli.command {
        Some(CliCommand::Tree { name }) => name.clone().or(Some(String::new())),
        _ => cli.tree,
    };

    if let Some(target) = tree_target {
        let target_ref = if target.is_empty() {
            None
        } else {
            Some(target.as_str())
        };
        let path = match wallpaper::resolve_wallpaper_path(target_ref) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        };

        match wallpaper::get_wallpaper_svg_tree(&path) {
            Ok(tree) => {
                println!("{}", tree);
            }
            Err(e) => {
                eprintln!("Failed to generate SVG tree: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    let edit_target = match &cli.command {
        Some(CliCommand::Edit { name }) => name.clone().or(Some(String::new())),
        _ => cli.edit,
    };

    if let Some(target) = edit_target {
        let target_ref = if target.is_empty() {
            None
        } else {
            Some(target.as_str())
        };
        let path = match wallpaper::resolve_wallpaper_path(target_ref) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        };

        if let Err(e) = wallpaper::edit_wallpaper(&path) {
            eprintln!("Edit failed: {}", e);
            std::process::exit(1);
        }
        return;
    }

    let pack_folder = match &cli.command {
        Some(CliCommand::Pack { folder }) => Some(folder.clone()),
        _ => cli.pack,
    };

    if let Some(folder) = pack_folder {
        let folder_name = folder
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("wallpaper");
        let dest = bgs_dir.join(format!("{}.zip", folder_name));
        match wallpaper::pack_directory_to_zip(&folder, &dest) {
            Ok(()) => {
                println!(" Successfully packaged {:?} into {:?}", folder, dest);
            }
            Err(e) => {
                eprintln!("❌ Failed to package directory: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    if cli.install_default {
        let default_svg = std::path::Path::new("assets/shell.svg");
        let sample_zip = bgs_dir.join("caelestia.zip");
        if default_svg.exists() {
            let svg_bytes = std::fs::read(default_svg).expect("Failed to read assets/shell.svg");
            let rhai_script = include_bytes!("../templates/default.rhai");
            if let Err(e) =
                wallpaper::create_wallpaper_zip(&svg_bytes, Some(rhai_script), &sample_zip)
            {
                eprintln!("Failed to install default zip: {}", e);
                std::process::exit(1);
            }
            println!(" Installed sample wallpaper to {:?}", sample_zip);
        } else {
            eprintln!("assets/shell.svg not found to create sample wallpaper.");
        }
        return;
    }

    let bgs_dir = wallpaper::get_bgs_dir();
    let bg_path = match wallpaper::resolve_wallpaper_path(cli.bg.as_deref()) {
        Ok(path) => path,
        Err(err) => {
            let default_svg = std::path::Path::new("assets/shell.svg");
            let sample_zip = bgs_dir.join("caelestia.zip");
            if default_svg.exists() {
                eprintln!(
                    "No wallpaper package found in {:?}. Initializing sample package 'caelestia.zip'...",
                    bgs_dir
                );
                let svg_bytes =
                    std::fs::read(default_svg).expect("Failed to read assets/shell.svg");
                let rhai_script = include_bytes!("../templates/default.rhai");
                if let Err(e) =
                    wallpaper::create_wallpaper_zip(&svg_bytes, Some(rhai_script), &sample_zip)
                {
                    eprintln!("Failed to create sample zip: {}", e);
                    std::process::exit(1);
                }
                sample_zip
            } else {
                eprintln!("Error: {}", err);
                eprintln!(
                    "\nTo use caelestia-live, place a wallpaper zip in {:?}:",
                    bgs_dir
                );
                eprintln!("  ~/.config/caelestia-live/bgs/<name>.zip");
                eprintln!("The zip must contain:");
                eprintln!("  - bg.svg (required): the SVG artwork");
                eprintln!("  - main.rhai (optional): Rhai animation script");
                std::process::exit(1);
            }
        }
    };

    let wallpaper_pkg = match wallpaper::load_wallpaper(&bg_path) {
        Ok(pkg) => pkg,
        Err(err) => {
            eprintln!("Failed to load wallpaper package: {}", err);
            std::process::exit(1);
        }
    };

    println!("🎨 Loaded wallpaper package: {}", wallpaper_pkg.name);
    if let Some(script) = &wallpaper_pkg.script_path {
        println!("📜 Using animation script: {:?}", script);
    }

    let mut app = App::new();

    if let Some(wallpaper_path) = cli.wallpaper {
        let colors = extract_colors_from_wallpaper(&wallpaper_path);
        app.insert_resource(colors.clone());
        app.insert_resource(ClearColor(colors.background));
    } else if let Ok(home) = std::env::var("HOME") {
        let scheme_path = format!("{}/.local/state/caelestia/scheme.json", home);
        let colors = load_scheme_colors(&scheme_path);

        app.insert_resource(colors.clone());
        app.insert_resource(ClearColor(colors.background));

        app.insert_resource(SchemeWatcher {
            path: scheme_path.clone(),
            last_modified: std::fs::metadata(&scheme_path)
                .and_then(|m| m.modified())
                .unwrap_or_else(|_| std::time::SystemTime::now()),
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        });

        app.add_systems(Update, watch_scheme);
    } else {
        eprintln!("Warning: HOME environment variable not set. Using default colors.");
        let colors = SchemeColors::default();
        app.insert_resource(colors.clone());
        app.insert_resource(ClearColor(colors.background));
    }

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: bevy::window::ExitCondition::DontExit,
                ..default()
            })
            .set(AssetPlugin {
                file_path: wallpaper_pkg.extracted_dir.to_string_lossy().to_string(),
                meta_check: AssetMetaCheck::Never,
                ..default()
            }),
    )
    .add_plugins(bevy_live_wallpaper::LiveWallpaperPlugin::default())
    .add_plugins(bevy_ecs_svg::SvgPlugin)
    .add_plugins(script::ScriptPlugin {
        script_path: wallpaper_pkg.script_path.clone(),
    })
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (
            // animate_scene,
            // attach_animations_to_svg,
            scale_svg_to_window,
            propagate_svg_styles,
            apply_theme_colors,
        ),
    )
    .run();
}


#[derive(Resource, Clone)]
pub struct SchemeColors {
    pub background: Color,
    pub color1: Color,
    pub color2: Color,
    pub color3: Color,
    pub color4: Color,
    pub color5: Color,
    pub color6: Color,
    pub color7: Color,
}

#[derive(Debug,Clone, Copy, PartialEq, Eq)]
enum ThemeColor {
    Background,
    Color1,
    Color2,
    Color3,
    Color4,
    Color5,
    Color6,
    Color7,
}

impl SchemeColors {
    fn get(&self, theme: ThemeColor) -> Color {
        match theme {
            ThemeColor::Background => self.background,
            ThemeColor::Color1 => self.color1,
            ThemeColor::Color2 => self.color2,
            ThemeColor::Color3 => self.color3,
            ThemeColor::Color4 => self.color4,
            ThemeColor::Color5 => self.color5,
            ThemeColor::Color6 => self.color6,
            ThemeColor::Color7 => self.color7,
        }
    }
}

impl Default for SchemeColors {
    fn default() -> Self {
        Self {
            background: Color::srgb(0.05, 0.05, 0.1),
            color1: Color::srgb(0.1, 0.05, 0.2),
            color2: Color::srgb(0.1, 0.15, 0.4),
            color3: Color::srgb(0.8, 0.5, 0.7),
            color4: Color::srgb(0.4, 0.1, 0.5),
            color5: Color::srgb(0.15, 0.1, 0.3),
            color6: Color::srgb(0.5, 0.3, 0.6),
            color7: Color::WHITE,
        }
    }
}

fn extract_colors_from_wallpaper(path: &str) -> SchemeColors {
    let mut colors = SchemeColors::default();

    if let Ok(bytes) = std::fs::read(path) {
        if let Ok(dyn_img) = image::load_from_memory(&bytes) {
            let img_resized = dyn_img.resize_exact(32, 32, image::imageops::FilterType::Triangle);
            let mut pixels: Vec<_> = img_resized.to_rgba8().pixels().map(|p| p.0).collect();

            pixels.sort_by(|a, b| {
                let max_a = a[0].max(a[1]).max(a[2]) as f32;
                let min_a = a[0].min(a[1]).min(a[2]) as f32;
                let sat_a = if max_a == 0.0 {
                    0.0
                } else {
                    (max_a - min_a) / max_a
                };

                let max_b = b[0].max(b[1]).max(b[2]) as f32;
                let min_b = b[0].min(b[1]).min(b[2]) as f32;
                let sat_b = if max_b == 0.0 {
                    0.0
                } else {
                    (max_b - min_b) / max_b
                };

                sat_b
                    .partial_cmp(&sat_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let mut chosen_colors = Vec::new();
            let mut target_dist = 0.5;

            while chosen_colors.len() < 8 && target_dist >= 0.0 {
                for p in &pixels {
                    let color = Color::srgba(
                        p[0] as f32 / 255.0,
                        p[1] as f32 / 255.0,
                        p[2] as f32 / 255.0,
                        1.0,
                    );

                    let mut similar = false;
                    for c in &chosen_colors {
                        let c: Color = *c;
                        let srgba1 = color.to_srgba();
                        let srgba2 = c.to_srgba();
                        let dist = (srgba1.red - srgba2.red).abs()
                            + (srgba1.green - srgba2.green).abs()
                            + (srgba1.blue - srgba2.blue).abs();
                        if dist < target_dist {
                            similar = true;
                            break;
                        }
                    }

                    if !similar {
                        chosen_colors.push(color);
                        if chosen_colors.len() == 8 {
                            break;
                        }
                    }
                }
                target_dist -= 0.05;
            }

            if chosen_colors.len() == 8 {
                colors.background = chosen_colors[0];
                colors.color1 = chosen_colors[1];
                colors.color2 = chosen_colors[2];
                colors.color3 = chosen_colors[3];
                colors.color4 = chosen_colors[4];
                colors.color5 = chosen_colors[5];
                colors.color6 = chosen_colors[6];
                colors.color7 = chosen_colors[7];
            } else {
                for (i, c) in chosen_colors.iter().enumerate() {
                    match i {
                        0 => colors.background = *c,
                        1 => colors.color1 = *c,
                        2 => colors.color2 = *c,
                        3 => colors.color3 = *c,
                        4 => colors.color4 = *c,
                        5 => colors.color5 = *c,
                        6 => colors.color6 = *c,
                        7 => colors.color7 = *c,
                        _ => {}
                    }
                }
            }
        }
    }

    colors
}

fn load_scheme_colors(path: &str) -> SchemeColors {
    let mut colors = SchemeColors::default();

    if let Ok(content) = std::fs::read_to_string(path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            let get_col = |key: &str, fallback: Color| -> Color {
                if let Some(hex) = json
                    .get("colours")
                    .and_then(|c| c.get(key))
                    .and_then(|s| s.as_str())
                {
                    if let Ok(color) = bevy::color::Srgba::hex(hex) {
                        return Color::from(color);
                    }
                }
                fallback
            };

            colors.background = get_col("crust", colors.background);
            colors.color1 = get_col("base", colors.color1);
            colors.color2 = get_col("blue", colors.color2);
            colors.color3 = get_col("pink", colors.color3);
            colors.color4 = get_col("mauve", colors.color4);
            colors.color5 = get_col("sapphire", colors.color5);
            colors.color6 = get_col("lavender", colors.color6);
            colors.color7 = get_col("text", colors.color7);
        }
    }

    colors
}

#[derive(Resource)]
struct SchemeWatcher {
    path: String,
    last_modified: std::time::SystemTime,
    timer: Timer,
}

fn watch_scheme(
    time: Res<Time>,
    mut watcher: ResMut<SchemeWatcher>,
    mut clear_color: ResMut<ClearColor>,
    mut colors_res: ResMut<SchemeColors>,
) {
    if watcher.timer.tick(time.delta()).just_finished() {
        if let Ok(metadata) = std::fs::metadata(&watcher.path) {
            if let Ok(modified) = metadata.modified() {
                if modified > watcher.last_modified {
                    watcher.last_modified = modified;
                    let new_colors = load_scheme_colors(&watcher.path);

                    clear_color.0 = new_colors.background;
                    *colors_res = new_colors;
                }
            }
        }
    }
}

fn apply_theme_colors(
    colors: Res<SchemeColors>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    query: Query<(&ActiveThemeColor, &MeshMaterial2d<ColorMaterial>)>,
) {
    // Always apply theme colours each frame; the SVG renderer may reset material colours.
    for (theme_color, mat_handle) in query.iter() {
        if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
            info!("Applying theme color: {:?}", theme_color.0);
            let mut new_c = colors.get(theme_color.0);
            // Preserve existing alpha (e.g., for twinkling stars)
            let current_alpha = mat.color.alpha();
            new_c.set_alpha(current_alpha);
            mat.color = new_c;
        }
    }
}


fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((Camera2d, bevy_live_wallpaper::LiveWallpaperCamera));

    // Spawn the SVG from the active wallpaper package
    commands.spawn((
        bevy_ecs_svg::SpawnSvg {
            document: asset_server.load("bg.svg"),
        },
        Transform::default(),
        GlobalTransform::default(),
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
    ));
}

#[derive(Component)]
struct SvgStyle {
    color: Option<ThemeColor>,
}


#[derive(Component, Debug)]
struct ActiveThemeColor(ThemeColor);

fn propagate_svg_styles(
    mut commands: Commands,
    roots: Query<(Entity, &SvgStyle), Added<SvgStyle>>,
    children_q: Query<&Children>,
    paths: Query<&MeshMaterial2d<ColorMaterial>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    colors_res: Res<SchemeColors>,
) {
    for (root_entity, style) in roots.iter() {
        let mut queue = vec![root_entity];
        while let Some(entity) = queue.pop() {
            if let Ok(mat_handle) = paths.get(entity) {
                if let Some(color_type) = style.color {
                    if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
                        mat.color = colors_res.get(color_type);
                        commands.entity(entity).insert(ActiveThemeColor(color_type));
                    }
                }
            }

            if let Ok(children) = children_q.get(entity) {
                for child in children.iter() {
                    queue.push(child);
                }
            }
        }
    }
}



fn scale_svg_to_window(
    mut query: Query<&mut Transform, With<bevy_ecs_svg::SvgHierarchyLoaded>>,
    spawn_query: Query<&bevy_ecs_svg::SpawnSvg>,
    svgs: Res<Assets<bevy_ecs_svg::SvgDocument>>,
    cameras: Query<&Camera, With<Camera2d>>,
) {
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    let Some(viewport_size) = camera.logical_viewport_size() else {
        return;
    };

    let cam_w = viewport_size.x;
    let cam_h = viewport_size.y;

    if cam_w == 0.0 || cam_h == 0.0 {
        return;
    }

    // Determine SVG dimensions from the loaded SVG asset or fallback
    let (svg_w, svg_h) = if let Some(spawn) = spawn_query.iter().next() {
        if let Some(svg_doc) = svgs.get(&spawn.document) {
            let size = svg_doc.tree.size();
            (size.width(), size.height())
        } else {
            (5120.0, 2160.0)
        }
    } else {
        (5120.0, 2160.0)
    };

    // Stretch to fit the resolution perfectly
    let scale_x = cam_w / svg_w;
    let scale_y = cam_h / svg_h;

    for mut transform in query.iter_mut() {
        transform.scale = Vec3::new(scale_x, scale_y, 1.0);
    }
}
