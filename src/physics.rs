use crate::map::MapData;

// Physics constants from C++ (Physics.cpp)
const GRAVITY: f32 = 0.14;
const FRICTION: f32 = 0.5;
const SLOPE_FACTOR: f32 = 0.1;
const GROUND_SLIP: f32 = 3.0;

/// Map boundaries calculated from footholds (from C++ FootholdTree)
#[derive(Debug, Clone, Default)]
pub struct MapBounds {
    pub wall_left: f32,   // leftw + 25
    pub wall_right: f32,  // rightw - 25
    pub border_top: f32,  // topb - 300
    pub border_bottom: f32, // botb + 100
}

impl MapBounds {
    /// Calculate bounds from map footholds (C++ FootholdTree constructor)
    pub fn from_map(map: &MapData) -> Self {
        let mut leftw: i32 = 30000;
        let mut rightw: i32 = -30000;
        let mut topb: i32 = 30000;
        let mut botb: i32 = -30000;

        for fh in &map.footholds {
            let l = fh.x1.min(fh.x2);
            let r = fh.x1.max(fh.x2);
            let t = fh.y1.min(fh.y2);
            let b = fh.y1.max(fh.y2);

            if l < leftw { leftw = l; }
            if r > rightw { rightw = r; }
            if t < topb { topb = t; }
            if b > botb { botb = b; }
        }

        Self {
            wall_left: (leftw + 25) as f32,
            wall_right: (rightw - 25) as f32,
            border_top: (topb - 300) as f32,
            border_bottom: (botb + 100) as f32,
        }
    }
}

pub struct PhysicsObject {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub on_ground: bool,
    pub fh_id: i32,
    pub fh_layer: i32,
}

impl PhysicsObject {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            on_ground: true,
            fh_id: 0,
            fh_layer: 0,
        }
    }
}

pub struct Physics;

impl Physics {
    pub fn move_object(obj: &mut PhysicsObject, map: &MapData, bounds: &MapBounds, _dt: f32) {
        // Update foothold
        Self::update_foothold(obj, map);

        // Apply gravity and friction (C++ move_normal)
        if !obj.on_ground {
            obj.vy += GRAVITY;
        } else {
            obj.vy = 0.0;
            // Apply horizontal friction when on ground
            if obj.vx.abs() < 0.1 {
                obj.vx = 0.0;
            } else {
                let inertia = obj.vx / GROUND_SLIP;
                obj.vx -= FRICTION * inertia;
            }
        }

        // Update position
        obj.x += obj.vx;
        obj.y += obj.vy;

        // Limit movement (wall and platform collision)
        Self::limit_movement(obj, map, bounds);
    }

    fn update_foothold(obj: &mut PhysicsObject, map: &MapData) {
        if obj.fh_id == 0 || !obj.on_ground {
            obj.fh_id = Self::get_foothold_below(obj.x, obj.y, map);
            return;
        }

        let current_fh = match map.footholds.iter().find(|f| f.id == obj.fh_id) {
            Some(fh) => fh,
            None => {
                obj.fh_id = Self::get_foothold_below(obj.x, obj.y, map);
                return;
            }
        };

        // Follow connected footholds when walking
        if obj.x > current_fh.x2 as f32 {
            if current_fh.next != 0 {
                if let Some(next_fh) = map.footholds.iter().find(|f| f.id == current_fh.next) {
                    let height_diff = (current_fh.y2 - next_fh.y1).abs();
                    if height_diff < 20 {
                        obj.fh_id = current_fh.next;
                    } else {
                        obj.on_ground = false;
                    }
                }
            } else {
                obj.on_ground = false;
            }
        } else if obj.x < current_fh.x1 as f32 {
            if current_fh.prev != 0 {
                if let Some(prev_fh) = map.footholds.iter().find(|f| f.id == current_fh.prev) {
                    let height_diff = (current_fh.y1 - prev_fh.y2).abs();
                    if height_diff < 20 {
                        obj.fh_id = current_fh.prev;
                    } else {
                        obj.on_ground = false;
                    }
                }
            } else {
                obj.on_ground = false;
            }
        }
    }

