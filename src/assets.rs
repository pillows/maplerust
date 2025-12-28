use macroquad::prelude::*;
use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzReader, WzNodeCast};

use wz_reader::WzObjectType;

#[cfg(not(target_arch = "wasm32"))]
use memmap2::MmapOptions;

/// Structure to hold an animation frame with its texture and origin coordinates
#[derive(Clone)]
pub struct FrameData {
    pub texture: Texture2D,
    pub origin: Vec2, // Origin coordinates (x, y) from the WZ file
}

#[cfg(target_arch = "wasm32")]
extern "C" {
    fn idb_save(name_ptr: *const u8, name_len: u32, data_ptr: *const u8, data_len: u32);
    fn console_save(
        filename_ptr: *const u8,
        filename_len: u32,
        content_ptr: *const u8,
        content_len: u32,
    );
}

pub struct AssetManager;

impl AssetManager {
    /// Helper function to prepare bytes for WzReader
    #[cfg(not(target_arch = "wasm32"))]
    fn prepare_wz_data(bytes: Vec<u8>) -> Result<memmap2::Mmap, String> {
        let mut mmap = MmapOptions::new()
            .len(bytes.len())
            .map_anon()
            .map_err(|e| format!("Failed to create anonymous mmap: {}", e))?;

        mmap.copy_from_slice(&bytes);

        mmap.make_read_only()
            .map_err(|e| format!("Failed to make mmap read-only: {}", e))
    }

    #[cfg(target_arch = "wasm32")]
    fn prepare_wz_data(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
        Ok(bytes)
    }

    pub async fn fetch_and_cache(url: &str, cache_path: &str) -> Result<Vec<u8>, String> {
        let idb_url = format!("idb://{}", cache_path);
        info!("Checking cache at: {}", idb_url);

        // Try loading from IndexedDB first
        if let Ok(bytes) = load_file(&idb_url).await {
            if bytes.len() > 0 {
                info!("Asset found in IndexedDB!");
                return Ok(bytes);
            }
        }

        info!("Asset NOT found in DB. Fetching from URL: {}", url);

        // Fallback to HTTP
        match load_file(url).await {
            Ok(bytes) => {
                info!(
                    "Asset loaded from URL. Saving to IndexedDB at '{}'...",
                    cache_path
                );
                #[cfg(target_arch = "wasm32")]
                unsafe {
                    idb_save(
                        cache_path.as_ptr(),
                        cache_path.len() as u32,
                        bytes.as_ptr(),
                        bytes.len() as u32,
                    );
                }
                Ok(bytes)
            }
            Err(e) => Err(format!("Failed to load asset: {:?}", e)),
        }
    }

    pub async fn load_texture(url: &str, cache_path: &str) -> Result<Texture2D, String> {
        let bytes = Self::fetch_and_cache(url, cache_path).await?;
        Ok(Texture2D::from_file_with_format(
            &bytes,
            Some(ImageFormat::Png),
        ))
    }

