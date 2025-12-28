use macroquad::prelude::*;
use crate::map::data::*;
use crate::flags;

pub struct MapRenderer {
    debug_footholds: bool,
    debug_portals: bool,
    debug_bounds: bool,
}

impl MapRenderer {
    pub fn new() -> Self {
        Self {
            debug_footholds: flags::SHOW_HITBOXES,
            debug_portals: true,
            debug_bounds: true,
        }
    }

    /// Render the entire map at the given camera position
    pub fn render(&self, map: &MapData, camera_x: f32, camera_y: f32) {
        // Draw backgrounds (layers behind player)
        self.render_backgrounds(map, camera_x, camera_y, false);

        // Draw footholds (platforms) for debugging
        if self.debug_footholds {
            self.render_footholds(map, camera_x, camera_y);
        }

        // Draw portals for debugging
        if self.debug_portals {
            self.render_portals(map, camera_x, camera_y);
        }

        // Draw map bounds for debugging
        if self.debug_bounds {
            self.render_bounds(map, camera_x, camera_y);
        }
    }

    /// Render foreground elements (after player is drawn)
    pub fn render_foreground(&self, map: &MapData, camera_x: f32, camera_y: f32) {
        // Draw backgrounds marked as "front"
        self.render_backgrounds(map, camera_x, camera_y, true);
    }

    /// Render background layers with parallax scrolling
    fn render_backgrounds(&self, map: &MapData, camera_x: f32, camera_y: f32, front_only: bool) {
        for bg in &map.backgrounds {
            // Skip if we're rendering front layers only and this isn't front
            // Or if we're rendering back layers and this is front
            if front_only != bg.front {
                continue;
            }

            // Calculate parallax offset
            // rx and ry are scroll ratios (0-100, where 0 = static, 100 = moves with camera)
            let parallax_x = if bg.rx != 0 {
                -(camera_x * bg.rx as f32 / 100.0)
            } else {
                0.0
            };

            let parallax_y = if bg.ry != 0 {
                -(camera_y * bg.ry as f32 / 100.0)
            } else {
                0.0
            };

            // Calculate screen position
            let screen_x = bg.x as f32 + parallax_x - camera_x;
            let screen_y = bg.y as f32 + parallax_y - camera_y;

            // Calculate alpha
            let alpha = (bg.a as f32 / 255.0 * 255.0) as u8;
            let color = Color::from_rgba(255, 255, 255, alpha);

            // Draw a placeholder for the background
            // TODO: Load and draw actual background texture
            // For now, draw a colored rectangle to show the layer
            let bg_color = match bg.layer_num % 5 {
                0 => Color::from_rgba(50, 50, 100, alpha / 3),
                1 => Color::from_rgba(60, 60, 110, alpha / 3),
                2 => Color::from_rgba(70, 70, 120, alpha / 3),
                3 => Color::from_rgba(80, 80, 130, alpha / 3),
                _ => Color::from_rgba(90, 90, 140, alpha / 3),
            };

            // Draw a simple gradient rectangle as placeholder
            draw_rectangle(
                screen_x,
                screen_y,
                screen_width(),
                screen_height(),
                bg_color,
            );

            // Draw layer info for debugging
            if flags::SHOW_DEBUG_UI {
                let info = format!("BG Layer {}: {}/{} @({},{})",
                    bg.layer_num, bg.bS, bg.no, bg.x, bg.y);
                draw_text(&info, screen_x + 10.0, screen_y + 20.0, 16.0, WHITE);
            }
        }
    }

    /// Render footholds (platforms) for debugging
    fn render_footholds(&self, map: &MapData, camera_x: f32, camera_y: f32) {
        for fh in &map.footholds {
            let screen_x1 = fh.x1 as f32 - camera_x;
            let screen_y1 = fh.y1 as f32 - camera_y;
            let screen_x2 = fh.x2 as f32 - camera_x;
            let screen_y2 = fh.y2 as f32 - camera_y;

            // Color based on layer
            let color = match fh.layer % 7 {
                0 => GREEN,
                1 => BLUE,
                2 => YELLOW,
                3 => PURPLE,
                4 => ORANGE,
                5 => PINK,
                _ => RED,
            };

            // Draw the foothold line
            draw_line(screen_x1, screen_y1, screen_x2, screen_y2, 2.0, color);

            // Draw endpoints
            draw_circle(screen_x1, screen_y1, 3.0, color);
            draw_circle(screen_x2, screen_y2, 3.0, color);
        }
    }

    /// Render portals for debugging
    fn render_portals(&self, map: &MapData, camera_x: f32, camera_y: f32) {
        for portal in &map.portals {
            let screen_x = portal.x as f32 - camera_x;
            let screen_y = portal.y as f32 - camera_y;

            // Draw portal as a circle
            let color = match portal.pt {
                0 => BLUE,      // Spawn point
                2 => GREEN,     // Regular portal
                3 => RED,       // Auto-enter
                _ => PURPLE,
            };

            draw_circle(screen_x, screen_y, 10.0, color);
            draw_circle_lines(screen_x, screen_y, 10.0, 2.0, WHITE);

            // Draw portal name
            if flags::SHOW_DEBUG_UI {
                let name = if !portal.pn.is_empty() {
                    &portal.pn
                } else {
                    "unnamed"
                };
                draw_text(name, screen_x - 20.0, screen_y - 15.0, 14.0, WHITE);
            }
        }
    }

    /// Render map bounds for debugging
    fn render_bounds(&self, map: &MapData, camera_x: f32, camera_y: f32) {
        let left = map.info.vr_left as f32 - camera_x;
        let top = map.info.vr_top as f32 - camera_y;
        let right = map.info.vr_right as f32 - camera_x;
        let bottom = map.info.vr_bottom as f32 - camera_y;

        let width = right - left;
        let height = bottom - top;

        // Draw boundary rectangle
        draw_rectangle_lines(left, top, width, height, 2.0, RED);

        // Draw corner markers
        let marker_size = 20.0;
        draw_line(left, top, left + marker_size, top, 3.0, RED);
        draw_line(left, top, left, top + marker_size, 3.0, RED);

        draw_line(right, top, right - marker_size, top, 3.0, RED);
        draw_line(right, top, right, top + marker_size, 3.0, RED);

        draw_line(left, bottom, left + marker_size, bottom, 3.0, RED);
        draw_line(left, bottom, left, bottom - marker_size, 3.0, RED);

        draw_line(right, bottom, right - marker_size, bottom, 3.0, RED);
        draw_line(right, bottom, right, bottom - marker_size, 3.0, RED);
    }

    /// Get ground Y position at given X coordinate
    pub fn get_ground_y(&self, map: &MapData, x: f32) -> Option<f32> {
        map.find_foothold_at(x, map.info.vr_bottom as f32)
            .map(|fh| {
                // Calculate Y on the foothold
                let dx = fh.x2 - fh.x1;
                let dy = fh.y2 - fh.y1;
                let ix = x as i32;

                if dx != 0 {
                    (fh.y1 + ((ix - fh.x1) * dy) / dx) as f32
                } else {
                    fh.y1 as f32
                }
            })
    }
}