    fn limit_movement(obj: &mut PhysicsObject, map: &MapData, bounds: &MapBounds) {
        // Horizontal wall collision (C++ FootholdTree::limit_movement)
        if obj.vx != 0.0 {
            let left = obj.vx < 0.0;
            let wall = Self::get_wall(obj.fh_id, left, obj.y, map, bounds);
            
            let collision = if left {
                obj.x <= wall
            } else {
                obj.x >= wall
            };

            if collision {
                obj.x = wall;
                obj.vx = 0.0;
            }
        }

        // Vertical platform collision
        if let Some(fh) = map.footholds.iter().find(|f| f.id == obj.fh_id) {
            let ground_y = Self::get_ground_y(fh, obj.x);
            if obj.vy >= 0.0 && obj.y >= ground_y {
                obj.y = ground_y;
                obj.vy = 0.0;
                obj.on_ground = true;
            } else {
                obj.on_ground = false;
            }
        } else {
            obj.on_ground = false;
        }

        // Map boundary limits (C++ lines 119-124)
        if obj.y < bounds.border_top {
            obj.y = bounds.border_top;
            obj.vy = 0.0;
        } else if obj.y > bounds.border_bottom {
            obj.y = bounds.border_bottom;
            obj.vy = 0.0;
        }

        // Horizontal boundary limits
        if obj.x < bounds.wall_left {
            obj.x = bounds.wall_left;
            obj.vx = 0.0;
        } else if obj.x > bounds.wall_right {
            obj.x = bounds.wall_right;
            obj.vx = 0.0;
        }
    }

    /// Get wall position for collision (C++ FootholdTree::get_wall)
    fn get_wall(fh_id: i32, left: bool, y: f32, map: &MapData, bounds: &MapBounds) -> f32 {
        let current_fh = match map.footholds.iter().find(|f| f.id == fh_id) {
            Some(fh) => fh,
            None => return if left { bounds.wall_left } else { bounds.wall_right },
        };

        // Check vertical range for blocking (y - 50 to y - 1)
        let vert_min = (y - 50.0) as i32;
        let vert_max = (y - 1.0) as i32;

        if left {
            // Check prev foothold for blocking wall
            if let Some(prev) = map.footholds.iter().find(|f| f.id == current_fh.prev) {
                if Self::is_blocking(prev, vert_min, vert_max) {
                    return current_fh.x1 as f32;
                }
                // Check prev's prev
                if let Some(prev_prev) = map.footholds.iter().find(|f| f.id == prev.prev) {
                    if Self::is_blocking(prev_prev, vert_min, vert_max) {
                        return prev.x1 as f32;
                    }
                }
            }
            bounds.wall_left
        } else {
            // Check next foothold for blocking wall
            if let Some(next) = map.footholds.iter().find(|f| f.id == current_fh.next) {
                if Self::is_blocking(next, vert_min, vert_max) {
                    return current_fh.x2 as f32;
                }
                // Check next's next
                if let Some(next_next) = map.footholds.iter().find(|f| f.id == next.next) {
                    if Self::is_blocking(next_next, vert_min, vert_max) {
                        return next.x2 as f32;
                    }
                }
            }
            bounds.wall_right
        }
    }

    /// Check if foothold is a blocking wall (C++ Foothold::is_blocking)
    fn is_blocking(fh: &crate::map::Foothold, vert_min: i32, vert_max: i32) -> bool {
        // A wall has x1 == x2 (vertical line)
        if fh.x1 != fh.x2 {
            return false;
        }
        // Check if wall's vertical range overlaps with player's vertical range
        let fh_min = fh.y1.min(fh.y2);
        let fh_max = fh.y1.max(fh.y2);
        fh_min < vert_max && vert_min < fh_max
    }

    fn get_ground_y(fh: &crate::map::Foothold, x: f32) -> f32 {
        if fh.x1 == fh.x2 {
            fh.y1 as f32
        } else {
            let dx = (fh.x2 - fh.x1) as f32;
            let dy = (fh.y2 - fh.y1) as f32;
            let slope = dy / dx;
            slope * (x - fh.x1 as f32) + fh.y1 as f32
        }
    }

    fn get_foothold_below(x: f32, y: f32, map: &MapData) -> i32 {
        let mut best_fh_id = 0;
        let mut best_y = 30000.0_f32;

        for fh in &map.footholds {
            // Skip walls
            if fh.x1 == fh.x2 {
                continue;
            }

            let min_x = fh.x1.min(fh.x2) as f32;
            let max_x = fh.x1.max(fh.x2) as f32;

            if x >= min_x && x <= max_x {
                let ground_y = Self::get_ground_y(fh, x);
                if ground_y >= y && ground_y < best_y {
                    best_y = ground_y;
                    best_fh_id = fh.id;
                }
            }
        }

        best_fh_id
    }
}