    pub async fn load_wz_img_from_url(url: &str, cache_path: &str) -> Result<(), String> {
        let bytes = Self::fetch_and_cache(url, cache_path).await?;

        info!("Parsing .img file, size: {} bytes", bytes.len());

        // Guess Version / IV
        let wz_iv = guess_iv_from_wz_img(&bytes).ok_or("Unable to guess version from img file")?;

        // Create Reader
        let byte_len = bytes.len();
        let wz_data = Self::prepare_wz_data(bytes)?;
        let reader = Arc::new(WzReader::new(wz_data).with_iv(wz_iv));

        // Create WzImage
        // Derive node name from cache_path
        let name = std::path::Path::new(cache_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.img");

        let _wz_image = WzImage::new(&name.into(), 0, byte_len, &reader);

        // Create Root Node
        let _node: WzNodeArc = WzNode::new(&name.into(), _wz_image, None).into();

        // Log generation disabled to prevent browser crashes
        /*
        // Traverse and Log
        info!("Generating WZ structure log...");
        let mut log_output = String::new();
        log_output.push_str("--- WZ Structure Start ---\n");

        walk_node(&node, true, &mut |n: &WzNodeArc| {
            let read = n.read().unwrap();
            let path = read.get_full_path();
            let type_name = match &read.object_type {
                WzObjectType::Directory(_) => "Directory",
                WzObjectType::Image(_) => "Image",
                WzObjectType::File(_) => "File",
                WzObjectType::MsFile(_) => "MsFile",
                WzObjectType::MsImage(_) => "MsImage",
                WzObjectType::Property(p) => match p {
                    wz_reader::property::WzSubProperty::Convex => "Convex",
                    wz_reader::property::WzSubProperty::Sound(_) => "Sound",
                    wz_reader::property::WzSubProperty::PNG(_) => "PNG",
                    wz_reader::property::WzSubProperty::Property => "Property",
                },
                WzObjectType::Value(v) => match v {
                    wz_reader::property::WzValue::Null => "Null",
                    wz_reader::property::WzValue::Short(_) => "Short",
                    wz_reader::property::WzValue::Int(_) => "Int",
                    wz_reader::property::WzValue::Long(_) => "Long",
                    wz_reader::property::WzValue::Float(_) => "Float",
                    wz_reader::property::WzValue::Double(_) => "Double",
                    wz_reader::property::WzValue::String(_) => "String",
                    wz_reader::property::WzValue::ParsedString(_) => "ParsedString",
                    wz_reader::property::WzValue::Vector(_) => "Vector",
                    wz_reader::property::WzValue::UOL(_) => "UOL",
                    wz_reader::property::WzValue::RawData(_) => "RawData",
                    wz_reader::property::WzValue::Video(_) => "Video",
                    wz_reader::property::WzValue::Lua(_) => "Lua",
                },
            };
            log_output.push_str(&format!("Node: {} [{}]\n", path, type_name));
        });
        log_output.push_str("--- WZ Structure End ---\n");

        info!(
            "Structure log generated ({} bytes). Saving file...",
            log_output.len()
        );

        let filename = format!("{}_structure.txt", name);
        unsafe {
            console_save(
                filename.as_ptr(),
                filename.len() as u32,
                log_output.as_ptr(),
                log_output.len() as u32,
            );
        }
        */

        info!("WZ img parsing complete (logging disabled)");
        Ok(())
    }

    pub async fn get_wz_child_names(
        url: &str,
        cache_path: &str,
        node_path: &str,
    ) -> Result<Vec<String>, String> {
        let bytes = Self::fetch_and_cache(url, cache_path).await?;

        info!(
            "Parsing .img file to get child names, size: {} bytes",
            bytes.len()
        );

        // Guess Version / IV
        let wz_iv = guess_iv_from_wz_img(&bytes).ok_or("Unable to guess version from img file")?;

        // Create Reader
        let byte_len = bytes.len();
        let wz_data = Self::prepare_wz_data(bytes)?;
        let reader = Arc::new(WzReader::new(wz_data).with_iv(wz_iv));

        // Create WzImage
        let name = std::path::Path::new(cache_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.img");

        let wz_image = WzImage::new(&name.into(), 0, byte_len, &reader);

        // Create Root Node
        let node: WzNodeArc = WzNode::new(&name.into(), wz_image, None).into();

        // Parse the node to populate children
        node.write()
            .unwrap()
            .parse(&node)
            .map_err(|e| format!("Failed to parse WZ node: {:?}", e))?;

        // Navigate to the target path
        info!("Navigating to path: {}", node_path);
        let target_node = node
            .read()
            .unwrap()
            .at_path(node_path)
            .ok_or_else(|| format!("Node not found at path: {}", node_path))?;

        // Parse the target node to ensure children are loaded
        target_node
            .write()
            .unwrap()
            .parse(&target_node)
            .map_err(|e| format!("Failed to parse target node: {:?}", e))?;

        // Get all child names
        let target_read = target_node.read().unwrap();
        let child_names: Vec<String> = target_read
            .children
            .keys()
            .map(|name| name.to_string())
            .collect();

        info!(
            "Found {} children at path '{}'",
            child_names.len(),
            node_path
        );
        Ok(child_names)
    }

    pub async fn load_wz_png_texture(
        url: &str,
        cache_path: &str,
        node_path: &str,
    ) -> Result<Texture2D, String> {
        let bytes = Self::fetch_and_cache(url, cache_path).await?;

        info!(
            "Parsing .img file for PNG extraction, size: {} bytes",
            bytes.len()
        );

        // Guess Version / IV
        let wz_iv = guess_iv_from_wz_img(&bytes).ok_or("Unable to guess version from img file")?;

        // Create Reader
        let byte_len = bytes.len();
        let wz_data = Self::prepare_wz_data(bytes)?;
        let reader = Arc::new(WzReader::new(wz_data).with_iv(wz_iv));

        // Create WzImage
        let name = std::path::Path::new(cache_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.img");

        let wz_image = WzImage::new(&name.into(), 0, byte_len, &reader);

        // Create Root Node
        let node: WzNodeArc = WzNode::new(&name.into(), wz_image, None).into();

        // Parse the node to populate children
        node.write()
            .unwrap()
            .parse(&node)
            .map_err(|e| format!("Failed to parse WZ node: {:?}", e))?;

        // Navigate to the target path
        info!("Navigating to path: {}", node_path);
        let target_node = node
            .read()
            .unwrap()
            .at_path(node_path)
            .ok_or_else(|| format!("Node not found at path: {}", node_path))?;

        // Parse the target node to ensure it's loaded
        target_node
            .write()
            .unwrap()
            .parse(&target_node)
            .map_err(|e| format!("Failed to parse target node: {:?}", e))?;

        // Extract PNG data
        let target_read = target_node.read().unwrap();
        match &target_read.object_type {
            WzObjectType::Property(wz_reader::property::WzSubProperty::PNG(png_data)) => {
                info!("Found PNG: {}x{}", png_data.width, png_data.height);

                // Get the DynamicImage
                let dynamic_img = png_data
                    .extract_png()
                    .map_err(|e| format!("Failed to extract PNG: {:?}", e))?;

                // Convert to RGBA8 bytes
                let rgba_img = dynamic_img.to_rgba8();
                let width = rgba_img.width() as u16;
                let height = rgba_img.height() as u16;
                let bytes = rgba_img.into_raw();

                // Convert to Macroquad texture directly from RGBA bytes
                let texture = Texture2D::from_rgba8(width, height, &bytes);

                info!("Successfully loaded PNG texture");
                Ok(texture)
            }
            _ => Err(format!("Node at path '{}' is not a PNG", node_path)),
        }
    }

    /// Get origin coordinates from a frame path (e.g., "Wizet/6/origin")
    async fn get_frame_origin(
        url: &str,
        cache_path: &str,
        origin_path: &str,
    ) -> Result<Vec2, String> {
        let bytes = Self::fetch_and_cache(url, cache_path).await?;

        // Guess Version / IV
        let wz_iv = guess_iv_from_wz_img(&bytes).ok_or("Unable to guess version from img file")?;

        // Create Reader
        let byte_len = bytes.len();
        let wz_data = Self::prepare_wz_data(bytes)?;
        let reader = Arc::new(WzReader::new(wz_data).with_iv(wz_iv));

        // Create WzImage
        let name = std::path::Path::new(cache_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.img");

        let wz_image = WzImage::new(&name.into(), 0, byte_len, &reader);

        // Create Root Node
        let node: WzNodeArc = WzNode::new(&name.into(), wz_image, None).into();

        // Parse the node to populate children
        node.write()
            .unwrap()
            .parse(&node)
            .map_err(|e| format!("Failed to parse WZ node: {:?}", e))?;

        // Navigate to the origin path (at_path_parsed automatically parses nodes along the path)
        let origin_node = node
            .read()
            .unwrap()
            .at_path_parsed(origin_path)
            .map_err(|e| format!("Failed to navigate to origin path '{}': {:?}", origin_path, e))?;

        // Extract Vector2D from the origin node
        let origin_read = origin_node.read().unwrap();
        if let Some(vector) = origin_read.try_as_vector2d() {
            Ok(vec2(vector.0 as f32, vector.1 as f32))
        } else {
            // Default to (0, 0) if origin is not found
            Ok(vec2(0.0, 0.0))
        }
    }

    /// Load all frames for a given animation path from a WZ file
    /// Discovers frame names, filters to numeric frames, sorts them, and loads each as a texture
    pub async fn load_animation_frames(
        base_url: &str,
        cache_name: &str,
        base_path: &str,
    ) -> Vec<Texture2D> {
        // First, discover all available frame names
        info!("Discovering animation frames for {}...", base_path);
        let frame_names = match Self::get_wz_child_names(base_url, cache_name, base_path).await {
            Ok(names) => {
                // Filter to only PNG nodes (exclude origin, z, etc.)
                let mut png_frames: Vec<String> = names
                    .into_iter()
                    .filter(|name| name.parse::<i32>().is_ok())
                    .collect();

                // Sort numerically for proper animation order
                png_frames.sort_by_key(|name| name.parse::<i32>().unwrap_or(0));

                info!("Found {} frames for {}", png_frames.len(), base_path);
                png_frames
            }
            Err(e) => {
                error!("Failed to discover frames for {}: {}", base_path, e);
                Vec::new()
            }
        };

        // Now load all the frames
        let mut frames: Vec<Texture2D> = Vec::new();
        for frame_name in &frame_names {
            let frame_path = format!("{}/{}", base_path, frame_name);
            match Self::load_wz_png_texture(base_url, cache_name, &frame_path).await {
                Ok(tex) => {
                    frames.push(tex);
                }
                Err(e) => {
                    error!("Failed to load frame {}: {}", frame_path, e);
                }
            }
        }

        info!("Loaded {} frames for {}", frames.len(), base_path);
        frames
    }

    /// Load a single frame from a parsed WZ node structure (synchronous, no async needed)
    fn load_frame_from_node(
        node: &WzNodeArc,
        frame_path: &str,
        origin_path: &str,
    ) -> Result<FrameData, String> {
        // Extract PNG texture first
        let frame_node = {
            let node_read = node.read().unwrap();
            node_read
                .at_path_parsed(frame_path)
                .map_err(|e| format!("Failed to navigate to frame path '{}': {:?}", frame_path, e))?
        };
        
        // Parse the frame node
        frame_node
            .write()
            .unwrap()
            .parse(&frame_node)
            .map_err(|e| format!("Failed to parse frame node: {:?}", e))?;

        // Extract PNG texture data
        let texture = {
            let frame_read = frame_node.read().unwrap();
            match &frame_read.object_type {
                WzObjectType::Property(wz_reader::property::WzSubProperty::PNG(png_data)) => {
                    let dynamic_img = png_data
                        .extract_png()
                        .map_err(|e| format!("Failed to extract PNG: {:?}", e))?;
                    let rgba_img = dynamic_img.to_rgba8();
                    let width = rgba_img.width() as u16;
                    let height = rgba_img.height() as u16;
                    let bytes = rgba_img.into_raw();
                    Texture2D::from_rgba8(width, height, &bytes)
                }
                _ => return Err(format!("Node at path '{}' is not a PNG", frame_path)),
            }
        };
        
        // Drop frame_node before accessing the root node again
        drop(frame_node);

        // Extract origin coordinates (now we can safely access the root node again)
        let origin = {
            let node_read = node.read().unwrap();
            node_read
                .at_path_parsed(origin_path)
                .ok()
                .and_then(|origin_node| {
                    origin_node
                        .read()
                        .unwrap()
                        .try_as_vector2d()
                        .map(|vec| vec2(vec.0 as f32, vec.1 as f32))
                })
                .unwrap_or(vec2(0.0, 0.0))
        };

        Ok(FrameData { texture, origin })
    }

    /// Load all frames for a given animation path from a WZ file with origin coordinates
    /// Optimized version: parses WZ file once, then loads all frames in parallel
    pub async fn load_animation_frames_with_origins(
        base_url: &str,
        cache_name: &str,
        base_path: &str,
    ) -> Vec<FrameData> {
        // Fetch and cache the WZ file once
        let bytes = match Self::fetch_and_cache(base_url, cache_name).await {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Failed to fetch WZ file: {}", e);
                return Vec::new();
            }
        };

        // Parse WZ file once
        let wz_iv = match guess_iv_from_wz_img(&bytes) {
            Some(iv) => iv,
            None => {
                error!("Unable to guess version from img file");
                return Vec::new();
            }
        };

        let byte_len = bytes.len();
        let wz_data = match Self::prepare_wz_data(bytes) {
            Ok(d) => d,
            Err(e) => {
                error!("Failed to prepare WZ data: {}", e);
                return Vec::new();
            }
        };
        let reader = Arc::new(WzReader::new(wz_data).with_iv(wz_iv));
        let name = std::path::Path::new(cache_name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.img");

        let wz_image = WzImage::new(&name.into(), 0, byte_len, &reader);
        let root_node: WzNodeArc = WzNode::new(&name.into(), wz_image, None).into();

        // Parse root node once
        if let Err(e) = root_node.write().unwrap().parse(&root_node) {
            error!("Failed to parse WZ node: {:?}", e);
            return Vec::new();
        }

        // Discover frame names from already-parsed node (much faster than re-parsing)
        info!("Discovering animation frames with origins for {}...", base_path);
        let target_node = root_node
            .read()
            .unwrap()
            .at_path_parsed(base_path);
        
        let frame_names = match target_node {
            Ok(node) => {
                let node_read = node.read().unwrap();
                let mut png_frames: Vec<String> = node_read
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
                png_frames.sort_by_key(|name| name.parse::<i32>().unwrap_or(0));
                info!("Found {} frames for {}", png_frames.len(), base_path);
                png_frames
            }
            Err(e) => {
                error!("Failed to discover frames for {}: {:?}", base_path, e);
                return Vec::new();
            }
        };

        // Load all frames sequentially but much faster since we've already parsed the WZ file
        // The expensive parsing step is done once above, so this is just navigation and extraction
        let mut frames: Vec<FrameData> = Vec::new();
        for frame_name in &frame_names {
            let frame_path = format!("{}/{}", base_path, frame_name);
            let origin_path = format!("{}/origin", frame_path);
            
            match Self::load_frame_from_node(&root_node, &frame_path, &origin_path) {
                Ok(frame_data) => {
                    frames.push(frame_data);
                }
                Err(e) => {
                    error!("Failed to load frame {}: {}", frame_path, e);
                }
            }
        }

        info!("Loaded {} frames with origins for {}", frames.len(), base_path);
        frames
    }

    /// Display an asset from a WZ .img file at the specified coordinates
    /// Fetches the .img file if not in IndexedDB cache, loads the PNG texture from the specified directory path,
    /// and returns the texture so it can be drawn using draw_texture(texture, x, y, WHITE)
    /// 
    /// # Parameters
    /// - `base_url`: The full URL to the .img file (e.g., "https://example.com/path/Logo.img")
    /// - `img_name`: The filename to use as the cache key (e.g., "Logo.img")
    /// - `directory`: The node path within the WZ file to the PNG (e.g., "Nexon/0" or "Nexon")
    /// - `x`: X coordinate for drawing
    /// - `y`: Y coordinate for drawing
    /// 
    /// # Returns
    /// Returns `Ok(Texture2D)` if successful, or `Err(String)` if there was an error
    /// 
    /// # Example
    /// ```rust
    /// let texture = AssetManager::display_asset(
    ///     "https://example.com/Logo.img",
    ///     "Logo.img",
    ///     "Nexon/0",
    ///     100.0,
    ///     200.0
    /// ).await?;
    /// draw_texture(&texture, 100.0, 200.0, WHITE);
    /// ```
    #[allow(unused_variables)] // x and y are part of the API contract for documentation purposes
    pub async fn display_asset(
        base_url: &str,
        img_name: &str,
        directory: &str,
        x: f32,
        y: f32,
    ) -> Result<Texture2D, String> {
        // Load the PNG texture from the WZ file (fetches from cache or URL if needed)
        let texture = Self::load_wz_png_texture(base_url, img_name, directory).await?;
        
        // Note: Drawing must be done synchronously in the render loop
        // The caller should use: draw_texture(&texture, x, y, WHITE)
        
        Ok(texture)
    }
}
