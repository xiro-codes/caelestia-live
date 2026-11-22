use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};




pub const APP_NAME: &str = "caelestia-live";

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WallpaperPackage {
    pub name: String,
    pub archive_path: PathBuf,
    pub extracted_dir: PathBuf,
    pub svg_path: PathBuf,
    pub script_path: Option<PathBuf>,
}

/// Returns the backgrounds directory: ~/.config/{app_name}/bgs
pub fn get_bgs_dir() -> PathBuf {
    if let Some(config) = dirs::config_dir() {
        config.join(APP_NAME).join("bgs")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config").join(APP_NAME).join("bgs")
    } else {
        PathBuf::from(".config").join(APP_NAME).join("bgs")
    }
}

/// Returns the runtime cache directory for extracted wallpaper assets: ~/.cache/{app_name}/active
pub fn get_active_cache_dir() -> PathBuf {
    if let Some(cache) = dirs::cache_dir() {
        cache.join(APP_NAME).join("active")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".cache").join(APP_NAME).join("active")
    } else {
        std::env::temp_dir().join(APP_NAME).join("active")
    }
}

/// Lists all available wallpaper zip packages in ~/.config/{app_name}/bgs
pub fn list_wallpapers() -> Vec<PathBuf> {
    let bgs_dir = get_bgs_dir();
    let mut packages = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&bgs_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("zip") {
                packages.push(path);
            }
        }
    }

    packages.sort();
    packages
}

/// Finds the target wallpaper zip based on optional name or path, falling back to defaults.
pub fn resolve_wallpaper_path(target: Option<&str>) -> Result<PathBuf, String> {
    let bgs_dir = get_bgs_dir();

    if let Some(name) = target {
        let direct_path = PathBuf::from(name);
        if direct_path.is_file() {
            return Ok(direct_path);
        }

        let zip_in_bgs = bgs_dir.join(format!("{}.zip", name));
        if zip_in_bgs.is_file() {
            return Ok(zip_in_bgs);
        }

        let exact_in_bgs = bgs_dir.join(name);
        if exact_in_bgs.is_file() {
            return Ok(exact_in_bgs);
        }

        return Err(format!(
            "Wallpaper '{}' not found at {:?} or in {:?}",
            name, direct_path, bgs_dir
        ));
    }

    // No target provided: check ~/.config/{app_name}/bgs/
    let default_zip = bgs_dir.join("default.zip");
    if default_zip.is_file() {
        return Ok(default_zip);
    }

    let caelestia_zip = bgs_dir.join("caelestia.zip");
    if caelestia_zip.is_file() {
        return Ok(caelestia_zip);
    }

    let all = list_wallpapers();
    if let Some(first) = all.first() {
        return Ok(first.clone());
    }

    Err(format!(
        "No wallpaper zip found in {:?}. Create a zip containing 'bg.svg' (and an optional script) in that directory.",
        bgs_dir
    ))
}

