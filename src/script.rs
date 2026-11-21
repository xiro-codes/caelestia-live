use bevy::prelude::*;
use rhai::{Engine, Scope, AST};
use std::path::PathBuf;

#[derive(Resource)]
pub struct ScriptEngine {
    pub engine: Engine,
    pub ast: Option<AST>,
    pub scope: Scope<'static>,
}

pub struct ScriptPlugin {
    pub script_path: Option<PathBuf>,
}

impl Plugin for ScriptPlugin {
    fn build(&self, app: &mut App) {
        let mut engine = Engine::new();

        engine.on_print(|msg| {
            info!("[Rhai] {}", msg);
        });

        let mut ast = None;
        let mut scope = Scope::new();

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

        app.insert_resource(ScriptEngine { engine, ast, scope });
        app.add_systems(Update, run_script_update);
    }
}

fn run_script_update(time: Res<Time>, mut script_res: ResMut<ScriptEngine>) {
    let script = &mut *script_res;
    if let Some(ast) = &script.ast {
        let t = time.elapsed_secs_f64();
        let dt = time.delta_secs_f64();
        if let Err(e) = script
            .engine
            .call_fn::<()>(&mut script.scope, ast, "update", (t, dt))
        {
            if !e.to_string().contains("not found") {
                // Ignore function not found, otherwise handle error
            }
        }
    }
}
