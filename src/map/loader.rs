use macroquad::prelude::*;
use crate::assets::AssetManager;
use crate::map::data::*;
use crate::map::tile_loader::TileCache;
use crate::map::object_loader::ObjectCache;
use crate::map::npc_loader::NpcCache;
use crate::map::mob_loader::MobCache;
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
        #[cfg(not(target_arch = "wasm32"))]
        use std::time::Instant;

        #[cfg(not(target_arch = "wasm32"))]
        let total_start = Instant::now();

        info!("=== LOADING MAP: {} ===", map_id);

        // Break down the map ID and show its components
        info!("Map ID breakdown:");
        info!("  Full ID: {}", map_id);

        // Determine the map category (Map0, Map1, etc.) from first digit
        let first_digit = map_id.chars().next().unwrap_or('0');
        let map_category = format!("Map{}", first_digit);
        info!("  First digit: {} -> Category: {}", first_digit, map_category);

        // Show the map ID components if it's 9 digits (standard format)
        if map_id.len() == 9 {
            let region = &map_id[0..3];
            let area = &map_id[3..5];
            let map_num = &map_id[5..9];
            info!("  Region code: {}", region);
            info!("  Area code: {}", area);
            info!("  Map number: {}", map_num);
        } else {
            info!("  Non-standard map ID length: {} characters", map_id.len());
        }

        // Build URL for the map file
        let url = format!(
            "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/Map/Map/{}/{}.img",
            map_category, map_id
        );
        let cache_name = format!("/01/Map/Map/{}/{}.img", map_category, map_id);

        info!("  Loading from: Map/Map/{}/{}.img", map_category, map_id);
        info!("  Full URL: {}", url);

        // Fetch and parse the map file
        #[cfg(not(target_arch = "wasm32"))]
        let fetch_start = Instant::now();

        let bytes = AssetManager::fetch_and_cache(&url, &cache_name).await
            .map_err(|e| format!("Failed to fetch map: {}", e))?;

        #[cfg(not(target_arch = "wasm32"))]
        let fetch_time = fetch_start.elapsed();

        info!("Parsing map file (size: {} bytes)...", bytes.len());

        let wz_iv = guess_iv_from_wz_img(&bytes)
            .ok_or_else(|| "Unable to guess version from map file".to_string())?;

        let byte_len = bytes.len();

        let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
        let cache_name_ref: wz_reader::WzNodeName = cache_name.clone().into();
        let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
        let root_node: WzNodeArc = WzNode::new(&cache_name.into(), wz_image, None).into();

        root_node.write().unwrap().parse(&root_node)
            .map_err(|e| format!("Failed to parse map WZ: {:?}", e))?;

        info!("Map WZ file parsed successfully");

        // Console log the raw map structure for debugging
        // Self::debug_log_map_structure(&root_node);

        // Parse map data
        let mut map_data = MapData::new();

        // Load map name from String/Map.img
        map_data.info.map_name = Self::get_map_name(map_id).await.unwrap_or_default();
        if !map_data.info.map_name.is_empty() {
            info!("Map name: {}", map_data.info.map_name);
        }

        // Parse info section
        if let Ok(info_node) = root_node.read().unwrap().at_path_parsed("info") {
            Self::parse_info(&info_node, &mut map_data)?;
        }

        // Parse background layers
        #[cfg(not(target_arch = "wasm32"))]
        let bg_start = Instant::now();

        if let Ok(back_node) = root_node.read().unwrap().at_path_parsed("back") {
            Self::parse_backgrounds(&back_node, &mut map_data).await?;
        }

        #[cfg(not(target_arch = "wasm32"))]
        let bg_time = bg_start.elapsed();

        // Parse footholds
        if let Ok(fh_node) = root_node.read().unwrap().at_path_parsed("foothold") {
            // Ensure foothold node is fully parsed
            fh_node.write().unwrap().parse(&fh_node)
                .map_err(|e| format!("Failed to parse foothold node: {:?}", e))?;
            Self::parse_footholds(&fh_node, &mut map_data)?;
            info!("Parsed {} footholds", map_data.footholds.len());
        } else {
            warn!("No foothold node found in map");
        }

        // Parse portals
        if let Ok(portal_node) = root_node.read().unwrap().at_path_parsed("portal") {
            Self::parse_portals(&portal_node, &mut map_data).await?;
        }

        // Parse life (NPCs/mobs)
        #[cfg(not(target_arch = "wasm32"))]
        let life_start = Instant::now();

        if let Ok(life_node) = root_node.read().unwrap().at_path_parsed("life") {
            Self::parse_life(&life_node, &mut map_data).await?;
        }

        #[cfg(not(target_arch = "wasm32"))]
        let life_time = life_start.elapsed();

        // Parse ladders/ropes
        if let Ok(ladder_node) = root_node.read().unwrap().at_path_parsed("ladderRope") {
            Self::parse_ladders(&ladder_node, &mut map_data)?;
        }

        // Parse tiles from numbered layers
        #[cfg(not(target_arch = "wasm32"))]
        let tiles_start = Instant::now();

        Self::parse_tiles(&root_node, &mut map_data).await?;

        #[cfg(not(target_arch = "wasm32"))]
        let tiles_time = tiles_start.elapsed();

        // Parse objects from numbered layers
        #[cfg(not(target_arch = "wasm32"))]
        let objects_start = Instant::now();

        Self::parse_objects(&root_node, &mut map_data).await?;

        #[cfg(not(target_arch = "wasm32"))]
        let objects_time = objects_start.elapsed();

        #[cfg(not(target_arch = "wasm32"))]
        let total_time = total_start.elapsed();

        // info!("=== MAP LOADED SUCCESSFULLY ===");
        // info!("  Map ID: {}", map_id);
        // info!("  Map Name: {}", if !map_data.info.map_name.is_empty() { &map_data.info.map_name } else { "Unknown" });
        // info!("  Backgrounds: {}", map_data.backgrounds.len());
        // info!("  Tiles: {}", map_data.tiles.len());
        // info!("  Objects: {}", map_data.objects.len());
        // info!("  Footholds: {}", map_data.footholds.len());
        // info!("  Portals: {}", map_data.portals.len());
        // info!("  Life (NPCs/Mobs): {}", map_data.life.len());
        // info!("  Ladders/Ropes: {}", map_data.ladders.len());
        // info!("  Map Bounds: ({}, {}) to ({}, {})",
        //       map_data.info.vr_left, map_data.info.vr_top,
        //       map_data.info.vr_right, map_data.info.vr_bottom);
        // #[cfg(not(target_arch = "wasm32"))]
        // {
        //     info!("--- Loading Time Breakdown ---");
        //     info!("  Fetch map file: {:?}", fetch_time);
        //     info!("  Backgrounds:    {:?}", bg_time);
        //     info!("  Life (NPCs):    {:?}", life_time);
        //     info!("  Tiles:          {:?}", tiles_time);
        //     info!("  Objects:        {:?}", objects_time);
        //     info!("  TOTAL:          {:?}", total_time);
        // }
        // info!("===============================");

        // Sort tiles and objects once during load (not every frame during rendering!)
        // This dramatically improves FPS by avoiding clone + sort every frame
        // info!("Sorting tiles and objects by depth...");
        map_data.tiles.sort_by_key(|tile| tile.z_m);
        map_data.objects.sort_by_key(|obj| obj.z);
        // info!("Sorted {} tiles and {} objects", map_data.tiles.len(), map_data.objects.len());

        Ok(map_data)
    }

    /// Debug log the raw map structure
    fn debug_log_map_structure(root_node: &WzNodeArc) {
        info!("=== RAW MAP STRUCTURE ===");
        let root_read = root_node.read().unwrap();

        for (name, child) in root_read.children.iter() {
            let child_read = child.read().unwrap();
            info!("  {}: {:?}", name, child_read.object_type);

            // Log first level children (layer numbers, back, info, etc.)
            for (child_name, grandchild) in child_read.children.iter() {
                let grandchild_read = grandchild.read().unwrap();
                info!("    {}/{}: {:?}", name, child_name, grandchild_read.object_type);

                // For numbered layers, show tile and info structure
                if name.chars().all(|c| c.is_numeric()) {
                    for (gc_name, _) in grandchild_read.children.iter().take(5) {
                        info!("      {}/{}/{}", name, child_name, gc_name);
                    }
                }
            }
        }
        info!("=== END RAW MAP STRUCTURE ===");
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
        
        info!("Map viewport bounds loaded: VR_TOP={}, VR_BOTTOM={}, VR_LEFT={}, VR_RIGHT={}", 
              map_data.info.vr_top, map_data.info.vr_bottom, 
              map_data.info.vr_left, map_data.info.vr_right);

        map_data.info.bgm = Self::get_string_property(&node_read, "bgm").unwrap_or_default();
        map_data.info.map_mark = Self::get_string_property(&node_read, "mapMark").unwrap_or_default();
        map_data.info.on_first_user_enter = Self::get_string_property(&node_read, "onFirstUserEnter").unwrap_or_default();
        map_data.info.on_user_enter = Self::get_string_property(&node_read, "onUserEnter").unwrap_or_default();

        info!("Map bounds: ({}, {}) to ({}, {})",
              map_data.info.vr_left, map_data.info.vr_top,
              map_data.info.vr_right, map_data.info.vr_bottom);

        Ok(())
    }

    /// Parse background layers (parallax scrolling images, NOT tiles)
    async fn parse_backgrounds(node: &WzNodeArc, map_data: &mut MapData) -> Result<(), String> {
        let node_read = node.read().unwrap();
        let children = node_read.children.clone();

        for (_name, child) in children.iter() {
            let child_read = child.read().unwrap();

            // Parse layer number from child name
            if let Ok(layer_num) = child_read.name.parse::<i32>() {
                drop(child_read);

                let bS = Self::get_string_property_from_node(child, "bS").unwrap_or_default();
                let no = Self::get_int_property_from_node(child, "no").unwrap_or(0);
                let x = Self::get_int_property_from_node(child, "x").unwrap_or(0);
                let y = Self::get_int_property_from_node(child, "y").unwrap_or(0);
                let rx = Self::get_int_property_from_node(child, "rx").unwrap_or(0);
                let ry = Self::get_int_property_from_node(child, "ry").unwrap_or(0);
                let cx = Self::get_int_property_from_node(child, "cx").unwrap_or(0);
                let cy = Self::get_int_property_from_node(child, "cy").unwrap_or(0);
                let front = Self::get_int_property_from_node(child, "front").unwrap_or(0) == 1;

                // Debug: Print all background layer properties
                info!("Background layer {}: bS='{}', no={}, pos=({},{}), scroll=({},{}), repeat=({},{}), front={}",
                    layer_num, bS, no, x, y, rx, ry, cx, cy, front);

                // NOTE: Background textures should be loaded from Map.wz/Back/[bS]
                // For now we're not loading them, just storing the metadata
                // TODO: Implement background texture loading from Map.wz/Back/

                let bg = BackgroundLayer {
                    layer_num,
                    bS,
                    ani: Self::get_int_property_from_node(child, "ani").unwrap_or(0) == 1,
                    no,
                    x,
                    y,
                    rx,
                    ry,
                    cx,
                    cy,
                    a: Self::get_int_property_from_node(child, "a").unwrap_or(255),
                    front,
                    flip_x: Self::get_int_property_from_node(child, "f").unwrap_or(0) == 1,
                    flip_y: false,
                    texture: None, // TODO: Load from Map.wz/Back/
                };

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

        info!("Parsing footholds...");
        let mut total_footholds = 0;

        // Footholds are organized as: foothold -> layer -> group -> individual footholds
        for (_layer_name, layer_child) in node_read.children.iter() {
            // Parse layer node to ensure children are loaded
            layer_child.write().unwrap().parse(layer_child)
                .map_err(|e| format!("Failed to parse foothold layer: {:?}", e))?;
            
            let layer_read = layer_child.read().unwrap();
            let layer_num = layer_read.name.parse::<i32>().unwrap_or(0);
            let mut layer_count = 0;

            for (_group_name, group_child) in layer_read.children.iter() {
                // Parse group node to ensure children are loaded
                group_child.write().unwrap().parse(group_child)
                    .map_err(|e| format!("Failed to parse foothold group: {:?}", e))?;
                
                let group_read = group_child.read().unwrap();
                let group_num = group_read.name.parse::<i32>().unwrap_or(0);

                for (_fh_name, fh_child) in group_read.children.iter() {
                    // Parse individual foothold node
                    fh_child.write().unwrap().parse(fh_child)
                        .map_err(|e| format!("Failed to parse foothold: {:?}", e))?;
                    
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

                    // Only add footholds with valid coordinates
                    if foothold.x1 != 0 || foothold.x2 != 0 || foothold.y1 != 0 || foothold.y2 != 0 {
                        map_data.footholds.push(foothold);
                        layer_count += 1;
                        total_footholds += 1;
                    }
                }
            }
            
            if layer_count > 0 {
                info!("  Layer {}: {} footholds", layer_num, layer_count);
            }
        }

        info!("Total footholds parsed: {}", total_footholds);
        Ok(())
    }

    /// Parse portals
    async fn parse_portals(node: &WzNodeArc, map_data: &mut MapData) -> Result<(), String> {
        use crate::map::portal_loader::PortalCache;

        let node_read = node.read().unwrap();
        let mut portal_cache = PortalCache::new();

        info!("Parsing portals from map data...");

        for (_name, child) in node_read.children.iter() {
            let child_read = child.read().unwrap();
            let portal_id = child_read.name.parse::<i32>().unwrap_or(0);
            drop(child_read);

            let pt = Self::get_int_property_from_node(child, "pt").unwrap_or(0);
            let pn = Self::get_string_property_from_node(child, "pn").unwrap_or_default();
            let x = Self::get_int_property_from_node(child, "x").unwrap_or(0);
            let y = Self::get_int_property_from_node(child, "y").unwrap_or(0);

            // Load portal textures based on portal type (only for types with graphics)
            let (textures, origins) = if let Some(portal_type_str) = crate::map::portal_loader::get_portal_type_string(pt) {
                info!("  Portal {}: type={} ({}), name='{}', pos=({},{})", portal_id, pt, portal_type_str, pn, x, y);

                match portal_cache.get_or_load_portal(portal_type_str).await {
                    Some((frames, frame_origins)) => {
                        info!("    Loaded {} frames for portal type '{}'", frames.len(), portal_type_str);
                        (frames, frame_origins)
                    }
                    None => {
                        warn!("    Failed to load portal textures for type '{}', using empty", portal_type_str);
                        (Vec::new(), Vec::new())
                    }
                }
            } else {
                // No graphics for this portal type (sp, pi, pc, ps, etc.)
                info!("  Portal {}: type={} (no graphics), name='{}', pos=({},{})", portal_id, pt, pn, x, y);
                (Vec::new(), Vec::new())
            };

            let portal = Portal {
                id: portal_id,
                pn: pn.clone(),
                pt,
                x,
                y,
                tm: Self::get_int_property_from_node(child, "tm").unwrap_or(999999999),
                tn: Self::get_string_property_from_node(child, "tn").unwrap_or_default(),
                script: Self::get_string_property_from_node(child, "script").unwrap_or_default(),
                horizontal_impact: Self::get_int_property_from_node(child, "horizontalImpact").unwrap_or(0),
                vertical_impact: Self::get_int_property_from_node(child, "verticalImpact").unwrap_or(0),
                textures: textures.clone(),
                origins: origins.clone(),
            };

            // Log portal state for debugging
            info!("    Portal added: id={}, type={}, textures={}, name='{}'",
                  portal_id, pt, portal.textures.len(), pn);

            map_data.portals.push(portal);
        }

        // info!("Portal parsing complete. Total portals: {}", map_data.portals.len());
        let portals_with_textures = map_data.portals.iter().filter(|p| !p.textures.is_empty()).count();
        info!("  Portals with textures: {}", portals_with_textures);
        info!("  Portals without textures: {}", map_data.portals.len() - portals_with_textures);

        Ok(())
    }

    /// Parse life (NPCs and mobs)
    async fn parse_life(node: &WzNodeArc, map_data: &mut MapData) -> Result<(), String> {
        let node_read = node.read().unwrap();
        let mut npc_cache = NpcCache::new();
        let mut mob_cache = MobCache::new();

        // FIRST PASS: Collect all NPC and Mob IDs and life data
        use std::collections::HashSet;
        let mut unique_npc_ids: HashSet<String> = HashSet::new();
        let mut unique_mob_ids: HashSet<String> = HashSet::new();
        
        // Store life entries temporarily (we'll create Life structs after loading textures)
        struct LifeData {
            id: String,
            life_type: String,
            x: i32,
            y: i32,
            foothold: i32,
            cx: i32,
            cy: i32,
            rx0: i32,
            rx1: i32,
            mob_time: i32,
            flip: bool,
            hide: bool,
        }
        let mut life_data: Vec<LifeData> = Vec::new();

        for (_name, child) in node_read.children.iter() {
            let child_read = child.read().unwrap();
            drop(child_read);

            let id = Self::get_string_property_from_node(child, "id").unwrap_or_default();
            let life_type = Self::get_string_property_from_node(child, "type").unwrap_or_default();
            let x = Self::get_int_property_from_node(child, "x").unwrap_or(0);
            let y = Self::get_int_property_from_node(child, "y").unwrap_or(0);

            // Store life data
            let f_val = Self::get_int_property_from_node(child, "f").unwrap_or(0);
            let hide_val = Self::get_int_property_from_node(child, "hide").unwrap_or(0);
            life_data.push(LifeData {
                id: id.clone(),
                life_type: life_type.clone(),
                x,
                y,
                foothold: Self::get_int_property_from_node(child, "fh").unwrap_or(0),
                cx: Self::get_int_property_from_node(child, "cx").unwrap_or(0),
                cy: Self::get_int_property_from_node(child, "cy").unwrap_or(0),
                rx0: Self::get_int_property_from_node(child, "rx0").unwrap_or(0),
                rx1: Self::get_int_property_from_node(child, "rx1").unwrap_or(0),
                mob_time: Self::get_int_property_from_node(child, "mobTime").unwrap_or(0),
                flip: f_val == 1,
                hide: hide_val == 1,
            });

            // Collect unique NPC and Mob IDs
            if life_type == "n" && !id.is_empty() {
                unique_npc_ids.insert(id.clone());
            } else if life_type == "m" && !id.is_empty() {
                unique_mob_ids.insert(id.clone());
            }
        }

        // SECOND PASS: Batch fetch all unique NPC and Mob WZ files
        if !unique_npc_ids.is_empty() {
            info!("Batch fetching {} unique NPC files...", unique_npc_ids.len());
            let npc_ids_vec: Vec<String> = unique_npc_ids.iter().cloned().collect();
            let mut fetch_requests = Vec::new();
            for npc_id in &npc_ids_vec {
                let url = format!(
                    "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/Npc/{}.img",
                    npc_id
                );
                let cache_name = format!("/01/Npc/{}.img", npc_id);
                fetch_requests.push((url, cache_name));
            }

            let fetch_results = AssetManager::fetch_and_cache_batch(fetch_requests).await;

            // Parse all the fetched files
            info!("Parsing {} NPC files...", npc_ids_vec.len());
            for (i, npc_id) in npc_ids_vec.iter().enumerate() {
                if let Ok(bytes) = &fetch_results[i] {
                    if let Err(e) = npc_cache.preload_npc_from_bytes(npc_id, bytes.clone()).await {
                        warn!("Failed to parse NPC {}: {}", npc_id, e);
                    }
                }
            }
        }

        if !unique_mob_ids.is_empty() {
            info!("Batch fetching {} unique Mob files...", unique_mob_ids.len());
            let mob_ids_vec: Vec<String> = unique_mob_ids.iter().cloned().collect();
            let mut fetch_requests = Vec::new();
            for mob_id in &mob_ids_vec {
                let url = format!(
                    "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/Mob/{}.img",
                    mob_id
                );
                let cache_name = format!("/01/Mob/{}.img", mob_id);
                fetch_requests.push((url, cache_name));
            }

            let fetch_results = AssetManager::fetch_and_cache_batch(fetch_requests).await;

            // Parse all the fetched files
            info!("Parsing {} Mob files...", mob_ids_vec.len());
            for (i, mob_id) in mob_ids_vec.iter().enumerate() {
                if let Ok(bytes) = &fetch_results[i] {
                    if let Err(e) = mob_cache.preload_mob_from_bytes(mob_id, bytes.clone()).await {
                        warn!("Failed to parse Mob {}: {}", mob_id, e);
                    }
                }
            }
        }

        // THIRD PASS: Load NPC/Mob names and textures, create life entries
        // info!("Loading {} life entries...", life_data.len());
        for life_entry in life_data {
            // info!("  Life: id='{}', type='{}', pos=({},{})", life_entry.id, life_entry.life_type, life_entry.x, life_entry.y);

            // Load name and texture based on life type
            let (name, texture, origin_x, origin_y, mob_textures, mob_origins) = if life_entry.life_type == "n" && !life_entry.id.is_empty() {
                // Load NPC
                let npc_name = NpcCache::get_npc_name(&life_entry.id).await.unwrap_or_default();

                match npc_cache.get_or_load_npc(&life_entry.id).await {
                    Some((tex, ox, oy)) => {
                        info!("    Loaded NPC: {} ({})", npc_name, life_entry.id);
                        (npc_name, Some(tex), ox, oy, Vec::new(), Vec::new())
                    }
                    None => {
                        warn!("    Failed to load NPC texture: {} ({})", npc_name, life_entry.id);
                        (npc_name, None, 0, 0, Vec::new(), Vec::new())
                    }
                }
            } else if life_entry.life_type == "m" && !life_entry.id.is_empty() {
                // Load Mob - try to load move frames first, fallback to single texture
                let mob_name = MobCache::get_mob_name(&life_entry.id).await.unwrap_or_default();

                // Try to load move animation frames
                if let Some((textures, origins)) = mob_cache.get_or_load_mob_move_frames(&life_entry.id).await {
                    if !textures.is_empty() {
                        info!("    Loaded Mob with {} move frames: {} ({})", textures.len(), mob_name, life_entry.id);
                        // Use first frame as fallback texture
                        let first_tex = textures[0].clone();
                        let (first_ox, first_oy) = origins[0];
                        (mob_name, Some(first_tex), first_ox, first_oy, textures, origins)
                    } else {
                        // Fallback to single texture
                        match mob_cache.get_or_load_mob(&life_entry.id).await {
                            Some((tex, ox, oy)) => {
                                info!("    Loaded Mob: {} ({})", mob_name, life_entry.id);
                                (mob_name, Some(tex), ox, oy, Vec::new(), Vec::new())
                            }
                            None => {
                                warn!("    Failed to load Mob texture: {} ({})", mob_name, life_entry.id);
                                (mob_name, None, 0, 0, Vec::new(), Vec::new())
                            }
                        }
                    }
                } else {
                    // Fallback to single texture if move frames not available
                    match mob_cache.get_or_load_mob(&life_entry.id).await {
                        Some((tex, ox, oy)) => {
                            info!("    Loaded Mob: {} ({})", mob_name, life_entry.id);
                            (mob_name, Some(tex), ox, oy, Vec::new(), Vec::new())
                        }
                        None => {
                            warn!("    Failed to load Mob texture: {} ({})", mob_name, life_entry.id);
                            (mob_name, None, 0, 0, Vec::new(), Vec::new())
                        }
                    }
                }
            } else {
                (String::new(), None, 0, 0, Vec::new(), Vec::new())
            };

            // Adjust y position to place life entity on the nearest foothold below
            let adjusted_y = if let Some((foothold_y, _fh)) = map_data.find_foothold_below(life_entry.x as f32, life_entry.y as f32) {
                // Place the life entity on the foothold
                // The origin_y represents the bottom of the sprite, so we subtract it to get proper positioning
                (foothold_y - origin_y as f32) as i32
            } else {
                // No foothold found, use original y position
                life_entry.y
            };

            let life = Life {
                id: life_entry.id,
                name,
                life_type: life_entry.life_type,
                x: life_entry.x,
                y: adjusted_y,
                foothold: life_entry.foothold,
                cx: life_entry.cx,
                cy: life_entry.cy,
                rx0: life_entry.rx0,
                rx1: life_entry.rx1,
                mob_time: life_entry.mob_time,
                flip: life_entry.flip,
                hide: life_entry.hide,
                origin_x,
                origin_y,
                texture,
                textures: mob_textures,
                origins: mob_origins,
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

    /// Parse tiles from numbered layers
    async fn parse_tiles(root_node: &WzNodeArc, map_data: &mut MapData) -> Result<(), String> {
        let root_read = root_node.read().unwrap();
        let mut tile_cache = TileCache::new();

        // FIRST PASS: Collect all unique tileset files and tile requests
        use std::collections::{HashSet, HashMap};
        let mut unique_tilesets: HashSet<String> = HashSet::new();
        let mut tile_requests: Vec<(String, String, i32)> = Vec::new(); // (tileset, u, no)
        let mut tile_data: Vec<(i32, i32, String, String, i32, i32, i32, i32)> = Vec::new();

        // Iterate through all children looking for numbered layers
        for (layer_name, layer_child) in root_read.children.iter() {
            // Check if this is a numbered layer (e.g., "3", "4", "6")
            if layer_name.chars().all(|c| c.is_numeric()) {
                let layer_num = layer_name.parse::<i32>().unwrap_or(0);
                info!("Processing tile layer: {}", layer_num);

                let layer_read = layer_child.read().unwrap();

                // Get tileset name from info/tS
                let tileset = if let Some(info_node) = layer_read.children.get("info") {
                    Self::get_string_property_from_node(info_node, "tS").unwrap_or_default()
                } else {
                    String::new()
                };

                // Parse tiles from the tile subdirectory
                if let Some(tile_node) = layer_read.children.get("tile") {
                    let tile_read = tile_node.read().unwrap();

                    for (tile_id_str, tile_child) in tile_read.children.iter() {
                        let tile_id = tile_id_str.parse::<i32>().unwrap_or(0);
                        let tile_child_read = tile_child.read().unwrap();
                        drop(tile_child_read);

                        let no = Self::get_int_property_from_node(tile_child, "no").unwrap_or(0);
                        let u = Self::get_string_property_from_node(tile_child, "u").unwrap_or_default();
                        let x = Self::get_int_property_from_node(tile_child, "x").unwrap_or(0);
                        let y = Self::get_int_property_from_node(tile_child, "y").unwrap_or(0);
                        let z_m = Self::get_int_property_from_node(tile_child, "zM").unwrap_or(0);

                        // Store tile data for later
                        tile_data.push((tile_id, layer_num, tileset.clone(), u.clone(), no, x, y, z_m));

                        // Collect unique tileset files and tile requests
                        if !tileset.is_empty() && !u.is_empty() {
                            unique_tilesets.insert(tileset.clone());
                            tile_requests.push((tileset.clone(), u.clone(), no));
                        }
                    }
                }
            }
        }

        // SECOND PASS: Batch preload all unique tileset WZ files
        // First, batch fetch all the WZ files
        info!("Batch fetching {} unique tileset files...", unique_tilesets.len());
        let tilesets_vec: Vec<String> = unique_tilesets.iter().cloned().collect();
        let mut fetch_requests = Vec::new();
        for tileset in &tilesets_vec {
            let url = format!(
                "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/Map/Tile/{}.img",
                tileset
            );
            let cache_name = format!("/01/Map/Tile/{}.img", tileset);
            fetch_requests.push((url, cache_name));
        }
        
        // Fetch all in parallel
        let fetch_results = AssetManager::fetch_and_cache_batch(fetch_requests).await;

        // Now parse all the fetched files (sequentially but from cache)
        info!("Parsing {} tileset files...", tilesets_vec.len());
        for (i, tileset) in tilesets_vec.iter().enumerate() {
            if let Ok(bytes) = &fetch_results[i] {
                // Parse and cache the WZ node
                if let Err(e) = tile_cache.preload_tileset_from_bytes(tileset, bytes.clone()).await {
                    warn!("Failed to parse tileset {}: {}", tileset, e);
                }
            }
        }

        info!("Preloaded tileset files");

        // THIRD PASS: Now load textures (WZ files should be cached) and create tiles
        info!("Loading {} individual tile textures...", tile_requests.len());
        let mut failed_tiles = 0;
        let mut successful_tiles = 0;

        for (tile_id, layer_num, tileset, u, no, x, y, z_m) in tile_data {
            // Load tile texture if tileset is specified
            let (texture, origin_x, origin_y) = if !tileset.is_empty() && !u.is_empty() {
                match tile_cache.get_or_load_tile(&tileset, &u, no).await {
                    Some((tex, ox, oy)) => {
                        successful_tiles += 1;
                        (Some(tex), ox, oy)
                    }
                    None => {
                        failed_tiles += 1;
                        warn!("    Failed to load tile: {}/{}/{} (tile ID: {})", tileset, u, no, tile_id);
                        (None, 0, 0)
                    }
                }
            } else {
                (None, 0, 0)
            };

            let tile = Tile {
                id: tile_id,
                layer: layer_num,
                tileset: tileset.clone(),
                u,
                no,
                x,
                y,
                z_m,
                origin_x,
                origin_y,
                texture,
            };

            map_data.tiles.push(tile);
        }

        if failed_tiles > 0 {
            warn!("⚠ Tile loading summary: {} succeeded, {} FAILED ({}% success rate)",
                  successful_tiles, failed_tiles,
                  (successful_tiles * 100) / (successful_tiles + failed_tiles));
        } else {
            info!("✓ All {} tiles loaded successfully", successful_tiles);
        }

        Ok(())
    }

    /// Parse objects from numbered layers
    async fn parse_objects(root_node: &WzNodeArc, map_data: &mut MapData) -> Result<(), String> {
        let root_read = root_node.read().unwrap();
        let mut object_cache = ObjectCache::new();

        // FIRST PASS: Collect all unique object sets and object requests
        use std::collections::HashSet;
        let mut unique_object_sets: HashSet<String> = HashSet::new();
        let mut object_data: Vec<(i32, i32, String, String, String, String, i32, i32, i32, i32, bool, i32)> = Vec::new();

        // Iterate through all children looking for numbered layers
        for (layer_name, layer_child) in root_read.children.iter() {
            // Check if this is a numbered layer (e.g., "3", "4", "6")
            if layer_name.chars().all(|c| c.is_numeric()) {
                let layer_num = layer_name.parse::<i32>().unwrap_or(0);

                let layer_read = layer_child.read().unwrap();

                // Parse objects from the obj subdirectory
                if let Some(obj_node) = layer_read.children.get("obj") {
                    let obj_read = obj_node.read().unwrap();

                    for (obj_id_str, obj_child) in obj_read.children.iter() {
                        let obj_id = obj_id_str.parse::<i32>().unwrap_or(0);
                        let obj_child_read = obj_child.read().unwrap();
                        drop(obj_child_read);

                        let oS = Self::get_string_property_from_node(obj_child, "oS").unwrap_or_default();
                        let l0 = Self::get_string_property_from_node(obj_child, "l0").unwrap_or_default();
                        let l1 = Self::get_string_property_from_node(obj_child, "l1").unwrap_or_default();
                        let l2 = Self::get_string_property_from_node(obj_child, "l2").unwrap_or_default();
                        let x = Self::get_int_property_from_node(obj_child, "x").unwrap_or(0);
                        let y = Self::get_int_property_from_node(obj_child, "y").unwrap_or(0);
                        let z = Self::get_int_property_from_node(obj_child, "z").unwrap_or(0);
                        let z_m = Self::get_int_property_from_node(obj_child, "zM").unwrap_or(0);
                        let f = Self::get_int_property_from_node(obj_child, "f").unwrap_or(0) == 1;
                        let r = Self::get_int_property_from_node(obj_child, "r").unwrap_or(0);

                        // Store object data for later
                        object_data.push((obj_id, layer_num, oS.clone(), l0.clone(), l1.clone(), l2.clone(), x, y, z, z_m, f, r));

                        // Collect unique object sets
                        if !oS.is_empty() {
                            unique_object_sets.insert(oS);
                        }
                    }
                }
            }
        }

        // SECOND PASS: Batch fetch all unique object set WZ files
        if !unique_object_sets.is_empty() {
            info!("Batch fetching {} unique object set files...", unique_object_sets.len());
            let object_sets_vec: Vec<String> = unique_object_sets.iter().cloned().collect();
            let mut fetch_requests = Vec::new();
            for oS in &object_sets_vec {
                let url = format!(
                    "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/Map/Obj/{}.img",
                    oS
                );
                let cache_name = format!("/01/Map/Obj/{}.img", oS);
                fetch_requests.push((url, cache_name));
            }
            
            let fetch_results = AssetManager::fetch_and_cache_batch(fetch_requests).await;
            
            // Parse all the fetched files
            info!("Parsing {} object set files...", object_sets_vec.len());
            for (i, oS) in object_sets_vec.iter().enumerate() {
                if let Ok(bytes) = &fetch_results[i] {
                    if let Err(e) = object_cache.preload_object_set_from_bytes(oS, bytes.clone()).await {
                        warn!("Failed to parse object set {}: {}", oS, e);
                    }
                }
            }
        }

        // THIRD PASS: Now load textures (WZ files should be cached) and create objects
        info!("Loading {} individual object textures...", object_data.len());
        let mut failed_objects = 0;
        let mut successful_objects = 0;

        for (obj_id, layer_num, oS, l0, l1, l2, x, y, z, z_m, f, r) in object_data {
            // Load object texture if object set is specified
            let (texture, origin_x, origin_y) = if !oS.is_empty() && !l0.is_empty() {
                match object_cache.get_or_load_object(&oS, &l0, &l1, &l2).await {
                    Some((tex, ox, oy)) => {
                        successful_objects += 1;
                        (Some(tex), ox, oy)
                    }
                    None => {
                        failed_objects += 1;
                        warn!("    Failed to load object: {}/{}/{}/{} (object ID: {})", oS, l0, l1, l2, obj_id);
                        (None, 0, 0)
                    }
                }
            } else {
                (None, 0, 0)
            };

            let object = MapObject {
                id: obj_id,
                layer: layer_num,
                oS,
                l0,
                l1,
                l2,
                x,
                y,
                z,
                z_m,
                f,
                r,
                origin_x,
                origin_y,
                texture,
            };

            map_data.objects.push(object);
        }

        if failed_objects > 0 {
            warn!("⚠ Object loading summary: {} succeeded, {} FAILED ({}% success rate)",
                  successful_objects, failed_objects,
                  (successful_objects * 100) / (successful_objects + failed_objects));
        } else if successful_objects > 0 {
            info!("✓ All {} objects loaded successfully", successful_objects);
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

    /// Get map name from String/Map.img by recursively searching for the map ID
    pub async fn get_map_name(map_id: &str) -> Result<String, String> {
        const MAP_STRING_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/00/String/Map.img";
        const MAP_STRING_CACHE: &str = "/01/String/Map.img";

        info!("Fetching String/Map.img to lookup map name for ID: {}", map_id);

        // Fetch and cache the Map.img file
        let bytes = AssetManager::fetch_and_cache(MAP_STRING_URL, MAP_STRING_CACHE).await
            .map_err(|e| format!("Failed to fetch String/Map.img: {}", e))?;

        info!("Parsing String/Map.img (size: {} bytes)...", bytes.len());

        // Guess IV and create reader
        let wz_iv = guess_iv_from_wz_img(&bytes)
            .ok_or_else(|| "Unable to guess version from String/Map.img".to_string())?;

        let byte_len = bytes.len();
        let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
        
        // Create root node
        let name: wz_reader::WzNodeName = "Map.img".into();
        let wz_image = WzImage::new(&name, 0, byte_len, &reader);
        let root_node: WzNodeArc = WzNode::new(&name, wz_image, None).into();

        // Parse the root node
        root_node.write().unwrap().parse(&root_node)
            .map_err(|e| format!("Failed to parse String/Map.img: {:?}", e))?;

        info!("String/Map.img parsed, searching for map ID: {}", map_id);

        // Strip leading zeros from map ID for lookup (e.g., "001000000" -> "1000000")
        let map_id_normalized = map_id.trim_start_matches('0');
        // If all zeros were stripped, keep at least one zero
        let map_id_normalized = if map_id_normalized.is_empty() { "0" } else { map_id_normalized };
        info!("Normalized map ID (stripped leading zeros): {} -> {}", map_id, map_id_normalized);

        // The structure is: Map (root) -> [region] (e.g., "victoria") -> [mapId] (e.g., "101020001") -> mapName
        // Navigate to the "Map" node (root node after parsing)
        let root_read = root_node.read().unwrap();
        
        // Iterate through all regions (children of Map)
        for (_region_name, region_node) in root_read.children.iter() {
            let region_read = region_node.read().unwrap();
            
            // Iterate through all map IDs in this region
            for (map_id_name, map_id_node) in region_read.children.iter() {
                let map_id_name_str = map_id_name.as_str();
                
                // Check if this map ID matches what we're looking for (using normalized ID)
                if map_id_name_str == map_id_normalized {
                    info!("Found map ID {} in region {}", map_id, region_read.name.as_str());
                    
                    // Get the mapName property from this node
                    let map_id_read = map_id_node.read().unwrap();
                    if let Ok(name_prop) = map_id_read.at_path_parsed("mapName") {
                        let name_prop_read = name_prop.read().unwrap();
                        if let WzObjectType::Value(wz_reader::property::WzValue::String(val)) = &name_prop_read.object_type {
                            if let Ok(map_name) = val.get_string() {
                                info!("Found map name: {}", map_name);
                                return Ok(map_name);
                            }
                        }
                    }
                    
                    warn!("Found map node for ID {} but could not extract mapName property", map_id);
                }
            }
        }

        warn!("Map ID {} not found in String/Map.img", map_id);
        Ok(String::new()) // Return empty string if not found
    }
}
