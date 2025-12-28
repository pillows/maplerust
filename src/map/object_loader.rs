use macroquad::prelude::*;
use crate::assets::AssetManager;
use std::collections::HashMap;
use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzObjectType, WzReader};

/// Cache for loaded object textures and parsed WZ nodes
pub struct ObjectCache {
    objects: HashMap<String, Texture2D>,
    wz_nodes: HashMap<String, WzNodeArc>, // Cache parsed WZ IMG files
}

impl ObjectCache {
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            wz_nodes: HashMap::new(),
        }
    }

    /// Get or load an object texture
    /// Key format: "oS/l0/l1/l2" (e.g., "acc1/grassySoil/artificiality/26")
    /// Returns: (texture, origin_x, origin_y)
    pub async fn get_or_load_object(
        &mut self,
        oS: &str,
        l0: &str,
        l1: &str,
        l2: &str,
    ) -> Option<(Texture2D, i32, i32)> {
        let key = format!("{}/{}/{}/{}", oS, l0, l1, l2);

        // Return cached texture if available
        if let Some(texture) = self.objects.get(&key).cloned() {
            // If cached, we need to get origin separately
            // For now, fetch origin each time (could be optimized to cache origin too)
            match self.get_object_origin_internal(oS, l0, l1, l2).await {
                Ok((ox, oy)) => return Some((texture, ox, oy)),
                Err(_) => return Some((texture, 0, 0)), // Default to (0,0) if origin not found
            }
        }

        // Load the object if not already loaded
        match self.load_object_with_origin_cached(oS, l0, l1, l2).await {
            Ok((texture, ox, oy)) => {
                self.objects.insert(key.clone(), texture.clone());
                Some((texture, ox, oy))
            }
            Err(e) => {
                warn!("Failed to load object {}: {}", key, e);
                None
            }
        }
    }

    /// Load a specific object from an object set with origin (using cached WZ nodes)
    /// Returns: (texture, origin_x, origin_y)
    async fn load_object_with_origin_cached(
        &mut self,
        oS: &str,
        l0: &str,
        l1: &str,
        l2: &str,
    ) -> Result<(Texture2D, i32, i32), String> {
        info!("Loading object: {}/{}/{}/{}", oS, l0, l1, l2);

        // Check if we already have this WZ node cached
        let root_node = if let Some(cached_node) = self.wz_nodes.get(oS) {
            info!("  Using cached WZ node for {}.img", oS);
            cached_node.clone()
        } else {
            // Build URL for the object file
            let url = format!(
                "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/Map/Obj/{}.img",
                oS
            );
            let cache_name = format!("/01/Map/Obj/{}.img", oS);

            // Fetch and parse the object file
            let bytes = AssetManager::fetch_and_cache(&url, &cache_name)
                .await
                .map_err(|e| format!("Failed to fetch object: {}", e))?;

            info!("  Parsing object file (size: {} bytes)...", bytes.len());

            let wz_iv = guess_iv_from_wz_img(&bytes)
                .ok_or_else(|| "Unable to guess version from object file".to_string())?;

            let byte_len = bytes.len();

            let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
            let cache_name_ref: wz_reader::WzNodeName = cache_name.clone().into();
            let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
            let node: WzNodeArc = WzNode::new(&cache_name.into(), wz_image, None).into();

            node.write()
                .unwrap()
                .parse(&node)
                .map_err(|e| format!("Failed to parse object WZ: {:?}", e))?;

            info!("  Object WZ file parsed successfully");

            // Cache the parsed node
            self.wz_nodes.insert(oS.to_string(), node.clone());
            node
        };

        // Navigate to the specific object: l0/l1/l2/0
        // Build path based on which layers are present
        // The PNG is typically at index 0 under the final layer
        let base_path = if !l2.is_empty() {
            format!("{}/{}/{}", l0, l1, l2)
        } else if !l1.is_empty() {
            format!("{}/{}", l0, l1)
        } else if !l0.is_empty() {
            l0.to_string()
        } else {
            return Err("No valid path components".to_string());
        };

        // Try with /0 suffix first (most common case)
        let path_with_zero = format!("{}/0", base_path);
        info!("Navigating to object path: {}", path_with_zero);

        let obj_node = match root_node.read().unwrap().at_path_parsed(&path_with_zero) {
            Ok(node) => node,
            Err(_) => {
                // Fallback: try without the /0 suffix
                info!("Failed with /0, trying base path: {}", base_path);
                root_node
                    .read()
                    .unwrap()
                    .at_path_parsed(&base_path)
                    .map_err(|_| format!("Object not found at path: {} or {}", path_with_zero, base_path))?
            }
        };

        // Extract origin if available
        let obj_read = obj_node.read().unwrap();
        let (origin_x, origin_y) = if let Ok(origin_node) = obj_read.at_path_parsed("origin") {
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
        match &obj_read.object_type {
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
                    "Loaded object {}/{}/{}/{} ({}x{}) with origin ({}, {})",
                    oS, l0, l1, l2, width, height, origin_x, origin_y
                );

                Ok((texture, origin_x, origin_y))
            }
            _ => Err(format!("Object node is not a PNG: {}", path_with_zero)),
        }
    }

    /// Internal helper to get object origin (reuses cached node if available)
    async fn get_object_origin_internal(
        &mut self,
        oS: &str,
        l0: &str,
        l1: &str,
        l2: &str,
    ) -> Result<(i32, i32), String> {
        // Check if we already have this WZ node cached
        let root_node = if let Some(cached_node) = self.wz_nodes.get(oS) {
            cached_node.clone()
        } else {
            let url = format!(
                "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/Map/Obj/{}.img",
                oS
            );
            let cache_name = format!("/01/Map/Obj/{}.img", oS);

            let bytes = AssetManager::fetch_and_cache(&url, &cache_name)
                .await
                .map_err(|e| format!("Failed to fetch object: {}", e))?;

            let wz_iv = guess_iv_from_wz_img(&bytes)
                .ok_or_else(|| "Unable to guess version from object file".to_string())?;

            let byte_len = bytes.len();

            let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
            let cache_name_ref: wz_reader::WzNodeName = cache_name.clone().into();
            let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
            let node: WzNodeArc = WzNode::new(&cache_name.into(), wz_image, None).into();

            node.write()
                .unwrap()
                .parse(&node)
                .map_err(|e| format!("Failed to parse object WZ: {:?}", e))?;

            // Cache the parsed node
            self.wz_nodes.insert(oS.to_string(), node.clone());
            node
        };

        // Navigate to the specific object
        let base_path = if !l2.is_empty() {
            format!("{}/{}/{}", l0, l1, l2)
        } else if !l1.is_empty() {
            format!("{}/{}", l0, l1)
        } else if !l0.is_empty() {
            l0.to_string()
        } else {
            return Err("No valid path components".to_string());
        };

        let path_with_zero = format!("{}/0", base_path);
        let obj_node = match root_node.read().unwrap().at_path_parsed(&path_with_zero) {
            Ok(node) => node,
            Err(_) => {
                root_node
                    .read()
                    .unwrap()
                    .at_path_parsed(&base_path)
                    .map_err(|_| format!("Object not found at path: {} or {}", path_with_zero, base_path))?
            }
        };

        let obj_read = obj_node.read().unwrap();
        if let Ok(origin_node) = obj_read.at_path_parsed("origin") {
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
}
