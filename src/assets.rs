use macroquad::prelude::*;
use std::sync::Arc;
use wz_reader::util::walk_node;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzReader};

use wz_reader::WzObjectType;

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
        // WASM fix: Mmap logic removed from library, WzReader now accepts Vec<u8>
        let reader = Arc::new(WzReader::new(bytes.clone()).with_iv(wz_iv));

        // Create WzImage
        // Derive node name from cache_path
        let name = std::path::Path::new(cache_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.img");

        let _wz_image = WzImage::new(&name.into(), 0, bytes.len(), &reader);

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
        let reader = Arc::new(WzReader::new(bytes.clone()).with_iv(wz_iv));

        // Create WzImage
        let name = std::path::Path::new(cache_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.img");

        let wz_image = WzImage::new(&name.into(), 0, bytes.len(), &reader);

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
        let reader = Arc::new(WzReader::new(bytes.clone()).with_iv(wz_iv));

        // Create WzImage
        let name = std::path::Path::new(cache_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.img");

        let wz_image = WzImage::new(&name.into(), 0, bytes.len(), &reader);

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
}
