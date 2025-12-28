use macroquad::prelude::*;
use crate::assets::AssetManager;
use std::collections::HashMap;
use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzObjectType, WzReader};

/// Cache for loaded tile textures
pub struct TileCache {
    tiles: HashMap<String, Texture2D>,
}

impl TileCache {
    pub fn new() -> Self {
        Self {
            tiles: HashMap::new(),
        }
    }

    /// Get or load a tile texture
    /// Key format: "tileset_name/category/tile_number" (e.g., "grassySoil/enH0/0")
    /// Returns: (texture, origin_x, origin_y)
    pub async fn get_or_load_tile(
        &mut self,
        tileset_name: &str,
        category: &str,
        tile_number: i32,
    ) -> Option<(Texture2D, i32, i32)> {
        let key = format!("{}/{}/{}", tileset_name, category, tile_number);

        // Return cached texture if available
        if let Some(texture) = self.tiles.get(&key) {
            // If cached, we need to get origin separately
            // For now, fetch origin each time (could be optimized to cache origin too)
            match Self::get_tile_origin_internal(tileset_name, category, tile_number).await {
                Ok((ox, oy)) => return Some((texture.clone(), ox, oy)),
                Err(_) => return Some((texture.clone(), 0, 0)), // Default to (0,0) if origin not found
            }
        }

        // Load the tileset if not already loaded
        match Self::load_tile_with_origin(tileset_name, category, tile_number).await {
            Ok((texture, ox, oy)) => {
                self.tiles.insert(key.clone(), texture.clone());
                Some((texture, ox, oy))
            }
            Err(e) => {
                warn!("Failed to load tile {}: {}", key, e);
                None
            }
        }
    }

    /// Load a specific tile from a tileset with origin
    /// Returns: (texture, origin_x, origin_y)
    async fn load_tile_with_origin(
        tileset_name: &str,
        category: &str,
        tile_number: i32,
    ) -> Result<(Texture2D, i32, i32), String> {
        info!("Loading tile: {}/{}/{}", tileset_name, category, tile_number);

        // Build URL for the tile file
        let url = format!(
            "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/Map/Tile/{}.img",
            tileset_name
        );
        let cache_name = format!("/01/Map/Tile/{}.img", tileset_name);

        // Fetch and parse the tile file
        let bytes = AssetManager::fetch_and_cache(&url, &cache_name)
            .await
            .map_err(|e| format!("Failed to fetch tile: {}", e))?;

        info!("Parsing tile file (size: {} bytes)...", bytes.len());

        let wz_iv = guess_iv_from_wz_img(&bytes)
            .ok_or_else(|| "Unable to guess version from tile file".to_string())?;

        let byte_len = bytes.len();

        let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
        let cache_name_ref: wz_reader::WzNodeName = cache_name.clone().into();
        let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
        let root_node: WzNodeArc = WzNode::new(&cache_name.into(), wz_image, None).into();

        root_node
            .write()
            .unwrap()
            .parse(&root_node)
            .map_err(|e| format!("Failed to parse tile WZ: {:?}", e))?;

        info!("Tile WZ file parsed successfully");

        // Navigate to the specific tile: category/tile_number
        let tile_path = format!("{}/{}", category, tile_number);
        let tile_node = root_node
            .read()
            .unwrap()
            .at_path_parsed(&tile_path)
            .map_err(|_| format!("Tile not found at path: {}", tile_path))?;

        // Extract origin if available
        let tile_read = tile_node.read().unwrap();
        let (origin_x, origin_y) = if let Ok(origin_node) = tile_read.at_path_parsed("origin") {
            let origin_read = origin_node.read().unwrap();
            match &origin_read.object_type {
                WzObjectType::Value(wz_reader::property::WzValue::Vector(vec)) => {
                    info!("  Found origin: ({}, {})", vec.0, vec.1);
                    (vec.0, vec.1)
                }
                _ => {
                    info!("  Origin found but not a Vector, using default (0, 0)");
                    (0, 0)
                }
            }
        } else {
            info!("  No origin found, using default (0, 0)");
            (0, 0)
        };

        // Extract PNG data
        match &tile_read.object_type {
            WzObjectType::Property(wz_reader::property::WzSubProperty::PNG(png_prop)) => {
                // Get PNG data
                let dynamic_img = png_prop.extract_png()
                    .map_err(|e| format!("Failed to extract PNG: {:?}", e))?;

                // Convert to RGBA8 format
                let rgba_img = dynamic_img.to_rgba8();
                let width = rgba_img.width() as u16;
                let height = rgba_img.height() as u16;
                let bytes = rgba_img.into_raw();

                // Load texture from PNG data
                let texture = Texture2D::from_rgba8(width, height, &bytes);
                texture.set_filter(FilterMode::Linear);

                info!(
                    "Loaded tile {}/{}/{} ({}x{}) with origin ({}, {})",
                    tileset_name, category, tile_number, width, height, origin_x, origin_y
                );

                Ok((texture, origin_x, origin_y))
            }
            _ => Err(format!("Tile node is not a PNG: {}", tile_path)),
        }
    }

