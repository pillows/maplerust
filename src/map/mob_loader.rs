use macroquad::prelude::*;
use crate::assets::AssetManager;
use std::collections::HashMap;
use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzObjectType, WzReader};

/// Cache for loaded Mob textures and parsed WZ nodes
pub struct MobCache {
    mobs: HashMap<String, (Texture2D, i32, i32)>, // (texture, origin_x, origin_y)
    wz_nodes: HashMap<String, WzNodeArc>, // Cache parsed WZ IMG files
}

impl MobCache {
    pub fn new() -> Self {
        Self {
            mobs: HashMap::new(),
            wz_nodes: HashMap::new(),
        }
    }

    /// Preload a Mob WZ file from already-fetched bytes
    pub async fn preload_mob_from_bytes(&mut self, mob_id: &str, bytes: Vec<u8>) -> Result<(), String> {
        // Skip if already loaded
        if self.wz_nodes.contains_key(mob_id) {
            return Ok(());
        }

        let wz_iv = guess_iv_from_wz_img(&bytes)
            .ok_or_else(|| "Unable to guess version from Mob file".to_string())?;

        let byte_len = bytes.len();
        let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
        let cache_name = format!("/01/Mob/{}.img", mob_id);
        let cache_name_ref: wz_reader::WzNodeName = cache_name.clone().into();
        let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
        let node: WzNodeArc = WzNode::new(&cache_name.into(), wz_image, None).into();

        node.write()
            .unwrap()
            .parse(&node)
            .map_err(|e| format!("Failed to parse Mob WZ: {:?}", e))?;

        self.wz_nodes.insert(mob_id.to_string(), node);
        Ok(())
    }

    /// Get or load a Mob texture
    /// Returns: (texture, origin_x, origin_y)
    pub async fn get_or_load_mob(
        &mut self,
        mob_id: &str,
    ) -> Option<(Texture2D, i32, i32)> {
        let key = format!("{}/stand/0", mob_id);

        // Return cached texture+origin if available
        if let Some((texture, ox, oy)) = self.mobs.get(&key).cloned() {
            return Some((texture, ox, oy));
        }

        // Load the Mob if not already loaded
        match self.load_mob_with_origin_cached(mob_id).await {
            Ok((texture, ox, oy)) => {
                self.mobs.insert(key.clone(), (texture.clone(), ox, oy));
                Some((texture, ox, oy))
            }
            Err(e) => {
                warn!("Failed to load Mob {}: {}", mob_id, e);
                None
            }
        }
    }

    /// Get or load all move animation frames for a Mob
    /// Returns: (textures, origins) - vectors of textures and their origin offsets
    pub async fn get_or_load_mob_move_frames(
        &mut self,
        mob_id: &str,
    ) -> Option<(Vec<Texture2D>, Vec<(i32, i32)>)> {
        info!("Loading move frames for Mob: {}", mob_id);

        // Reuse generic WZ animation loader that also handles origins.
        // This is the same mechanism used for logo and portal animations,
        // so it's well-tested and safe in WASM.
        let url = format!(
            "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/Mob/{}.img",
            mob_id
        );
        let cache_name = format!("/01/Mob/{}.img", mob_id);

        // Mob structure: root -> move -> [0, 1, 2, ...]
        let frames = AssetManager::load_animation_frames_with_origins(
            &url,
            &cache_name,
            "move",
        )
        .await;

        if frames.is_empty() {
            info!("  No move frames found for Mob {}", mob_id);
            return None;
        }

        let mut textures = Vec::with_capacity(frames.len());
        let mut origins = Vec::with_capacity(frames.len());

        for frame in frames {
            textures.push(frame.texture);
            origins.push((frame.origin.x as i32, frame.origin.y as i32));
        }

        info!("  Loaded {} move frames for Mob {}", textures.len(), mob_id);
        Some((textures, origins))
    }

