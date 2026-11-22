use bevy::color::Srgba;
use bevy::prelude::*;
use rhai::{Dynamic, Engine, EvalAltResult, FnPtr, NativeCallContext, Scope, AST};
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crate::SchemeColors;

#[derive(Component, Clone, Copy, Debug)]
pub struct BaseTransform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

#[derive(Debug, Clone, Default)]
pub struct NodeTransformData {
    pub entity: Option<Entity>,
    pub id: String,
    pub index: i64,
    pub base_x: f64,
    pub base_y: f64,
    pub base_z: f64,
    pub base_rotation: f64,
    pub base_scale_x: f64,
    pub base_scale_y: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub rotation: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    pub alpha: f64,
    pub color_request: Option<ColorRequest>,
    pub modified_transform: bool,
    pub modified_alpha: bool,
    pub modified_color: bool,
}

#[derive(Debug, Clone)]
pub enum ColorRequest {
    Theme(String),
    Custom(Color),
}

#[derive(Debug, Clone)]
pub struct ScriptTransform {
    pub(crate) data: Arc<RwLock<NodeTransformData>>,
}

impl ScriptTransform {
    pub fn get_id(&mut self) -> String {
        self.data.read().unwrap().id.clone()
    }
    pub fn get_index(&mut self) -> i64 {
        self.data.read().unwrap().index
    }
    pub fn get_base_x(&mut self) -> f64 {
        self.data.read().unwrap().base_x
    }
    pub fn get_base_y(&mut self) -> f64 {
        self.data.read().unwrap().base_y
    }
    pub fn get_base_z(&mut self) -> f64 {
        self.data.read().unwrap().base_z
    }
    pub fn get_base_rot(&mut self) -> f64 {
        self.data.read().unwrap().base_rotation
    }
    pub fn get_base_scale_x(&mut self) -> f64 {
        self.data.read().unwrap().base_scale_x
    }
    pub fn get_base_scale_y(&mut self) -> f64 {
        self.data.read().unwrap().base_scale_y
    }
    pub fn get_x(&mut self) -> f64 {
        self.data.read().unwrap().x
    }
    pub fn set_x(&mut self, val: f64) {
        let mut d = self.data.write().unwrap();
        d.x = val;
        d.modified_transform = true;
    }
    pub fn get_y(&mut self) -> f64 {
        self.data.read().unwrap().y
    }
    pub fn set_y(&mut self, val: f64) {
        let mut d = self.data.write().unwrap();
        d.y = val;
        d.modified_transform = true;
    }
    pub fn get_z(&mut self) -> f64 {
        self.data.read().unwrap().z
    }
    pub fn set_z(&mut self, val: f64) {
        let mut d = self.data.write().unwrap();
        d.z = val;
        d.modified_transform = true;
    }
    pub fn get_rotation(&mut self) -> f64 {
        self.data.read().unwrap().rotation
    }
    pub fn set_rotation(&mut self, val: f64) {
        let mut d = self.data.write().unwrap();
        d.rotation = val;
        d.modified_transform = true;
    }
    pub fn get_alpha(&mut self) -> f64 {
        self.data.read().unwrap().alpha
    }
    pub fn set_alpha(&mut self, val: f64) {
        let mut d = self.data.write().unwrap();
        d.alpha = val.clamp(0.0, 1.0);
        d.modified_alpha = true;
    }
    pub fn set_position(&mut self, x: f64, y: f64) {
        let mut d = self.data.write().unwrap();
        d.x = x;
        d.y = y;
        d.modified_transform = true;
    }
    pub fn translate(&mut self, dx: f64, dy: f64) {
        let mut d = self.data.write().unwrap();
        d.x += dx;
        d.y += dy;
        d.modified_transform = true;
    }
    pub fn rotate(&mut self, angle: f64) {
        let mut d = self.data.write().unwrap();
        d.rotation += angle;
        d.modified_transform = true;
    }
    pub fn set_theme_color(&mut self, theme_color_name: &str) {
        let mut d = self.data.write().unwrap();
        d.color_request = Some(ColorRequest::Theme(theme_color_name.to_string()));
        d.modified_color = true;
    }
    pub fn set_color(&mut self, hex_color: &str) {
        let mut d = self.data.write().unwrap();
        if let Ok(srgba) = Srgba::hex(hex_color) {
            d.color_request = Some(ColorRequest::Custom(Color::from(srgba)));
            d.modified_color = true;
        } else {
            // Check if it's a theme color name
            d.color_request = Some(ColorRequest::Theme(hex_color.to_string()));
            d.modified_color = true;
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScriptScale {
    pub(crate) data: Arc<RwLock<NodeTransformData>>,
}

impl ScriptScale {
    pub fn get_x(&mut self) -> f64 {
        self.data.read().unwrap().scale_x
    }
    pub fn set_x(&mut self, val: f64) {
        let mut d = self.data.write().unwrap();
        d.scale_x = val;
        d.modified_transform = true;
    }
    pub fn get_y(&mut self) -> f64 {
        self.data.read().unwrap().scale_y
    }
    pub fn set_y(&mut self, val: f64) {
        let mut d = self.data.write().unwrap();
        d.scale_y = val;
        d.modified_transform = true;
    }
    pub fn set_uniform(&mut self, val: f64) {
        let mut d = self.data.write().unwrap();
        d.scale_x = val;
        d.scale_y = val;
        d.modified_transform = true;
    }
    pub fn scale(&mut self, factor: f64) {
        let mut d = self.data.write().unwrap();
        d.scale_x *= factor;
        d.scale_y *= factor;
        d.modified_transform = true;
    }
}

#[derive(Debug, Clone, Default)]
pub struct ScriptTheme {
    pub background: String,
    pub color1: String,
    pub color2: String,
    pub color3: String,
    pub color4: String,
    pub color5: String,
    pub color6: String,
    pub color7: String,
}

thread_local! {
    static ACTIVE_FRAME_NODES: RefCell<Vec<Arc<RwLock<NodeTransformData>>>> = const { RefCell::new(Vec::new()) };
    static CURRENT_TIME: RefCell<f64> = const { RefCell::new(0.0) };
    static CURRENT_DELTA: RefCell<f64> = const { RefCell::new(0.0) };
    static CURRENT_THEME: RefCell<ScriptTheme> = const { RefCell::new(ScriptTheme {
        background: String::new(),
        color1: String::new(),
        color2: String::new(),
        color3: String::new(),
        color4: String::new(),
        color5: String::new(),
        color6: String::new(),
        color7: String::new(),
    })};
}

#[derive(Resource)]
pub struct ScriptEngine {
    pub engine: Engine,
    pub ast: Option<AST>,
    pub scope: Scope<'static>,
    pub has_update: bool,
}

pub struct ScriptPlugin {
    pub script_path: Option<PathBuf>,
}

pub fn color_to_hex(c: Color) -> String {
    let srgba = c.to_srgba();
    format!(
        "#{:02x}{:02x}{:02x}",
        (srgba.red * 255.0).round() as u8,
        (srgba.green * 255.0).round() as u8,
        (srgba.blue * 255.0).round() as u8
    )
}

pub fn resolve_theme_color(name: &str, colors: &SchemeColors) -> Option<Color> {
    match name.to_lowercase().replace(['-', ' '], "_").as_str() {
        "background" | "bg" | "color0" | "color_0" | "0" | "crust" => Some(colors.background),
        "color1" | "color_1" | "1" | "base" | "deep_violet" => Some(colors.color1),
        "color2" | "color_2" | "2" | "blue" | "royal_blue" => Some(colors.color2),
        "color3" | "color_3" | "3" | "pink" | "pastel_pink" => Some(colors.color3),
        "color4" | "color_4" | "4" | "mauve" | "vibrant_purple" => Some(colors.color4),
        "color5" | "color_5" | "5" | "sapphire" | "indigo" => Some(colors.color5),
        "color6" | "color_6" | "6" | "lavender" | "light_purple" => Some(colors.color6),
        "color7" | "color_7" | "7" | "text" | "star_white" | "white" => Some(colors.color7),
        _ => None,
    }
}

pub fn register_engine_api(engine: &mut Engine) {
    // Register Transform type
    engine
        .register_type_with_name::<ScriptTransform>("Transform")
        .register_get("id", ScriptTransform::get_id)
        .register_get("index", ScriptTransform::get_index)
        .register_get("base_x", ScriptTransform::get_base_x)
        .register_get("base_y", ScriptTransform::get_base_y)
        .register_get("base_z", ScriptTransform::get_base_z)
        .register_get("base_rot", ScriptTransform::get_base_rot)
        .register_get("base_rotation", ScriptTransform::get_base_rot)
        .register_get("base_scale_x", ScriptTransform::get_base_scale_x)
        .register_get("base_scale_y", ScriptTransform::get_base_scale_y)
        .register_get_set("x", ScriptTransform::get_x, ScriptTransform::set_x)
        .register_get_set("y", ScriptTransform::get_y, ScriptTransform::set_y)
        .register_get_set("z", ScriptTransform::get_z, ScriptTransform::set_z)
        .register_get_set(
            "rotation",
            ScriptTransform::get_rotation,
            ScriptTransform::set_rotation,
        )
        .register_get_set("rot", ScriptTransform::get_rotation, ScriptTransform::set_rotation)
        .register_get_set("alpha", ScriptTransform::get_alpha, ScriptTransform::set_alpha)
        .register_fn("set_position", ScriptTransform::set_position)
        .register_fn("set_pos", ScriptTransform::set_position)
        .register_fn("translate", ScriptTransform::translate)
        .register_fn("rotate", ScriptTransform::rotate)
        .register_fn("set_theme_color", ScriptTransform::set_theme_color)
        .register_fn("set_color", ScriptTransform::set_color);

    // Register Scale type
    engine
        .register_type_with_name::<ScriptScale>("Scale")
        .register_get_set("x", ScriptScale::get_x, ScriptScale::set_x)
        .register_get_set("y", ScriptScale::get_y, ScriptScale::set_y)
        .register_fn("set", ScriptScale::set_uniform)
        .register_fn("scale", ScriptScale::scale);

    // Register Theme type
    engine
        .register_type_with_name::<ScriptTheme>("Theme")
        .register_get("background", |t: &mut ScriptTheme| t.background.clone())
        .register_get("bg", |t: &mut ScriptTheme| t.background.clone())
        .register_get("color1", |t: &mut ScriptTheme| t.color1.clone())
        .register_get("color_1", |t: &mut ScriptTheme| t.color1.clone())
        .register_get("color2", |t: &mut ScriptTheme| t.color2.clone())
        .register_get("color_2", |t: &mut ScriptTheme| t.color2.clone())
        .register_get("color3", |t: &mut ScriptTheme| t.color3.clone())
        .register_get("color_3", |t: &mut ScriptTheme| t.color3.clone())
        .register_get("color4", |t: &mut ScriptTheme| t.color4.clone())
        .register_get("color_4", |t: &mut ScriptTheme| t.color4.clone())
        .register_get("color5", |t: &mut ScriptTheme| t.color5.clone())
        .register_get("color_5", |t: &mut ScriptTheme| t.color5.clone())
        .register_get("color6", |t: &mut ScriptTheme| t.color6.clone())
        .register_get("color_6", |t: &mut ScriptTheme| t.color6.clone())
        .register_get("color7", |t: &mut ScriptTheme| t.color7.clone())
        .register_get("color_7", |t: &mut ScriptTheme| t.color7.clone())
        .register_fn("color", |t: &mut ScriptTheme, index: i64| -> String {
            match index {
                0 => t.background.clone(),
                1 => t.color1.clone(),
                2 => t.color2.clone(),
                3 => t.color3.clone(),
                4 => t.color4.clone(),
                5 => t.color5.clone(),
                6 => t.color6.clone(),
                7 => t.color7.clone(),
                _ => t.color1.clone(),
            }
        });

    // Global helpers
    engine.register_fn("theme", || -> ScriptTheme {
        CURRENT_THEME.with(|t| t.borrow().clone())
    });
    engine.register_fn("time", || -> f64 { CURRENT_TIME.with(|t| *t.borrow()) });
    engine.register_fn("delta", || -> f64 { CURRENT_DELTA.with(|d| *d.borrow()) });

    // Register for_each API: for_each("star", |mut transform, mut scale| { ... })
    engine.register_fn(
        "for_each",
        |context: NativeCallContext,
         pattern: &str,
         callback: FnPtr|
         -> Result<(), Box<EvalAltResult>> {
            let nodes = ACTIVE_FRAME_NODES.with(|cell| cell.borrow().clone());
            let pattern_lower = pattern.to_lowercase();

            let mut matched_index: i64 = 0;
            for node_arc in nodes {
                let id = {
                    let d = node_arc.read().unwrap();
                    d.id.clone()
                };

                // Match exact, contains, or wildcard "*"
                if pattern == "*" || id.to_lowercase().contains(&pattern_lower) {
                    {
                        let mut d = node_arc.write().unwrap();
                        d.index = matched_index;
                    }

                    let transform = ScriptTransform {
                        data: Arc::clone(&node_arc),
                    };
                    let scale = ScriptScale {
                        data: Arc::clone(&node_arc),
                    };

                    let dyn_transform = Dynamic::from(transform);
                    let dyn_scale = Dynamic::from(scale);
                    let dyn_index = Dynamic::from(matched_index);

                    // Call closure: try (transform, scale, index) first, then (transform, scale), then (transform,)
                    let call_result = callback.call_within_context::<()>(
                        &context,
                        (dyn_transform.clone(), dyn_scale.clone(), dyn_index),
                    );

                    if let Err(err) = call_result {
                        let err_msg = err.to_string();
                        info!("Error calling for_each callback for '{}': {}", id, err.to_string());
                        if err_msg.contains("takes 2 parameter") || err_msg.contains("takes 2 argument") {
                            let _ = callback.call_within_context::<()>(&context, (dyn_transform, dyn_scale));
                        } else if err_msg.contains("takes 1 parameter") || err_msg.contains("takes 1 argument") {
                            let _ = callback.call_within_context::<()>(&context, (dyn_transform,));
                        } else {
                            warn!("Error calling for_each callback for '{}': {}", id, err);
                        }
                    }

                    matched_index += 1;
                }
            }
            Ok(())
        },
    );
}

impl Plugin for ScriptPlugin {
    fn build(&self, app: &mut App) {
        let mut engine = Engine::new();

        engine.on_print(|msg| {
            info!("[Rhai] {}", msg);
        });

        register_engine_api(&mut engine);

        let mut ast = None;
        let mut scope = Scope::new();
        let mut has_update = false;

        if let Some(path) = &self.script_path {
            if path.exists() {
                match std::fs::read_to_string(path) {
                    Ok(source) => match engine.compile(&source) {
                        Ok(compiled_ast) => {
                            info!("Loaded wallpaper script: {:?}", path);
                            if let Err(e) =
                                engine.call_fn::<()>(&mut scope, &compiled_ast, "init", ())
                            {
                                if !e.to_string().contains("not found") {
                                    warn!("Error executing script init(): {}", e);
                                }
                            }
                            has_update = source.contains("fn update");
                            ast = Some(compiled_ast);
                        }
                        Err(e) => {
                            error!("Failed to compile Rhai script {:?}: {}", path, e);
                        }
                    },
                    Err(e) => {
                        warn!("Could not read script file {:?}: {}", path, e);
                    }
                }
            }
        }

        app.insert_resource(ScriptEngine {
            engine,
            ast,
            scope,
            has_update,
        });
        app.add_systems(
            Update,
            (
                init_svg_base_transforms,
                run_script_update.after(init_svg_base_transforms),
            ),
        );
    }
}

/// Attach BaseTransform to every spawned SvgNode so the initial position/rotation/scale is preserved
fn init_svg_base_transforms(
    mut commands: Commands,
    query: Query<(Entity, &Transform), (With<bevy_ecs_svg::SvgNode>, Without<BaseTransform>)>,
) {
    for (entity, transform) in query.iter() {
        commands.entity(entity).insert(BaseTransform {
            translation: transform.translation,
            rotation: transform.rotation,
            scale: transform.scale,
        });
    }
}

fn run_script_update(
    time: Res<Time>,
    colors: Res<SchemeColors>,
    mut script_res: ResMut<ScriptEngine>,
    mut nodes_query: Query<(Entity, &bevy_ecs_svg::SvgNode, &mut Transform, Option<&BaseTransform>)>,
    children_query: Query<&Children>,
    paths_query: Query<&MeshMaterial2d<ColorMaterial>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let script = &mut *script_res;
    let Some(ast) = &script.ast else {
        return;
    };
    if !script.has_update {
        return;
    }

    let t = time.elapsed_secs_f64();
    let dt = time.delta_secs_f64();

    CURRENT_TIME.with(|cell| *cell.borrow_mut() = t);
    CURRENT_DELTA.with(|cell| *cell.borrow_mut() = dt);

    // Update current theme in thread-local storage for Rhai
    CURRENT_THEME.with(|cell| {
        *cell.borrow_mut() = ScriptTheme {
            background: color_to_hex(colors.background),
            color1: color_to_hex(colors.color1),
            color2: color_to_hex(colors.color2),
            color3: color_to_hex(colors.color3),
            color4: color_to_hex(colors.color4),
            color5: color_to_hex(colors.color5),
            color6: color_to_hex(colors.color6),
            color7: color_to_hex(colors.color7),
        };
    });

    // Collect all active SVG nodes into frame data
    let mut frame_nodes = Vec::new();
    for (entity, svg_node, transform, base_opt) in nodes_query.iter() {
        let (base_trans, base_rot, base_scale) = if let Some(base) = base_opt {
            let rot_z = base.rotation.to_euler(EulerRot::XYZ).2 as f64;
            (
                base.translation,
                rot_z,
                Vec2::new(base.scale.x, base.scale.y),
            )
        } else {
            let rot_z = transform.rotation.to_euler(EulerRot::XYZ).2 as f64;
            (
                transform.translation,
                rot_z,
                Vec2::new(transform.scale.x, transform.scale.y),
            )
        };

        let mut current_alpha = 1.0;
        let mut queue = vec![entity];
        'alpha_search: while let Some(e) = queue.pop() {
            if let Ok(mat_handle) = paths_query.get(e) {
                if let Some(mat) = materials.get(&mat_handle.0) {
                    current_alpha = mat.color.alpha() as f64;
                    break 'alpha_search;
                }
            }
            if let Ok(children) = children_query.get(e) {
                for child in children.iter() {
                    queue.push(child);
                }
            }
        }

        let rot_z = transform.rotation.to_euler(EulerRot::XYZ).2 as f64;

        frame_nodes.push(Arc::new(RwLock::new(NodeTransformData {
            entity: Some(entity),
            id: svg_node.id.clone(),
            index: 0,
            base_x: base_trans.x as f64,
            base_y: base_trans.y as f64,
            base_z: base_trans.z as f64,
            base_rotation: base_rot,
            base_scale_x: base_scale.x as f64,
            base_scale_y: base_scale.y as f64,
            x: transform.translation.x as f64,
            y: transform.translation.y as f64,
            z: transform.translation.z as f64,
            rotation: rot_z,
            scale_x: transform.scale.x as f64,
            scale_y: transform.scale.y as f64,
            alpha: current_alpha,
            color_request: None,
            modified_transform: false,
            modified_alpha: false,
            modified_color: false,
        })));
    }

    // Set thread-local active frame nodes
    ACTIVE_FRAME_NODES.with(|cell| {
        *cell.borrow_mut() = frame_nodes.clone();
    });

    // Execute Rhai script's update(time, delta)
    if let Err(e) = script
        .engine
        .call_fn::<()>(&mut script.scope, ast, "update", (t, dt))
    {
        if !e.to_string().contains("not found") {
            warn_once!("Script update error: {}", e);
        }
    }

    // Clear active frame nodes
    ACTIVE_FRAME_NODES.with(|cell| {
        cell.borrow_mut().clear();
    });

    // Apply any modifications made by the script back into Bevy ECS
    for node_arc in &frame_nodes {
        let data = node_arc.read().unwrap();
        if data.modified_transform {
            if let Some(entity) = data.entity {
                if let Ok((_, _, mut transform, _)) = nodes_query.get_mut(entity) {
                    transform.translation.x = data.x as f32;
                    transform.translation.y = data.y as f32;
                    transform.translation.z = data.z as f32;
                    transform.rotation = Quat::from_rotation_z(data.rotation as f32);
                    transform.scale.x = data.scale_x as f32;
                    transform.scale.y = data.scale_y as f32;
                }
            }
        }
        if data.modified_alpha {
            if let Some(entity) = data.entity {
                let mut queue = vec![entity];
                while let Some(e) = queue.pop() {
                    if let Ok(mat_handle) = paths_query.get(e) {
                        if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
                            mat.color.set_alpha(data.alpha as f32);
                        }
                    }
                    if let Ok(children) = children_query.get(e) {
                        for child in children.iter() {
                            queue.push(child);
                        }
                    }
                }
            }
        }
        if data.modified_color {
            if let Some(entity) = data.entity {
                let resolved_color = match &data.color_request {
                    Some(ColorRequest::Theme(name)) => resolve_theme_color(name, &colors),
                    Some(ColorRequest::Custom(col)) => Some(*col),
                    None => None,
                };

                if let Some(mut new_c) = resolved_color {
                    let mut queue = vec![entity];
                    while let Some(e) = queue.pop() {
                        if let Ok(mat_handle) = paths_query.get(e) {
                            if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
                                let current_alpha = mat.color.alpha();
                                new_c.set_alpha(current_alpha);
                                mat.color = new_c;
                            }
                        }
                        if let Ok(children) = children_query.get(e) {
                            for child in children.iter() {
                                queue.push(child);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_for_each_script_api() {
        let mut engine = Engine::new();
        register_engine_api(&mut engine);

        let node1 = Arc::new(RwLock::new(NodeTransformData {
            id: "Star_alpha".into(),
            base_x: 10.0,
            base_y: 20.0,
            x: 10.0,
            y: 20.0,
            scale_x: 1.0,
            scale_y: 1.0,
            rotation: 0.0,
            alpha: 1.0,
            ..default()
        }));

        let node2 = Arc::new(RwLock::new(NodeTransformData {
            id: "Star_beta".into(),
            base_x: 50.0,
            base_y: 60.0,
            x: 50.0,
            y: 60.0,
            scale_x: 1.0,
            scale_y: 1.0,
            rotation: 0.0,
            alpha: 1.0,
            ..default()
        }));

        ACTIVE_FRAME_NODES.with(|c| *c.borrow_mut() = vec![Arc::clone(&node1), Arc::clone(&node2)]);

        let script = r#"
            for_each("Star", |mut transform, mut scale| {
                transform.x = transform.base_x + 15.0;
                transform.rotation = 0.5;
                transform.alpha = 0.75;
                scale.set(2.0);
            });
        "#;

        engine.eval::<()>(script).expect("Script execution failed");

        let d1 = node1.read().unwrap();
        assert_eq!(d1.x, 25.0);
        assert_eq!(d1.y, 20.0);
        assert_eq!(d1.rotation, 0.5);
        assert_eq!(d1.alpha, 0.75);
        assert_eq!(d1.scale_x, 2.0);
        assert_eq!(d1.scale_y, 2.0);
        assert!(d1.modified_transform);
        assert!(d1.modified_alpha);

        let d2 = node2.read().unwrap();
        assert_eq!(d2.x, 65.0);
        assert_eq!(d2.y, 60.0);
        assert_eq!(d2.index, 1);

        ACTIVE_FRAME_NODES.with(|c| c.borrow_mut().clear());
    }
}
