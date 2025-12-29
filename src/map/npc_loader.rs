use macroquad::prelude::*;
use crate::assets::AssetManager;
use std::collections::HashMap;
use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzObjectType, WzReader};

/// Cache for loaded NPC textures and parsed WZ nodes
pub struct NpcCache {
    npcs: HashMap<String, (Texture2D, i32, i32)>, // (texture, origin_x, origin_y)
    wz_nodes: HashMap<String, WzNodeArc>, // Cache parsed WZ IMG files
}

impl NpcCache {
    pub fn new() -> Self {
        Self {
            npcs: HashMap::new(),
            wz_nodes: HashMap::new(),
        }
    }

    /// Preload an NPC WZ file from already-fetched bytes
    pub async fn preload_npc_from_bytes(&mut self, npc_id: &str, bytes: Vec<u8>) -> Result<(), String> {
        // Skip if already loaded
        if self.wz_nodes.contains_key(npc_id) {
            return Ok(());
        }

        let wz_iv = guess_iv_from_wz_img(&bytes)
            .ok_or_else(|| "Unable to guess version from NPC file".to_string())?;

        let byte_len = bytes.len();
        let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
        let cache_name = format!("/01/Npc/{}.img", npc_id);
        let cache_name_ref: wz_reader::WzNodeName = cache_name.clone().into();
        let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
        let node: WzNodeArc = WzNode::new(&cache_name.into(), wz_image, None).into();

        node.write()
            .unwrap()
            .parse(&node)
            .map_err(|e| format!("Failed to parse NPC WZ: {:?}", e))?;

        self.wz_nodes.insert(npc_id.to_string(), node);
        Ok(())
    }

    /// Get or load an NPC texture
    /// Returns: (texture, origin_x, origin_y)
    pub async fn get_or_load_npc(
        &mut self,
        npc_id: &str,
    ) -> Option<(Texture2D, i32, i32)> {
        let key = format!("{}/stand/0", npc_id);

        // Return cached texture+origin if available
        if let Some((texture, ox, oy)) = self.npcs.get(&key).cloned() {
            return Some((texture, ox, oy));
        }

        // Load the NPC if not already loaded
        match self.load_npc_with_origin_cached(npc_id).await {
            Ok((texture, ox, oy)) => {
                self.npcs.insert(key.clone(), (texture.clone(), ox, oy));
                Some((texture, ox, oy))
            }
            Err(e) => {
                warn!("Failed to load NPC {}: {}", npc_id, e);
                None
            }
        }
    }

