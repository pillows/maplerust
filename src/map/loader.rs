use macroquad::prelude::*;
use crate::assets::AssetManager;
use crate::map::data::*;
use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzReader, WzObjectType};

#[cfg(not(target_arch = "wasm32"))]
use memmap2::MmapOptions;

pub struct MapLoader;

impl MapLoader {
    /// Helper function to prepare bytes for WzReader
    /// For native: converts Vec<u8> to Mmap
    /// For WASM: returns Vec<u8> directly
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

    /// Load a map from the given map ID
    pub async fn load_map(map_id: &str) -> Result<MapData, String> {
        info!("Loading map: {}", map_id);

        // Determine the map category (Map0, Map1, etc.) from first digit
        let first_digit = map_id.chars().next().unwrap_or('0');
        let map_category = format!("Map{}", first_digit);

        // Build URL for the map file
        let url = format!(
            "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/Map/Map/{}/{}.img",
            map_category, map_id
        );
        let cache_name = format!("/01/Map/Map/{}/{}.img", map_category, map_id);

        // Fetch and parse the map file
        let bytes = AssetManager::fetch_and_cache(&url, &cache_name).await
            .map_err(|e| format!("Failed to fetch map: {}", e))?;

        info!("Parsing map file (size: {} bytes)...", bytes.len());

        let wz_iv = guess_iv_from_wz_img(&bytes)
            .ok_or_else(|| "Unable to guess version from map file".to_string())?;

        let byte_len = bytes.len();
        let wz_data = Self::prepare_wz_data(bytes)?;

        let reader = Arc::new(WzReader::new(wz_data).with_iv(wz_iv));
        let cache_name_ref: wz_reader::WzNodeName = cache_name.clone().into();
        let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
        let root_node: WzNodeArc = WzNode::new(&cache_name.into(), wz_image, None).into();

        root_node.write().unwrap().parse(&root_node)
            .map_err(|e| format!("Failed to parse map WZ: {:?}", e))?;

        info!("Map WZ file parsed successfully");

        // Parse map data
        let mut map_data = MapData::new();

        // Parse info section
        if let Ok(info_node) = root_node.read().unwrap().at_path_parsed("info") {
            Self::parse_info(&info_node, &mut map_data)?;
        }

        // Parse background layers
        if let Ok(back_node) = root_node.read().unwrap().at_path_parsed("back") {
            Self::parse_backgrounds(&back_node, &mut map_data).await?;
        }

        // Parse footholds
        if let Ok(fh_node) = root_node.read().unwrap().at_path_parsed("foothold") {
            Self::parse_footholds(&fh_node, &mut map_data)?;
        }

        // Parse portals
        if let Ok(portal_node) = root_node.read().unwrap().at_path_parsed("portal") {
            Self::parse_portals(&portal_node, &mut map_data)?;
        }

        // Parse life (NPCs/mobs)
        if let Ok(life_node) = root_node.read().unwrap().at_path_parsed("life") {
            Self::parse_life(&life_node, &mut map_data)?;
        }

        // Parse ladders/ropes
        if let Ok(ladder_node) = root_node.read().unwrap().at_path_parsed("ladderRope") {
            Self::parse_ladders(&ladder_node, &mut map_data)?;
        }

        info!("Map loaded: {} backgrounds, {} footholds, {} portals",
              map_data.backgrounds.len(), map_data.footholds.len(), map_data.portals.len());

        Ok(map_data)
    }

    /// Parse map info section
    fn parse_info(node: &WzNodeArc, map_data: &mut MapData) -> Result<(), String> {
        let node_read = node.read().unwrap();

        map_data.info.version = Self::get_int_property(&node_read, "version").unwrap_or(0);
        map_data.info.cloud = Self::get_int_property(&node_read, "cloud").unwrap_or(0) == 1;
        map_data.info.town = Self::get_int_property(&node_read, "town").unwrap_or(0) == 1;
        map_data.info.swim = Self::get_int_property(&node_read, "swim").unwrap_or(0) == 1;
        map_data.info.fly = Self::get_int_property(&node_read, "fly").unwrap_or(0) == 1;
        map_data.info.no_map_cmd = Self::get_int_property(&node_read, "noMapCmd").unwrap_or(0) == 1;
        map_data.info.hide_minimap = Self::get_int_property(&node_read, "hideMinimap").unwrap_or(0) == 1;

        map_data.info.return_map = Self::get_int_property(&node_read, "returnMap").unwrap_or(999999999);
        map_data.info.forced_return = Self::get_int_property(&node_read, "forcedReturn").unwrap_or(999999999);
        map_data.info.mob_rate = Self::get_float_property(&node_read, "mobRate").unwrap_or(1.0);
        map_data.info.field_limit = Self::get_int_property(&node_read, "fieldLimit").unwrap_or(0);

        map_data.info.vr_top = Self::get_int_property(&node_read, "VRTop").unwrap_or(0);
        map_data.info.vr_left = Self::get_int_property(&node_read, "VRLeft").unwrap_or(0);
        map_data.info.vr_bottom = Self::get_int_property(&node_read, "VRBottom").unwrap_or(600);
        map_data.info.vr_right = Self::get_int_property(&node_read, "VRRight").unwrap_or(800);

        map_data.info.bgm = Self::get_string_property(&node_read, "bgm").unwrap_or_default();
        map_data.info.map_mark = Self::get_string_property(&node_read, "mapMark").unwrap_or_default();
        map_data.info.on_first_user_enter = Self::get_string_property(&node_read, "onFirstUserEnter").unwrap_or_default();
        map_data.info.on_user_enter = Self::get_string_property(&node_read, "onUserEnter").unwrap_or_default();

        info!("Map bounds: ({}, {}) to ({}, {})",
              map_data.info.vr_left, map_data.info.vr_top,
              map_data.info.vr_right, map_data.info.vr_bottom);

        Ok(())
    }