/// Unpacks a wallpaper zip archive into the active cache directory, verifying `bg.svg`.
pub fn load_wallpaper(archive_path: &Path) -> Result<WallpaperPackage, String> {
    let file = File::open(archive_path)
        .map_err(|e| format!("Failed to open wallpaper archive {:?}: {}", archive_path, e))?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read zip archive {:?}: {}", archive_path, e))?;

    let cache_dir = get_active_cache_dir();
    if cache_dir.exists() {
        let _ = std::fs::remove_dir_all(&cache_dir);
    }
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create cache dir {:?}: {}", cache_dir, e))?;

    let mut has_bg_svg = false;
    let mut script_path = None;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read zip entry #{}: {}", i, e))?;

        let _raw_name = file.name().to_string();
        let path_name = match file.enclosed_name() {
            Some(p) => p.to_owned(),
            None => continue,
        };

        let outpath = cache_dir.join(&path_name);

        if file.is_dir() {
            std::fs::create_dir_all(&outpath)
                .map_err(|e| format!("Failed to create dir {:?}: {}", outpath, e))?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p)
                        .map_err(|e| format!("Failed to create parent dir {:?}: {}", p, e))?;
                }
            }

            let mut outfile = File::create(&outpath)
                .map_err(|e| format!("Failed to create file {:?}: {}", outpath, e))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("Failed to write file {:?}: {}", outpath, e))?;

            let file_name = path_name
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default();

            if file_name == "bg.svg" {
                has_bg_svg = true;
            } else if file_name == "main.rhai" || file_name == "script.rhai" {
                script_path = Some(outpath.clone());
            }
        }
    }

    if !has_bg_svg {
        return Err(format!(
            "Wallpaper package {:?} is missing required 'bg.svg'!",
            archive_path
        ));
    }

    let name = archive_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("wallpaper")
        .to_string();

    Ok(WallpaperPackage {
        name,
        archive_path: archive_path.to_path_buf(),
        svg_path: cache_dir.join("bg.svg"),
        script_path,
        extracted_dir: cache_dir,
    })
}

/// Helper to create a zip package from an SVG file and optional Rhai script.
pub fn create_wallpaper_zip(
    svg_source: &[u8],
    script_source: Option<&[u8]>,
    dest_zip_path: &Path,
) -> Result<(), String> {
    if let Some(parent) = dest_zip_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent dir {:?}: {}", parent, e))?;
    }

    let file = File::create(dest_zip_path)
        .map_err(|e| format!("Failed to create zip file {:?}: {}", dest_zip_path, e))?;

    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("bg.svg", options)
        .map_err(|e| format!("Failed to add bg.svg: {}", e))?;
    zip.write_all(svg_source)
        .map_err(|e| format!("Failed to write bg.svg: {}", e))?;

    if let Some(script) = script_source {
        zip.start_file("main.rhai", options)
            .map_err(|e| format!("Failed to add main.rhai: {}", e))?;
        zip.write_all(script)
            .map_err(|e| format!("Failed to write main.rhai: {}", e))?;
    }

    zip.finish()
        .map_err(|e| format!("Failed to finalize zip: {}", e))?;

    Ok(())
}

/// Helper to pack a directory into a wallpaper zip.
pub fn pack_directory_to_zip(source_dir: &Path, dest_zip_path: &Path) -> Result<(), String> {
    if !source_dir.is_dir() {
        return Err(format!("{:?} is not a directory", source_dir));
    }

    let bg_svg = source_dir.join("bg.svg");
    if !bg_svg.is_file() {
        return Err(format!(
            "Directory {:?} is missing required 'bg.svg'",
            source_dir
        ));
    }

    if let Some(parent) = dest_zip_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent dir {:?}: {}", parent, e))?;
    }

    let file = File::create(dest_zip_path)
        .map_err(|e| format!("Failed to create zip file {:?}: {}", dest_zip_path, e))?;

    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for entry in std::fs::read_dir(source_dir)
        .map_err(|e| format!("Failed to read directory {:?}: {}", source_dir, e))?
        .flatten()
    {
        let path = entry.path();
        if path.is_file() {
            let file_name = path
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or_else(|| "Invalid file name".to_string())?;

            zip.start_file(file_name, options)
                .map_err(|e| format!("Failed to add {} to zip: {}", file_name, e))?;

            let mut f = File::open(&path)
                .map_err(|e| format!("Failed to open file {:?}: {}", path, e))?;
            std::io::copy(&mut f, &mut zip)
                .map_err(|e| format!("Failed to copy file into zip {:?}: {}", path, e))?;
        }
    }

    zip.finish()
        .map_err(|e| format!("Failed to finalize zip: {}", e))?;

    Ok(())
}

#[derive(Default, Debug)]
struct TreeNode {
    tag: String,
    id: Option<String>,
    classes: Option<String>,
    children: Vec<TreeNode>,
}

