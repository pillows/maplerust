use macroquad::prelude::*;
use crate::assets::AssetManager;
use std::collections::HashMap;
use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzObjectType, WzReader};

/// Cache for loaded portal textures and parsed WZ nodes
pub struct PortalCache {
    portals: HashMap<String, (Vec<Texture2D>, Vec<(i32, i32)>)>, // (animation frames, origins per frame)
    wz_node: Option<WzNodeArc>, // Cache the MapHelper.img node
}

impl PortalCache {
    pub fn new() -> Self {
        Self {
            portals: HashMap::new(),
            wz_node: None,
        }
    }

    /// Get or load portal animation frames by portal type
    /// Portal types: "sp" (spawn), "pv" (regular), "pt" (auto), "pc" (collision), etc.
    /// Returns: (animation frames, Vec of (origin_x, origin_y) for each frame)
    pub async fn get_or_load_portal(
        &mut self,
        portal_type: &str,
    ) -> Option<(Vec<Texture2D>, Vec<(i32, i32)>)> {
        let key = portal_type.to_string();

        // Return cached animation if available
        if let Some((frames, origins)) = self.portals.get(&key) {
            return Some((frames.clone(), origins.clone()));
        }

        // Load the portal animation
        match self.load_portal_animation_cached(portal_type).await {
            Ok((frames, origins)) => {
                self.portals.insert(key.clone(), (frames.clone(), origins.clone()));
                Some((frames, origins))
            }
            Err(e) => {
                warn!("Failed to load portal {}: {}", key, e);
                None
            }
        }
    }

    /// Load portal animation with origins (using cached WZ node)
    /// Returns: (animation frames, Vec of (origin_x, origin_y) for each frame)
    async fn load_portal_animation_cached(
        &mut self,
        portal_type: &str,
    ) -> Result<(Vec<Texture2D>, Vec<(i32, i32)>), String> {
        info!("Loading portal animation: {}", portal_type);

        // Check if we already have MapHelper.img cached
        let root_node = if let Some(cached_node) = &self.wz_node {
            info!("  Using cached MapHelper.img node");
            cached_node.clone()
        } else {
            // Build URL for MapHelper.img
            let url = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/Map/MapHelper.img";
            let cache_name = "/01/Map/MapHelper.img";

            // Fetch and parse MapHelper.img
            let bytes = AssetManager::fetch_and_cache(url, cache_name)
                .await
                .map_err(|e| format!("Failed to fetch MapHelper.img: {}", e))?;

            info!("  Parsing MapHelper.img (size: {} bytes)...", bytes.len());

            let wz_iv = guess_iv_from_wz_img(&bytes)
                .ok_or_else(|| "Unable to guess version from MapHelper.img".to_string())?;

            let byte_len = bytes.len();
            let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
            let cache_name_ref: wz_reader::WzNodeName = cache_name.to_string().into();
            let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
            let node: WzNodeArc = WzNode::new(&cache_name.into(), wz_image, None).into();

            node.write()
                .unwrap()
                .parse(&node)
                .map_err(|e| format!("Failed to parse MapHelper.img: {:?}", e))?;

            info!("  MapHelper.img parsed successfully");

            // Cache the parsed node
            self.wz_node = Some(node.clone());
            node
        };

        // Navigate to portal/game/[portal_type]
        let portal_path = format!("portal/game/{}", portal_type);
        info!("  Navigating to portal path: {}", portal_path);

        let portal_node = root_node
            .read()
            .unwrap()
            .at_path_parsed(&portal_path)
            .map_err(|_| format!("Portal type '{}' not found at path: {}", portal_type, portal_path))?;

        let portal_read = portal_node.read().unwrap();

        // For simple portals like "pv", frames are directly numbered (0, 1, 2, ...)
        // For complex portals like "ph", frames are under default/portalContinue
        let frames_node = if portal_read.children.contains_key("default") {
            // Complex portal (ph, psh) - use default/portalContinue
            info!("  Using default/portalContinue for portal type '{}'", portal_type);
            if let Some(default_node) = portal_read.children.get("default") {
                let default_read = default_node.read().unwrap();
                if let Some(continue_node) = default_read.children.get("portalContinue") {
                    continue_node.clone()
                } else {
                    portal_node.clone() // Fallback
                }
            } else {
                portal_node.clone()
            }
        } else {
            // Simple portal (pv) - frames are direct children
            portal_node.clone()
        };

        let frames_read = frames_node.read().unwrap();

        // Load all animation frames (0, 1, 2, ...) with their individual origins
        let mut frames: Vec<Texture2D> = Vec::new();
        let mut origins: Vec<(i32, i32)> = Vec::new();

        // Get all numeric frame names and sort them
        let mut frame_names: Vec<String> = frames_read
            .children
            .keys()
            .filter_map(|name| {
                let name_str = name.to_string();
                if name_str.parse::<i32>().is_ok() {
                    Some(name_str)
                } else {
                    None
                }
            })
            .collect();

        // Sort numeric frames
        frame_names.sort_by_key(|name| name.parse::<i32>().unwrap_or(0));

        info!("  Found {} frames: {:?}", frame_names.len(), frame_names);

        for frame_name in &frame_names {
            if let Some(frame_child) = frames_read.children.get(frame_name.as_str()) {
                let frame_child_read = frame_child.read().unwrap();

                // Get origin for this specific frame
                let (origin_x, origin_y) = if let Ok(origin_node) = frame_child_read.at_path_parsed("origin") {
                    let origin_read = origin_node.read().unwrap();
                    match &origin_read.object_type {
                        WzObjectType::Value(wz_reader::property::WzValue::Vector(vec)) => {
                            (vec.0, vec.1)
                        }
                        _ => (0, 0)
                    }
                } else {
                    (0, 0)
                };

                // PNG is directly at this node
                match &frame_child_read.object_type {
                    WzObjectType::Property(wz_reader::property::WzSubProperty::PNG(png_prop)) => {
                        let dynamic_img = png_prop.extract_png()
                            .map_err(|e| format!("Failed to extract PNG from frame {}: {:?}", frame_name, e))?;

                        let rgba_img = dynamic_img.to_rgba8();
                        let width = rgba_img.width() as u16;
                        let height = rgba_img.height() as u16;
                        let bytes = rgba_img.into_raw();

                        let texture = Texture2D::from_rgba8(width, height, &bytes);
                        texture.set_filter(FilterMode::Linear);

                        info!("  Loaded frame {} ({}x{}) origin=({}, {})", frame_name, width, height, origin_x, origin_y);
                        frames.push(texture);
                        origins.push((origin_x, origin_y));
                    }
                    _ => {
                        warn!("  Frame {} is not a PNG, skipping", frame_name);
                    }
                }
            }
        }

        if frames.is_empty() {
            return Err(format!("No frames found for portal type: {}", portal_type));
        }

        info!(
            "Loaded portal {} with {} frames",
            portal_type,
            frames.len()
        );

        Ok((frames, origins))
    }
}

/// Convert portal type number to portal type string
/// Only types that have graphics in MapHelper.img are supported
/// 0 = sp (spawn point - no graphics, invisible)
/// 1 = pi (invisible portal - no graphics)
/// 2 = pv (visible portal - has graphics)
/// 3 = pc (collision portal - no graphics)
/// 7 = ps (script portal - no graphics)
/// 10 = ph (hidden portal - has graphics)
pub fn get_portal_type_string(pt: i32) -> Option<&'static str> {
    match pt {
        2 => Some("pv"),   // Visible portal (the most common one)
        10 => Some("ph"),  // Hidden portal
        _ => None,         // No graphics for this portal type
    }
}