    /// Parse background layers
    async fn parse_backgrounds(node: &WzNodeArc, map_data: &mut MapData) -> Result<(), String> {
        let node_read = node.read().unwrap();
        let children = node_read.children.clone();

        for (_name, child) in children.iter() {
            let child_read = child.read().unwrap();

            // Parse layer number from child name
            if let Ok(layer_num) = child_read.name.parse::<i32>() {
                drop(child_read);

                let mut bg = BackgroundLayer {
                    layer_num,
                    bS: Self::get_string_property_from_node(child, "bS").unwrap_or_default(),
                    ani: Self::get_int_property_from_node(child, "ani").unwrap_or(0) == 1,
                    no: Self::get_int_property_from_node(child, "no").unwrap_or(0),
                    x: Self::get_int_property_from_node(child, "x").unwrap_or(0),
                    y: Self::get_int_property_from_node(child, "y").unwrap_or(0),
                    rx: Self::get_int_property_from_node(child, "rx").unwrap_or(0),
                    ry: Self::get_int_property_from_node(child, "ry").unwrap_or(0),
                    cx: Self::get_int_property_from_node(child, "cx").unwrap_or(0),
                    cy: Self::get_int_property_from_node(child, "cy").unwrap_or(0),
                    a: Self::get_int_property_from_node(child, "a").unwrap_or(255),
                    front: Self::get_int_property_from_node(child, "front").unwrap_or(0) == 1,
                    flip_x: Self::get_int_property_from_node(child, "f").unwrap_or(0) == 1,
                    flip_y: false,
                    texture: None,
                };

                // TODO: Load actual background texture from Back.img
                // For now, we'll skip texture loading to avoid complexity

                map_data.backgrounds.push(bg);
            }
        }

        // Sort backgrounds by layer number
        map_data.backgrounds.sort_by_key(|bg| bg.layer_num);

        Ok(())
    }

    /// Parse footholds (platforms)
    fn parse_footholds(node: &WzNodeArc, map_data: &mut MapData) -> Result<(), String> {
        let node_read = node.read().unwrap();

        // Footholds are organized as: foothold -> layer -> group -> individual footholds
        for (_layer_name, layer_child) in node_read.children.iter() {
            let layer_read = layer_child.read().unwrap();
            let layer_num = layer_read.name.parse::<i32>().unwrap_or(0);

            for (_group_name, group_child) in layer_read.children.iter() {
                let group_read = group_child.read().unwrap();
                let group_num = group_read.name.parse::<i32>().unwrap_or(0);

                for (_fh_name, fh_child) in group_read.children.iter() {
                    let fh_read = fh_child.read().unwrap();
                    let fh_id = fh_read.name.parse::<i32>().unwrap_or(0);

                    drop(fh_read);

                    let foothold = Foothold {
                        id: fh_id,
                        layer: layer_num,
                        group: group_num,
                        x1: Self::get_int_property_from_node(fh_child, "x1").unwrap_or(0),
                        y1: Self::get_int_property_from_node(fh_child, "y1").unwrap_or(0),
                        x2: Self::get_int_property_from_node(fh_child, "x2").unwrap_or(0),
                        y2: Self::get_int_property_from_node(fh_child, "y2").unwrap_or(0),
                        prev: Self::get_int_property_from_node(fh_child, "prev").unwrap_or(0),
                        next: Self::get_int_property_from_node(fh_child, "next").unwrap_or(0),
                        piece: Self::get_int_property_from_node(fh_child, "piece").unwrap_or(0),
                    };

                    map_data.footholds.push(foothold);
                }
            }
        }

        Ok(())
    }