/// Parses raw SVG string and produces an ASCII/Unicode hierarchy tree with targetable IDs
pub fn parse_svg_tree(svg_content: &str) -> String {
    // Simple manual SVG parser without external crates.
    let mut roots: Vec<TreeNode> = Vec::new();
    let mut stack: Vec<TreeNode> = Vec::new();
    let mut named_ids: Vec<String> = Vec::new();

    let mut pos = 0;
    while let Some(start) = svg_content[pos..].find('<') {
        let start_idx = pos + start;
        if let Some(end_rel) = svg_content[start_idx..].find('>') {
            let end_idx = start_idx + end_rel;
            let tag_str = &svg_content[start_idx + 1..end_idx].trim();
            pos = end_idx + 1;
            if tag_str.starts_with('!') || tag_str.starts_with('?') {
                continue;
            }
            if tag_str.starts_with('/') {
                if let Some(node) = stack.pop() {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(node);
                    } else {
                        roots.push(node);
                    }
                }
                continue;
            }
            let self_closing = tag_str.ends_with('/');
            let core = if self_closing {
                &tag_str[..tag_str.len() - 1].trim()
            } else {
                tag_str
            };
            let mut parts = core.split_whitespace();
            let tag_name = parts.next().unwrap_or("").to_string();
            let mut id_opt: Option<String> = None;
            let mut class_opt: Option<String> = None;
            for attr in parts {
                if let Some(eq_idx) = attr.find('=') {
                    let key = &attr[..eq_idx];
                    let mut val = &attr[eq_idx + 1..];
                    if (val.starts_with('"') && val.ends_with('"')) || (val.starts_with('\'') && val.ends_with('\'')) {
                        val = &val[1..val.len() - 1];
                    }
                    match key {
                        "id" => {
                            id_opt = Some(val.to_string());
                            named_ids.push(val.to_string());
                        }
                        "class" => {
                            class_opt = Some(val.to_string());
                        }
                        _ => {}
                    }
                }
            }
            let node = TreeNode {
                tag: tag_name,
                id: id_opt,
                classes: class_opt,
                children: Vec::new(),
            };
            if self_closing {
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    roots.push(node);
                }
            } else {
                stack.push(node);
            }
        } else {
            break;
        }
    }
    while let Some(node) = stack.pop() {
        if let Some(parent) = stack.last_mut() {
            parent.children.push(node);
        } else {
            roots.push(node);
        }
    }
    fn format_node(node: &TreeNode, prefix: &str, is_last: bool, out: &mut String) {
        let connector = if is_last { "└── " } else { "├── " };
        let mut desc = format!("<{}", node.tag);
        if let Some(id) = &node.id {
            desc.push_str(&format!(" id=\"{}\"", id));
        }
        if let Some(cls) = &node.classes {
            desc.push_str(&format!(" class=\"{}\"", cls));
        }
        if node.children.is_empty() {
            desc.push_str(" />");
        } else {
            desc.push('>');
        }
        out.push_str(&format!("{}{}{}\n", prefix, connector, desc));
        let new_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });
        let total = node.children.len();
        for (i, child) in node.children.iter().enumerate() {
            format_node(child, &new_prefix, i == total - 1, out);
        }
    }
    let mut out = String::new();
    out.push_str("SVG Hierarchy Tree:\n");
    let total = roots.len();
    for (i, root) in roots.iter().enumerate() {
        format_node(root, "", i == total - 1, &mut out);
    }
    out.push('\n');
    out.push_str("--------------------------------------------------------------------------------\n");
    out.push_str("Targetable IDs for Rhai scripts (for_each):\n");
    if named_ids.is_empty() {
        out.push_str("  (no explicit IDs found - use for_each(\"*\", ...) to match all elements)\n");
    } else {
        let mut ids = named_ids;
        ids.sort();
        ids.dedup();
        for id in ids {
            out.push_str(&format!("  - \"{}\"  -> for_each(\"{}\", |mut transform, mut scale| {{ ... }} )\n", id, id));
        }
    }
    out.push_str("--------------------------------------------------------------------------------\n");
    out
}

