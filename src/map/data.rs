use macroquad::prelude::*;

/// Complete map data structure
#[derive(Debug, Clone)]
pub struct MapData {
    pub info: MapInfo,
    pub backgrounds: Vec<BackgroundLayer>,
    pub tiles: Vec<Tile>,
    pub objects: Vec<MapObject>,
    pub footholds: Vec<Foothold>,
    pub portals: Vec<Portal>,
    pub life: Vec<Life>,
    pub ladders: Vec<Ladder>,
    pub minimap: Option<MiniMapInfo>,
}

/// Map information and metadata
#[derive(Debug, Clone, Default)]
pub struct MapInfo {
    pub version: i32,
    pub cloud: bool,
    pub town: bool,
    pub swim: bool,
    pub return_map: i32,
    pub forced_return: i32,
    pub mob_rate: f32,
    pub bgm: String,
    pub map_mark: String,
    pub fly: bool,
    pub no_map_cmd: bool,
    pub hide_minimap: bool,
    pub field_limit: i32,
    pub vr_top: i32,
    pub vr_left: i32,
    pub vr_bottom: i32,
    pub vr_right: i32,
    pub on_first_user_enter: String,
    pub on_user_enter: String,
    pub map_name: String, // Map name from String/Map.img
}

/// Background layer with positioning and scrolling
#[derive(Debug, Clone)]
pub struct BackgroundLayer {
    pub layer_num: i32,
    pub bS: String,  // Background set name
    pub ani: bool,   // Is animated
    pub no: i32,     // Image number in set
    pub x: i32,
    pub y: i32,
    pub rx: i32,     // Scroll ratio X (for parallax)
    pub ry: i32,     // Scroll ratio Y
    pub cx: i32,     // Repeat horizontally
    pub cy: i32,     // Repeat vertically
    pub a: i32,      // Alpha (opacity)
    pub front: bool, // Draw in front of everything
    pub flip_x: bool,
    pub flip_y: bool,
    pub texture: Option<Texture2D>,
}

/// Tile (ground texture) data
#[derive(Debug, Clone)]
pub struct Tile {
    pub id: i32,           // Tile ID
    pub layer: i32,        // Layer number (3, 4, 6, etc.)
    pub tileset: String,   // Tileset name from info/tS
    pub u: String,         // Category (enH0, bsc, etc.)
    pub no: i32,           // Tile number in tileset
    pub x: i32,
    pub y: i32,
    pub z_m: i32,          // Z-depth
    pub origin_x: i32,     // Origin offset X
    pub origin_y: i32,     // Origin offset Y
    pub texture: Option<Texture2D>,
}

/// Map object (decorative elements)
#[derive(Debug, Clone)]
pub struct MapObject {
    pub id: i32,           // Object ID
    pub layer: i32,        // Layer number
    pub oS: String,        // Object set name
    pub l0: String,        // Layer 0
    pub l1: String,        // Layer 1
    pub l2: String,        // Layer 2
    pub x: i32,
    pub y: i32,
    pub z: i32,            // Z-depth
    pub z_m: i32,          // Z-depth multiplier
    pub f: bool,           // Flip
    pub r: i32,            // Rotation
    pub origin_x: i32,     // Origin offset X
    pub origin_y: i32,     // Origin offset Y
    pub texture: Option<Texture2D>,
}

/// Foothold platform data
#[derive(Debug, Clone)]
pub struct Foothold {
    pub id: i32,
    pub layer: i32,
    pub group: i32,
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
    pub prev: i32,  // Previous foothold ID
    pub next: i32,  // Next foothold ID
    pub piece: i32,
}

/// Portal (warp) data
#[derive(Debug, Clone)]
pub struct Portal {
    pub id: i32,
    pub pn: String,        // Portal name
    pub pt: i32,           // Portal type (0=sp, 2=pv, etc.)
    pub x: i32,
    pub y: i32,
    pub tm: i32,           // Target map
    pub tn: String,        // Target portal name
    pub script: String,
    pub horizontal_impact: i32,
    pub vertical_impact: i32,
    pub textures: Vec<Texture2D>, // Animation frames
    pub origins: Vec<(i32, i32)>,  // Origin offset for each frame (x, y)
}

/// Life (NPC/Mob) spawn data
#[derive(Debug, Clone)]
pub struct Life {
    pub id: String,
    pub name: String,      // NPC/Mob name from String data
    pub life_type: String, // "n" for NPC, "m" for mob
    pub x: i32,
    pub y: i32,
    pub foothold: i32,
    pub cx: i32,
    pub cy: i32,
    pub rx0: i32,
    pub rx1: i32,
    pub mob_time: i32,
    pub flip: bool,
    pub hide: bool,
    pub origin_x: i32,     // Origin offset X (for single texture)
    pub origin_y: i32,     // Origin offset Y (for single texture)
    pub texture: Option<Texture2D>, // Single texture (for NPCs or fallback)
    pub textures: Vec<Texture2D>, // Animation frames (for mobs)
    pub origins: Vec<(i32, i32)>,  // Origin offset for each frame (x, y)
}

/// Ladder or rope data
#[derive(Debug, Clone)]
pub struct Ladder {
    pub id: i32,
    pub x: i32,
    pub y1: i32,
    pub y2: i32,
    pub ladder: bool,  // true = ladder, false = rope
    pub page: i32,
}

/// Minimap information
#[derive(Debug, Clone)]
pub struct MiniMapInfo {
    pub width: i32,
    pub height: i32,
    pub mag: i32,
    pub canvas: Option<Texture2D>,
}