    /// Parse portals
    fn parse_portals(node: &WzNodeArc, map_data: &mut MapData) -> Result<(), String> {
        let node_read = node.read().unwrap();

        for (_name, child) in node_read.children.iter() {
            let child_read = child.read().unwrap();
            let portal_id = child_read.name.parse::<i32>().unwrap_or(0);
            drop(child_read);

            let portal = Portal {
                id: portal_id,
                pn: Self::get_string_property_from_node(child, "pn").unwrap_or_default(),
                pt: Self::get_int_property_from_node(child, "pt").unwrap_or(0),
                x: Self::get_int_property_from_node(child, "x").unwrap_or(0),
                y: Self::get_int_property_from_node(child, "y").unwrap_or(0),
                tm: Self::get_int_property_from_node(child, "tm").unwrap_or(999999999),
                tn: Self::get_string_property_from_node(child, "tn").unwrap_or_default(),
                script: Self::get_string_property_from_node(child, "script").unwrap_or_default(),
                horizontal_impact: Self::get_int_property_from_node(child, "horizontalImpact").unwrap_or(0),
                vertical_impact: Self::get_int_property_from_node(child, "verticalImpact").unwrap_or(0),
            };

            map_data.portals.push(portal);
        }

        Ok(())
    }

    /// Parse life (NPCs and mobs)
    fn parse_life(node: &WzNodeArc, map_data: &mut MapData) -> Result<(), String> {
        let node_read = node.read().unwrap();

        for (_name, child) in node_read.children.iter() {
            let child_read = child.read().unwrap();
            drop(child_read);

            let life = Life {
                id: Self::get_string_property_from_node(child, "id").unwrap_or_default(),
                life_type: Self::get_string_property_from_node(child, "type").unwrap_or_default(),
                x: Self::get_int_property_from_node(child, "x").unwrap_or(0),
                y: Self::get_int_property_from_node(child, "y").unwrap_or(0),
                foothold: Self::get_int_property_from_node(child, "fh").unwrap_or(0),
                cx: Self::get_int_property_from_node(child, "cx").unwrap_or(0),
                cy: Self::get_int_property_from_node(child, "cy").unwrap_or(0),
                rx0: Self::get_int_property_from_node(child, "rx0").unwrap_or(0),
                rx1: Self::get_int_property_from_node(child, "rx1").unwrap_or(0),
                mob_time: Self::get_int_property_from_node(child, "mobTime").unwrap_or(0),
                flip: Self::get_int_property_from_node(child, "f").unwrap_or(0) == 1,
                hide: Self::get_int_property_from_node(child, "hide").unwrap_or(0) == 1,
            };

            map_data.life.push(life);
        }

        Ok(())
    }

    /// Parse ladders and ropes
    fn parse_ladders(node: &WzNodeArc, map_data: &mut MapData) -> Result<(), String> {
        let node_read = node.read().unwrap();

        for (_name, child) in node_read.children.iter() {
            let child_read = child.read().unwrap();
            let ladder_id = child_read.name.parse::<i32>().unwrap_or(0);
            drop(child_read);

            let ladder = Ladder {
                id: ladder_id,
                x: Self::get_int_property_from_node(child, "x").unwrap_or(0),
                y1: Self::get_int_property_from_node(child, "y1").unwrap_or(0),
                y2: Self::get_int_property_from_node(child, "y2").unwrap_or(0),
                ladder: Self::get_int_property_from_node(child, "l").unwrap_or(1) == 1,
                page: Self::get_int_property_from_node(child, "page").unwrap_or(0),
            };

            map_data.ladders.push(ladder);
        }

        Ok(())
    }

    // Helper methods to extract property values
    fn get_int_property(node: &wz_reader::WzNode, key: &str) -> Option<i32> {
        node.at_path(key).and_then(|prop_node| {
            let prop_read = prop_node.read().unwrap();
            match &prop_read.object_type {
                WzObjectType::Value(wz_reader::property::WzValue::Short(val)) => Some(*val as i32),
                WzObjectType::Value(wz_reader::property::WzValue::Int(val)) => Some(*val),
                WzObjectType::Value(wz_reader::property::WzValue::Long(val)) => Some(*val as i32),
                _ => None,
            }
        })
    }

    fn get_float_property(node: &wz_reader::WzNode, key: &str) -> Option<f32> {
        node.at_path(key).and_then(|prop_node| {
            let prop_read = prop_node.read().unwrap();
            match &prop_read.object_type {
                WzObjectType::Value(wz_reader::property::WzValue::Float(val)) => Some(*val),
                WzObjectType::Value(wz_reader::property::WzValue::Double(val)) => Some(*val as f32),
                _ => None,
            }
        })
    }

    fn get_string_property(node: &wz_reader::WzNode, key: &str) -> Option<String> {
        node.at_path(key).and_then(|prop_node| {
            let prop_read = prop_node.read().unwrap();
            match &prop_read.object_type {
                WzObjectType::Value(wz_reader::property::WzValue::String(val)) => val.get_string().ok(),
                _ => None,
            }
        })
    }

    fn get_int_property_from_node(node: &WzNodeArc, key: &str) -> Option<i32> {
        let node_read = node.read().unwrap();
        Self::get_int_property(&*node_read, key)
    }

    fn get_string_property_from_node(node: &WzNodeArc, key: &str) -> Option<String> {
        let node_read = node.read().unwrap();
        Self::get_string_property(&*node_read, key)
    }
}