/// Reads bg.svg from a wallpaper zip archive and formats its SVG tree
pub fn get_wallpaper_svg_tree(archive_path: &Path) -> Result<String, String> {
    let file = File::open(archive_path)
        .map_err(|e| format!("Failed to open {:?}: {}", archive_path, e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read zip archive {:?}: {}", archive_path, e))?;

    let mut svg_file = archive
        .by_name("bg.svg")
        .map_err(|_| format!("Archive {:?} does not contain 'bg.svg'", archive_path))?;

    let mut content = String::new();
    svg_file
        .read_to_string(&mut content)
        .map_err(|e| format!("Failed to read bg.svg from archive: {}", e))?;

    Ok(parse_svg_tree(&content))
}

/// Opens an interactive editor session on main.rhai with a side-by-side read-only SVG tree view,
/// and automatically recompresses the package into ~/.config/caelestia-live/bgs/<name>.zip on save & exit.
pub fn edit_wallpaper(archive_path: &Path) -> Result<(), String> {
    let pkg_name = archive_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("wallpaper");

    let pid = std::process::id();
    let temp_dir = std::env::temp_dir().join(format!("caelestia_edit_{}_{}", pkg_name, pid));
    if temp_dir.exists() {
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp directory {:?}: {}", temp_dir, e))?;

    // Extract all entries from archive into temp_dir
    let file = File::open(archive_path)
        .map_err(|e| format!("Failed to open {:?}: {}", archive_path, e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read zip archive {:?}: {}", archive_path, e))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read zip entry #{}: {}", i, e))?;
        if let Some(p) = entry.enclosed_name() {
            let outpath = temp_dir.join(p);
            if entry.is_dir() {
                std::fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
            } else {
                if let Some(parent) = outpath.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let mut out = File::create(&outpath).map_err(|e| e.to_string())?;
                std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
            }
        }
    }

    // Ensure main.rhai exists
    let script_file = temp_dir.join("main.rhai");
    if !script_file.is_file() {
        let default_script = include_str!("../templates/default.rhai");
        std::fs::write(&script_file, default_script)
            .map_err(|e| format!("Failed to create main.rhai: {}", e))?;
    }

    // Generate SVG tree reference
    let bg_svg_file = temp_dir.join("bg.svg");
    if !bg_svg_file.is_file() {
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(format!("Wallpaper archive {:?} missing bg.svg", archive_path));
    }
    let svg_content = std::fs::read_to_string(&bg_svg_file)
        .map_err(|e| format!("Failed to read bg.svg: {}", e))?;
    let tree_str = parse_svg_tree(&svg_content);

    let tree_file = temp_dir.join("SVG_TREE.txt");
    let mut header = String::new();
    header.push_str("================================================================================\n");
    header.push_str(&format!("SVG TREE REFERENCE: {} (READ-ONLY VIEW)\n", pkg_name));
    header.push_str("Look up element IDs here for your Rhai animation script.\n");
    header.push_str("Hooks:\n");
    header.push_str("  - for_each(\"pattern\", |mut transform, mut scale| { ... })\n");
    header.push_str("  - for_each(\"pattern\", |mut transform| { ... })\n");
    header.push_str("Theme colors: background, color1, color2, color3, color4, color5, color6, color7\n");
    header.push_str("================================================================================\n\n");
    header.push_str(&tree_str);

    std::fs::write(&tree_file, header)
        .map_err(|e| format!("Failed to write SVG_TREE.txt: {}", e))?;

    // Mark SVG_TREE.txt read-only
    if let Ok(metadata) = std::fs::metadata(&tree_file) {
        let mut perms = metadata.permissions();
        perms.set_readonly(true);
        let _ = std::fs::set_permissions(&tree_file, perms);
    }

    // Detect editor: prefer VISUAL, then EDITOR, then nvim, vim, hx, nano
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| {
            for candidate in &["nvim", "vim", "hx", "nano", "vi"] {
                if let Ok(output) = std::process::Command::new("which").arg(candidate).output() {
                    if output.status.success() {
                        return candidate.to_string();
                    }
                }
            }
            "nvim".to_string()
        });

    println!("🛠️  Opening editor ({}) for wallpaper '{}'...", editor, pkg_name);
    println!("💡 Left split: main.rhai (editable animation script)");
    println!("💡 Right split: SVG_TREE.txt (read-only uncloseable reference)");

    loop {
        let status = if editor.ends_with("nvim") || editor.ends_with("vim") {
            std::process::Command::new(&editor)
                .arg("-O")
                .arg(&script_file)
                .arg(&tree_file)
                .arg("-c")
                .arg("wincmd l")
                .arg("-c")
                .arg("set readonly nomodifiable buftype=nofile")
                .arg("-c")
                .arg("wincmd h")
                .status()
        } else {
            std::process::Command::new(&editor)
                .arg(&script_file)
                .arg(&tree_file)
                .status()
        };

        match status {
            Ok(s) if s.success() => {
                // Validate script syntax with Rhai
                let engine = rhai::Engine::new();
                if let Ok(script_str) = std::fs::read_to_string(&script_file) {
                    if let Err(compile_err) = engine.compile(&script_str) {
                        eprintln!("\n⚠️  Syntax error detected in main.rhai: {}", compile_err);
                        eprint!("Reopen editor to fix? [Y/n]: ");
                        let mut input = String::new();
                        let _ = std::io::stdin().read_line(&mut input);
                        if input.trim().is_empty() || input.trim().eq_ignore_ascii_case("y") {
                            continue;
                        }
                    }
                }
                break;
            }
            Ok(s) => {
                eprintln!("Editor exited with status: {}", s);
                break;
            }
            Err(e) => {
                let _ = std::fs::remove_dir_all(&temp_dir);
                return Err(format!("Failed to launch editor '{}': {}", editor, e));
            }
        }
    }

    // Delete read-only SVG_TREE.txt before re-zipping
    let _ = std::fs::remove_file(&tree_file);

    // Re-pack temp_dir into archive_path atomically
    let tmp_zip = archive_path.with_extension("tmp.zip");
    if let Err(e) = pack_directory_to_zip(&temp_dir, &tmp_zip) {
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::remove_file(&tmp_zip);
        return Err(format!("Failed to re-pack archive: {}", e));
    }

    std::fs::rename(&tmp_zip, archive_path)
        .map_err(|e| format!("Failed to update {:?}: {}", archive_path, e))?;

    let _ = std::fs::remove_dir_all(&temp_dir);

    println!("✨ Successfully updated and re-packed {:?}", archive_path);
    println!("🚀 Run your updated wallpaper with: caelestia-live --bg {}", pkg_name);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_svg_tree() {
        let svg = r#"
            <svg id="root" width="100" height="100">
                <g id="Waves">
                    <path id="Wave1" class="primary" />
                    <path id="Wave2" />
                </g>
                <circle id="Star1" />
            </svg>
        "#;

        let tree = parse_svg_tree(svg);
        assert!(tree.contains("<svg id=\"root\">"));
        assert!(tree.contains("<g id=\"Waves\">"));
        assert!(tree.contains("<path id=\"Wave1\" class=\"primary\" />"));
        assert!(tree.contains("<path id=\"Wave2\" />"));
        assert!(tree.contains("<circle id=\"Star1\" />"));
        assert!(tree.contains("for_each(\"Wave1\""));
        assert!(tree.contains("for_each(\"Star1\""));
    }
}