impl MapData {
    pub fn new() -> Self {
        Self {
            info: MapInfo::default(),
            backgrounds: Vec::new(),
            tiles: Vec::new(),
            objects: Vec::new(),
            footholds: Vec::new(),
            portals: Vec::new(),
            life: Vec::new(),
            ladders: Vec::new(),
            minimap: None,
        }
    }

    /// Get map bounds
    pub fn get_bounds(&self) -> (i32, i32, i32, i32) {
        (
            self.info.vr_left,
            self.info.vr_top,
            self.info.vr_right,
            self.info.vr_bottom,
        )
    }

    /// Get map width
    pub fn get_width(&self) -> i32 {
        self.info.vr_right - self.info.vr_left
    }

    /// Get map height
    pub fn get_height(&self) -> i32 {
        self.info.vr_bottom - self.info.vr_top
    }

    /// Find foothold at position (for collision)
    /// Returns the foothold that the point is standing on
    /// Point must be within the foothold's horizontal bounds and close to its Y position
    pub fn find_foothold_at(&self, x: f32, y: f32) -> Option<&Foothold> {
        let ix = x as i32;
        let iy = y as i32;

        let mut best_fh: Option<&Foothold> = None;
        let mut best_distance = 50.0; // Tighter tolerance for standing on platforms

        for fh in &self.footholds {
            // Check if point is within horizontal range (strict - no margin)
            let min_x = fh.x1.min(fh.x2);
            let max_x = fh.x1.max(fh.x2);

            // Point must be within foothold horizontal bounds
            if ix >= min_x && ix <= max_x {
                // Calculate Y position on this foothold at the given X
                let dx = fh.x2 - fh.x1;
                let dy = fh.y2 - fh.y1;

                let fh_y = if dx != 0 {
                    fh.y1 + ((ix - fh.x1) * dy) / dx
                } else {
                    fh.y1
                };

                // Check vertical distance - must be close to the foothold
                let vertical_distance = (iy as f32 - fh_y as f32).abs();
                
                // Accept footholds that are close vertically (within 50px)
                // Prefer footholds that are at or slightly below the point
                if vertical_distance < best_distance {
                    best_distance = vertical_distance;
                    best_fh = Some(fh);
                }
            }
        }

        best_fh
    }

    /// Find the nearest foothold below a position (for spawning/falling)
    /// Strict horizontal bounds - point must be within foothold range
    pub fn find_foothold_below(&self, x: f32, y: f32) -> Option<(f32, &Foothold)> {
        let ix = x as i32;
        let iy = y as i32;

        let mut closest_y = None;
        let mut closest_fh = None;

        for fh in &self.footholds {
            // Check if point is within horizontal range (strict - no margin)
            let min_x = fh.x1.min(fh.x2);
            let max_x = fh.x1.max(fh.x2);
            
            // Point must be within foothold horizontal bounds
            if ix >= min_x && ix <= max_x {
                // Calculate Y position on this foothold at the given X
                let dx = fh.x2 - fh.x1;
                let dy = fh.y2 - fh.y1;

                let fh_y = if dx != 0 {
                    fh.y1 + ((ix - fh.x1) * dy) / dx
                } else {
                    fh.y1
                };

                // Only consider footholds below or at the position (with small tolerance)
                if fh_y >= iy - 10 {
                    // Find the closest one below
                    if closest_y.is_none() || fh_y < closest_y.unwrap() {
                        closest_y = Some(fh_y);
                        closest_fh = Some(fh);
                    }
                }
            }
        }

        closest_fh.map(|fh| (closest_y.unwrap() as f32, fh))
    }

    /// Find the nearest foothold strictly below a given Y position (for drop-through)
    /// This excludes the current foothold the player is standing on
    pub fn find_foothold_strictly_below(&self, x: f32, y: f32, min_distance: f32) -> Option<(f32, &Foothold)> {
        let ix = x as i32;
        let iy = y as i32;

        let mut closest_y: Option<i32> = None;
        let mut closest_fh = None;

        for fh in &self.footholds {
            // Check if point is within horizontal range
            let min_x = fh.x1.min(fh.x2);
            let max_x = fh.x1.max(fh.x2);
            
            if ix >= min_x && ix <= max_x {
                // Calculate Y position on this foothold at the given X
                let dx = fh.x2 - fh.x1;
                let dy = fh.y2 - fh.y1;

                let fh_y = if dx != 0 {
                    fh.y1 + ((ix - fh.x1) * dy) / dx
                } else {
                    fh.y1
                };

                // Only consider footholds strictly below (with minimum distance)
                if fh_y as f32 > y + min_distance {
                    // Find the closest one below
                    if closest_y.is_none() || fh_y < closest_y.unwrap() {
                        closest_y = Some(fh_y);
                        closest_fh = Some(fh);
                    }
                }
            }
        }

        closest_fh.map(|fh| (closest_y.unwrap() as f32, fh))
    }

    /// Find foothold by ID
    pub fn find_foothold_by_id(&self, id: i32) -> Option<&Foothold> {
        self.footholds.iter().find(|fh| fh.id == id)
    }

    /// Get Y position on a foothold at given X
    pub fn get_foothold_y_at(&self, fh: &Foothold, x: f32) -> f32 {
        let ix = x as i32;
        let dx = fh.x2 - fh.x1;
        let dy = fh.y2 - fh.y1;

        if dx != 0 {
            (fh.y1 + ((ix - fh.x1) * dy) / dx) as f32
        } else {
            fh.y1 as f32
        }
    }
}