    /// Internal helper to get tile origin
    async fn get_tile_origin_internal(
        tileset_name: &str,
        category: &str,
        tile_number: i32,
    ) -> Result<(i32, i32), String> {
        let url = format!(
            "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/Map/Tile/{}.img",
            tileset_name
        );
        let cache_name = format!("/01/Map/Tile/{}.img", tileset_name);

        let bytes = AssetManager::fetch_and_cache(&url, &cache_name)
            .await
            .map_err(|e| format!("Failed to fetch tile: {}", e))?;

        let wz_iv = guess_iv_from_wz_img(&bytes)
            .ok_or_else(|| "Unable to guess version from tile file".to_string())?;

        let byte_len = bytes.len();

        let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
        let cache_name_ref: wz_reader::WzNodeName = cache_name.clone().into();
        let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
        let root_node: WzNodeArc = WzNode::new(&cache_name.into(), wz_image, None).into();

        root_node
            .write()
            .unwrap()
            .parse(&root_node)
            .map_err(|e| format!("Failed to parse tile WZ: {:?}", e))?;

        // Navigate to the specific tile: category/tile_number/origin
        let tile_path = format!("{}/{}", category, tile_number);
        let tile_node = root_node
            .read()
            .unwrap()
            .at_path_parsed(&tile_path)
            .map_err(|_| format!("Tile not found at path: {}", tile_path))?;

        let tile_read = tile_node.read().unwrap();
        if let Ok(origin_node) = tile_read.at_path_parsed("origin") {
            let origin_read = origin_node.read().unwrap();
            match &origin_read.object_type {
                WzObjectType::Value(wz_reader::property::WzValue::Vector(vec)) => {
                    return Ok((vec.0, vec.1));
                }
                _ => {}
            }
        }

        Ok((0, 0)) // Default origin
    }

    /// Get tile origin (offset for rendering)
    pub async fn get_tile_origin(
        tileset_name: &str,
        category: &str,
        tile_number: i32,
    ) -> Result<(i32, i32), String> {
        let url = format!(
            "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/Map/Tile/{}.img",
            tileset_name
        );
        let cache_name = format!("/01/Map/Tile/{}.img", tileset_name);

        let bytes = AssetManager::fetch_and_cache(&url, &cache_name)
            .await
            .map_err(|e| format!("Failed to fetch tile: {}", e))?;

        let wz_iv = guess_iv_from_wz_img(&bytes)
            .ok_or_else(|| "Unable to guess version from tile file".to_string())?;

        let byte_len = bytes.len();

        let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
        let cache_name_ref: wz_reader::WzNodeName = cache_name.clone().into();
        let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
        let root_node: WzNodeArc = WzNode::new(&cache_name.into(), wz_image, None).into();

        root_node
            .write()
            .unwrap()
            .parse(&root_node)
            .map_err(|e| format!("Failed to parse tile WZ: {:?}", e))?;

        // Navigate to origin: tileset_name/category/tile_number/origin
        let origin_path = format!("{}/{}/{}/origin", tileset_name, category, tile_number);
        let origin_node = root_node
            .read()
            .unwrap()
            .at_path_parsed(&origin_path)
            .map_err(|_| format!("Origin not found at path: {}", origin_path))?;

        let origin_read = origin_node.read().unwrap();
        match &origin_read.object_type {
            WzObjectType::Value(wz_reader::property::WzValue::Vector(vec)) => {
                Ok((vec.0, vec.1))
            }
            _ => Err(format!("Origin is not a Vector at path: {}", origin_path)),
        }
    }
}

/// Information about a tile in the tileset
#[derive(Debug, Clone)]
pub struct TileInfo {
    pub texture: Texture2D,
    pub origin_x: i32,
    pub origin_y: i32,
    pub z: i32,
}

/// Helper function to parse tile category from background set name
/// Based on MapleStory naming conventions:
/// - enH0, enH1: Horizontal edges
/// - enV0, enV1: Vertical edges
/// - edU: Edge up
/// - edD: Edge down
/// - bsc: Basic tiles (0-4 only in grassySoil)
/// - slLU, slLD, slRU, slRD: Slopes (Left/Right, Up/Down)
pub fn get_tile_category_for_number(tile_number: i32) -> &'static str {
    // This is a simplified mapping - in a real implementation,
    // you'd need to determine the category based on the tile's position
    // and surrounding tiles. For now, we'll use a basic mapping:
    // Most tiles 0-3 are edge tiles (enH0), and 4+ might be bsc
    // But since we don't know the exact mapping, default to enH0
    match tile_number {
        0..=3 => "enH0",  // Horizontal edges
        _ => "bsc",       // Basic tiles (but might not exist for all numbers)
    }
}
