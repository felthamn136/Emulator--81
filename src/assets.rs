use std::path::{Path, PathBuf};

use macroquad::prelude::*;

/// Resolves optional assets from an override, working directory, executable
/// directory, or the Cargo project directory, in that order.
pub struct AssetManager {
    root: PathBuf,
}

pub struct TvAssets {
    pub tv_cabinet: Option<Texture2D>,
    pub crt_glass: Option<Texture2D>,
    pub crt_reflections: Option<Texture2D>,
    pub crt_shadow_mask: Option<Texture2D>,
    pub tv_shadow: Option<Texture2D>,
}

impl AssetManager {
    pub fn new() -> Self {
        let root = if let Some(path) = std::env::var_os("ZX81_ASSETS_DIR") {
            PathBuf::from(path)
        } else {
            let mut candidates = vec![PathBuf::from("assets")];
            if let Ok(exe) = std::env::current_exe() {
                if let Some(parent) = exe.parent() {
                    candidates.push(parent.join("assets"));
                }
            }
            candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"));
            candidates
                .into_iter()
                .find(|p| p.is_dir())
                .unwrap_or_else(|| {
                    eprintln!("No assets directory found; using built-in placeholder graphics.");
                    PathBuf::from("assets")
                })
        };
        Self { root }
    }

    pub fn load_texture_safe(&self, relative: &str) -> Option<Texture2D> {
        let path = self.root.join(relative);
        match read_texture(&path) {
            Ok(texture) => Some(texture),
            Err(error) => {
                eprintln!("Could not load texture {}: {error}", path.display());
                None
            }
        }
    }

    fn load_artwork_layer(&self, relative: &str) -> Option<Texture2D> {
        let texture = self.load_texture_safe(relative)?;
        if texture.width() != texture.height() {
            eprintln!("Artwork {relative} must use a square, full-canvas image; using fallback.");
            return None;
        }
        Some(texture)
    }

    pub fn load_bezel_assets(&self) -> TvAssets {
        TvAssets {
            tv_cabinet: self.load_artwork_layer("bezel/tv_cabinet.png"),
            crt_glass: self.load_artwork_layer("bezel/crt_glass.png"),
            crt_reflections: self.load_artwork_layer("bezel/crt_reflections.png"),
            crt_shadow_mask: self.load_artwork_layer("bezel/crt_shadow_mask.png"),
            tv_shadow: self.load_artwork_layer("bezel/tv_shadow.png"),
        }
    }
}

fn read_texture(path: &Path) -> Result<Texture2D, String> {
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    // Macroquad's convenience texture loader panics on malformed images;
    // decoding first allows a corrupt optional asset to use the fallback.
    let image = Image::from_file_with_format(&bytes, None).map_err(|error| error.to_string())?;
    let texture = Texture2D::from_image(&image);
    texture.set_filter(FilterMode::Linear);
    Ok(texture)
}