    /// Load a specific NPC with origin (using cached WZ nodes)
    /// Returns: (texture, origin_x, origin_y)
    async fn load_npc_with_origin_cached(
        &mut self,
        npc_id: &str,
    ) -> Result<(Texture2D, i32, i32), String> {
        info!("Loading NPC: {}", npc_id);

        // Check if we already have this WZ node cached
        let root_node = if let Some(cached_node) = self.wz_nodes.get(npc_id) {
            info!("  Using cached WZ node for {}.img", npc_id);
            cached_node.clone()
        } else {
            // Build URL for the NPC file
            let url = format!(
                "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/Npc/{}.img",
                npc_id
            );
            let cache_name = format!("/01/Npc/{}.img", npc_id);

            // Fetch and parse the NPC file
            let bytes = AssetManager::fetch_and_cache(&url, &cache_name)
                .await
                .map_err(|e| format!("Failed to fetch NPC: {}", e))?;

            info!("  Parsing NPC file (size: {} bytes)...", bytes.len());

            let wz_iv = guess_iv_from_wz_img(&bytes)
                .ok_or_else(|| "Unable to guess version from NPC file".to_string())?;

            let byte_len = bytes.len();

            let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
            let cache_name_ref: wz_reader::WzNodeName = cache_name.clone().into();
            let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
            let node: WzNodeArc = WzNode::new(&cache_name.into(), wz_image, None).into();

            node.write()
                .unwrap()
                .parse(&node)
                .map_err(|e| format!("Failed to parse NPC WZ: {:?}", e))?;

            info!("  NPC WZ file parsed successfully");

            // Cache the parsed node
            self.wz_nodes.insert(npc_id.to_string(), node.clone());
            node
        };

        // Navigate to the NPC stand animation: stand/0
        // Try multiple paths as NPC structure can vary
        let paths_to_try = vec![
            "stand/0",
            "stand",
            "default/0",
            "default",
            "0/stand/0",
            "0",
        ];

        let mut npc_node = None;
        for path in paths_to_try {
            info!("  Trying NPC path: {}", path);
            if let Ok(node) = root_node.read().unwrap().at_path_parsed(path) {
                npc_node = Some(node);
                info!("  Found NPC at path: {}", path);
                break;
            }
        }

        // If still not found, try to find the first available animation frame
        if npc_node.is_none() {
            info!("  No standard paths found, searching for first available frame...");
            let root_read = root_node.read().unwrap();
            // Look for any child that might be an animation directory
            for (child_name, child_node) in root_read.children.iter() {
                let child_read = child_node.read().unwrap();
                // Check if this child has a "0" frame (common animation structure)
                if let Ok(frame_node) = child_read.at_path_parsed("0") {
                    npc_node = Some(frame_node);
                    info!("  Using fallback animation: {}/0", child_name.as_str());
                    break;
                }
                // Or if the child itself is a PNG
                if matches!(child_read.object_type, wz_reader::WzObjectType::Property(_)) {
                    npc_node = Some(child_node.clone());
                    info!("  Using fallback node: {}", child_name.as_str());
                    break;
                }
            }
        }

        let npc_node = npc_node.ok_or_else(|| {
            warn!("NPC {}: No suitable animation found (tried: stand/0, stand, default/0, default, 0/stand/0, 0, and first available)", npc_id);
            "NPC animation not found".to_string()
        })?;

        // Extract origin if available
        let npc_read = npc_node.read().unwrap();
        let (origin_x, origin_y) = if let Ok(origin_node) = npc_read.at_path_parsed("origin") {
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
        match &npc_read.object_type {
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
                    "Loaded NPC {} ({}x{}) with origin ({}, {})",
                    npc_id, width, height, origin_x, origin_y
                );

                Ok((texture, origin_x, origin_y))
            }
            _ => Err("NPC node is not a PNG".to_string()),
        }
    }

    /// Get NPC name from String/NPC.img
    pub async fn get_npc_name(npc_id: &str) -> Result<String, String> {
        const NPC_STRING_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/String/Npc.img";
        const NPC_STRING_CACHE: &str = "/01/String/Npc.img";

        info!("Fetching String/Npc.img to lookup NPC name for ID: {}", npc_id);

        // Fetch and cache the Npc.img file
        let bytes = AssetManager::fetch_and_cache(NPC_STRING_URL, NPC_STRING_CACHE).await
            .map_err(|e| format!("Failed to fetch String/Npc.img: {}", e))?;

        info!("Parsing String/Npc.img (size: {} bytes)...", bytes.len());

        // Guess IV and create reader
        let wz_iv = guess_iv_from_wz_img(&bytes)
            .ok_or_else(|| "Unable to guess version from String/Npc.img".to_string())?;

        let byte_len = bytes.len();
        let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));

        // Create root node
        let name: wz_reader::WzNodeName = "Npc.img".into();
        let wz_image = WzImage::new(&name, 0, byte_len, &reader);
        let root_node: WzNodeArc = WzNode::new(&name, wz_image, None).into();

        // Parse the root node
        root_node.write().unwrap().parse(&root_node)
            .map_err(|e| format!("Failed to parse String/Npc.img: {:?}", e))?;

        info!("String/Npc.img parsed, searching for NPC ID: {}", npc_id);

        // The structure is: root -> [npcId] -> name
        // Navigate directly to the NPC ID
        let root_read = root_node.read().unwrap();

        if let Some(npc_node) = root_read.children.get(npc_id) {
            info!("Found NPC ID {}", npc_id);

            // Get the name property from this node
            let npc_read = npc_node.read().unwrap();
            if let Ok(name_prop) = npc_read.at_path_parsed("name") {
                let name_prop_read = name_prop.read().unwrap();
                if let WzObjectType::Value(wz_reader::property::WzValue::String(val)) = &name_prop_read.object_type {
                    if let Ok(npc_name) = val.get_string() {
                        info!("Found NPC name: {}", npc_name);
                        return Ok(npc_name);
                    }
                }
            }

            warn!("Found NPC node for ID {} but could not extract name property", npc_id);
        }

        warn!("NPC ID {} not found in String/Npc.img", npc_id);
        Ok(String::new()) // Return empty string if not found
    }
}
