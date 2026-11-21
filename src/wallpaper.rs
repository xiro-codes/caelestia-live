use std::fs::File;
use std::io::Write;
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