    /// Load a specific Mob with origin (using cached WZ nodes)
    /// Returns: (texture, origin_x, origin_y)
    async fn load_mob_with_origin_cached(
        &mut self,
        mob_id: &str,
    ) -> Result<(Texture2D, i32, i32), String> {
        info!("Loading Mob: {}", mob_id);

        // Check if we already have this WZ node cached
        let root_node = if let Some(cached_node) = self.wz_nodes.get(mob_id) {
            info!("  Using cached WZ node for {}.img", mob_id);
            cached_node.clone()
        } else {
            // Build URL for the Mob file
            let url = format!(
                "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/Mob/{}.img",
                mob_id
            );
            let cache_name = format!("/01/Mob/{}.img", mob_id);

            // Fetch and parse the Mob file
            let bytes = AssetManager::fetch_and_cache(&url, &cache_name)
                .await
                .map_err(|e| format!("Failed to fetch Mob: {}", e))?;

            info!("  Parsing Mob file (size: {} bytes)...", bytes.len());

            let wz_iv = guess_iv_from_wz_img(&bytes)
                .ok_or_else(|| "Unable to guess version from Mob file".to_string())?;

            let byte_len = bytes.len();

            let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
            let cache_name_ref: wz_reader::WzNodeName = cache_name.clone().into();
            let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
            let node: WzNodeArc = WzNode::new(&cache_name.into(), wz_image, None).into();

            node.write()
                .unwrap()
                .parse(&node)
                .map_err(|e| format!("Failed to parse Mob WZ: {:?}", e))?;

            info!("  Mob WZ file parsed successfully");

            // Cache the parsed node
            self.wz_nodes.insert(mob_id.to_string(), node.clone());
            node
        };

        // Navigate to the Mob stand animation: stand/0
        // Mob structure: root -> stand -> 0 (or 1, 2, etc.)
        let paths_to_try = vec![
            "stand/0",
            "stand/1",
            "move/0",
            "fly/0",
        ];

        let mut mob_node = None;
        for path in paths_to_try {
            info!("  Trying Mob path: {}", path);
            if let Ok(node) = root_node.read().unwrap().at_path_parsed(path) {
                mob_node = Some(node);
                info!("  Found Mob at path: {}", path);
                break;
            }
        }

        let mob_node = mob_node.ok_or_else(|| "Mob stand animation not found".to_string())?;

        // Extract origin if available
        let mob_read = mob_node.read().unwrap();
        let (origin_x, origin_y) = if let Ok(origin_node) = mob_read.at_path_parsed("origin") {
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
        match &mob_read.object_type {
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
                    "Loaded Mob {} ({}x{}) with origin ({}, {})",
                    mob_id, width, height, origin_x, origin_y
                );

                Ok((texture, origin_x, origin_y))
            }
            _ => Err("Mob node is not a PNG".to_string()),
        }
    }

    /// Get Mob name from String/Mob.img
    pub async fn get_mob_name(mob_id: &str) -> Result<String, String> {
        const MOB_STRING_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/String/Mob.img";
        const MOB_STRING_CACHE: &str = "/01/String/Mob.img";

        info!("Fetching String/Mob.img to lookup Mob name for ID: {}", mob_id);

        // Fetch and cache the Mob.img file
        let bytes = AssetManager::fetch_and_cache(MOB_STRING_URL, MOB_STRING_CACHE).await
            .map_err(|e| format!("Failed to fetch String/Mob.img: {}", e))?;

        info!("Parsing String/Mob.img (size: {} bytes)...", bytes.len());

        // Guess IV and create reader
        let wz_iv = guess_iv_from_wz_img(&bytes)
            .ok_or_else(|| "Unable to guess version from String/Mob.img".to_string())?;

        let byte_len = bytes.len();
        let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));

        // Create root node
        let name: wz_reader::WzNodeName = "Mob.img".into();
        let wz_image = WzImage::new(&name, 0, byte_len, &reader);
        let root_node: WzNodeArc = WzNode::new(&name, wz_image, None).into();

        // Parse the root node
        root_node.write().unwrap().parse(&root_node)
            .map_err(|e| format!("Failed to parse String/Mob.img: {:?}", e))?;

        info!("String/Mob.img parsed, searching for Mob ID: {}", mob_id);

        // The structure is: root -> [mobId] -> name
        // Navigate directly to the Mob ID
        let root_read = root_node.read().unwrap();

        if let Some(mob_node) = root_read.children.get(mob_id) {
            info!("Found Mob ID {}", mob_id);

            // Get the name property from this node
            let mob_read = mob_node.read().unwrap();
            if let Ok(name_prop) = mob_read.at_path_parsed("name") {
                let name_prop_read = name_prop.read().unwrap();
                if let WzObjectType::Value(wz_reader::property::WzValue::String(val)) = &name_prop_read.object_type {
                    if let Ok(mob_name) = val.get_string() {
                        info!("Found Mob name: {}", mob_name);
                        return Ok(mob_name);
                    }
                }
            }

            warn!("Found Mob node for ID {} but could not extract name property", mob_id);
        }

        warn!("Mob ID {} not found in String/Mob.img", mob_id);
        Ok(String::new()) // Return empty string if not found
    }
}
