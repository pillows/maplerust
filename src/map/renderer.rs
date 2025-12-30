use macroquad::prelude::*;
use crate::map::data::*;
use crate::flags;
use crate::game_world::bot_ai::BotAI;

pub struct MapRenderer {
    debug_footholds: bool,
    debug_portals: bool,
    debug_bounds: bool,
    npc_font: Option<Font>,
}

impl MapRenderer {
    pub fn new() -> Self {
        Self {
            debug_footholds: flags::SHOW_HITBOXES,
            debug_portals: true, // Show portal circles for debugging
            debug_bounds: false,
            npc_font: None,
        }
    }

    /// Load the NPC name font (call this once during initialization)
    pub async fn load_font(&mut self) {
        // Load Liberation Sans Bold font (metric-compatible Arial Bold replacement)
        // Liberation Sans is an open-source font that looks identical to Arial
        info!("Loading Arial Bold-compatible font for NPC names...");

        match load_ttf_font("https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/MaplestoryLight.ttf").await {
            Ok(font) => {
                info!("Loaded Arial Bold-compatible font successfully");
                self.npc_font = Some(font);
            }
            Err(e) => {
                warn!("Failed to load Arial Bold-compatible font: {:?}, using default font", e);
            }
        }
    }


    /// Render the entire map at the given camera position
    pub fn render(&self, map: &MapData, camera_x: f32, camera_y: f32, bot_ai: Option<&BotAI>) {
        // Draw backgrounds (layers behind player)
        self.render_backgrounds(map, camera_x, camera_y, false);

        // Draw tiles (ground textures)
        self.render_tiles(map, camera_x, camera_y);

        // Draw objects (decorative elements)
        self.render_objects(map, camera_x, camera_y);

        // Draw NPCs and mobs
        self.render_life(map, camera_x, camera_y, bot_ai);

        // Draw footholds (platforms) for debugging
        if self.debug_footholds {
            self.render_footholds(map, camera_x, camera_y);
        }

        // Draw portals (always render)
        self.render_portals(map, camera_x, camera_y);

        // Draw map bounds for debugging
        if self.debug_bounds {
            self.render_bounds(map, camera_x, camera_y);
        }
    }

    /// Render foreground elements (after player is drawn)
    pub fn render_foreground(&self, map: &MapData, camera_x: f32, camera_y: f32, _bot_ai: Option<&BotAI>) {
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
            // Note: parallax already includes camera adjustment, so we don't subtract camera again
            let screen_x = bg.x as f32 + parallax_x;
            let screen_y = bg.y as f32 + parallax_y;

            // Calculate alpha
            let alpha = (bg.a as f32 / 255.0 * 255.0) as u8;
            let color = Color::from_rgba(255, 255, 255, alpha);

            // Draw the actual background texture if loaded
            if let Some(texture) = &bg.texture {
                // Calculate texture dimensions
                let tex_width = texture.width();
                let tex_height = texture.height();
                
                // Handle tiling (cx and cy control repetition)
                // If cx=0 or cy=0, tile infinitely to cover the visible screen
                let should_tile = bg.cx > 0 || bg.cy > 0 || tex_width < screen_width() || tex_height < screen_height();

                if should_tile {
                    // Calculate how many tiles needed to cover screen if cx/cy are 0
                    let repeat_x = if bg.cx > 0 {
                        bg.cx
                    } else {
                        // Tile horizontally to cover map width
                        ((map.get_width() as f32 / tex_width).ceil() as i32).max(1)
                    };

                    let repeat_y = if bg.cy > 0 {
                        bg.cy
                    } else {
                        // Tile vertically to cover map height
                        ((map.get_height() as f32 / tex_height).ceil() as i32).max(1)
                    };

                    for ty in 0..repeat_y {
                        for tx in 0..repeat_x {
                            let tile_x = screen_x + (tx as f32 * tex_width);
                            let tile_y = screen_y + (ty as f32 * tex_height);

                            draw_texture_ex(
                                texture,
                                tile_x,
                                tile_y,
                                color,
                                DrawTextureParams {
                                    flip_x: bg.flip_x,
                                    flip_y: bg.flip_y,
                                    ..Default::default()
                                },
                            );
                        }
                    }
                } else {
                    // Draw single tile
                    draw_texture_ex(
                        texture,
                        screen_x,
                        screen_y,
                        color,
                        DrawTextureParams {
                            flip_x: bg.flip_x,
                            flip_y: bg.flip_y,
                            ..Default::default()
                        },
                    );
                }

                // Draw layer info for debugging
                if flags::SHOW_DEBUG_UI {
                    let info = format!("BG Layer {}: {}/{} @({},{})",
                        bg.layer_num, bg.bS, bg.no, bg.x, bg.y);
                    draw_text(&info, screen_x + 10.0, screen_y + 20.0, 16.0, WHITE);
                }
            }
        }
    }

    /// Render tiles (ground textures)
    fn render_tiles(&self, map: &MapData, camera_x: f32, camera_y: f32) {
        // Get screen dimensions for culling
        let screen_width = screen_width();
        let screen_height = screen_height();

        // NOTE: Tiles should be pre-sorted by z_m when the map loads
        // If not sorted, uncomment the sorting code (but this hurts FPS)
        for tile in &map.tiles {
            // Calculate screen position (tiles don't have parallax scrolling)
            // Apply origin offset: origin defines the anchor point of the sprite
            let screen_x = tile.x as f32 - camera_x - tile.origin_x as f32;
            let screen_y = tile.y as f32 - camera_y - tile.origin_y as f32;

            // Screen culling - skip tiles outside view (with margin for tile size)
            if screen_x < -100.0 || screen_x > screen_width + 100.0
                || screen_y < -100.0 || screen_y > screen_height + 100.0 {
                continue;
            }

            // Draw the tile texture if loaded
            if let Some(texture) = &tile.texture {
                draw_texture(texture, screen_x, screen_y, WHITE);

                // Draw tile info for debugging
                if flags::SHOW_DEBUG_UI {
                    let info = format!("T{}: {}/{}", tile.id, tile.u, tile.no);
                    draw_text(&info, screen_x + 5.0, screen_y + 15.0, 12.0, YELLOW);
                }
            } else {
                // Draw placeholder for missing tile texture
                if flags::SHOW_DEBUG_UI {
                    draw_rectangle(screen_x, screen_y, 90.0, 60.0, Color::from_rgba(100, 50, 0, 100));
                    let info = format!("T{}", tile.id);
                    draw_text(&info, screen_x + 5.0, screen_y + 30.0, 12.0, RED);
                }
            }
        }
    }

    /// Render objects (decorative elements)
    fn render_objects(&self, map: &MapData, camera_x: f32, camera_y: f32) {
        // Get screen dimensions for culling
        let screen_width = screen_width();
        let screen_height = screen_height();

        // NOTE: Objects should be pre-sorted by z when the map loads
        // If not sorted, uncomment the sorting code (but this hurts FPS)
        for obj in &map.objects {
            // Calculate screen position (objects don't have parallax scrolling)
            // Apply origin offset: origin defines the anchor point of the sprite
            let screen_x = obj.x as f32 - camera_x - obj.origin_x as f32;
            let screen_y = obj.y as f32 - camera_y - obj.origin_y as f32;

            // Screen culling - skip objects outside view (with generous margin)
            if screen_x < -200.0 || screen_x > screen_width + 200.0
                || screen_y < -200.0 || screen_y > screen_height + 200.0 {
                continue;
            }

            // Draw the object texture if loaded
            if let Some(texture) = &obj.texture {
                let params = DrawTextureParams {
                    flip_x: obj.f,
                    flip_y: false,
                    rotation: (obj.r as f32).to_radians(),
                    ..Default::default()
                };
                draw_texture_ex(texture, screen_x, screen_y, WHITE, params);

                // Draw object info for debugging
                if flags::SHOW_DEBUG_UI {
                    let info = format!("O{}: {}", obj.id, obj.oS);
                    draw_text(&info, screen_x + 5.0, screen_y + 15.0, 12.0, ORANGE);
                }
            } else {
                // Draw placeholder for missing object texture
                if flags::SHOW_DEBUG_UI {
                    draw_circle(screen_x, screen_y, 5.0, Color::from_rgba(255, 165, 0, 150));
                    let info = format!("O{}", obj.id);
                    draw_text(&info, screen_x + 10.0, screen_y + 5.0, 12.0, ORANGE);
                }
            }
        }
    }

    /// Render life (NPCs and mobs)
    fn render_life(&self, map: &MapData, camera_x: f32, camera_y: f32, bot_ai: Option<&BotAI>) {
        for life in &map.life {
            // Skip if hidden
            if life.hide {
                continue;
            }

            // Get position from bot AI if this is a mob, otherwise use static position
            let (pos_x, pos_y, flip) = if life.life_type == "m" {
                if let Some(bot_ai) = bot_ai {
                    if let Some(bot) = bot_ai.get_bot_state(&life.id) {
                        (bot.x, bot.y, !bot.facing_right)
                    } else {
                        (life.x as f32, life.y as f32, life.flip)
                    }
                } else {
                    (life.x as f32, life.y as f32, life.flip)
                }
            } else {
                // NPCs use static positions, but snap Y to foothold if available
                let mut npc_x = life.x as f32;
                let mut npc_y = life.y as f32;
                
                // Snap NPC to foothold if it has one
                if life.foothold != 0 {
                    if let Some(fh) = map.footholds.iter().find(|fh| fh.id == life.foothold) {
                        // Calculate Y position on the foothold at NPC's X
                        let dx = fh.x2 - fh.x1;
                        let dy = fh.y2 - fh.y1;
                        let ix = npc_x as i32;
                        
                        let fh_y = if dx != 0 {
                            (fh.y1 + ((ix - fh.x1) * dy) / dx) as f32
                        } else {
                            fh.y1 as f32
                        };
                        
                        // Use foothold Y, but keep original X
                        npc_y = fh_y;
                    }
                }
                
                (npc_x, npc_y, life.flip)
            };

            // Calculate screen position
            // Apply origin offset: origin defines the anchor point of the sprite
            let screen_x = pos_x - camera_x - life.origin_x as f32;
            let screen_y = pos_y - camera_y - life.origin_y as f32;

            // Draw the NPC/mob texture if loaded
            if let Some(texture) = &life.texture {
                let params = DrawTextureParams {
                    flip_x: flip,
                    flip_y: false,
                    ..Default::default()
                };
                draw_texture_ex(texture, screen_x, screen_y, WHITE, params);

                // Draw NPC name label underneath sprite (for NPCs only)
                if life.life_type == "n" && !life.name.is_empty() {
                    let font_size = 12.0;

                    // Measure text with the custom font if available
                    let text_dims = if let Some(font) = &self.npc_font {
                        measure_text(&life.name, Some(font), font_size as u16, 1.0)
                    } else {
                        measure_text(&life.name, None, font_size as u16, 1.0)
                    };

                    let label_x = screen_x + (texture.width() / 2.0) - (text_dims.width / 2.0);
                    let label_y = screen_y + texture.height() + 20.0;

                    // Draw text background for better readability
                    let padding = 4.0;
                    draw_rectangle(
                        label_x - padding,
                        label_y - text_dims.height - padding,
                        text_dims.width + padding * 2.0,
                        text_dims.height + padding * 2.0,
                        Color::from_rgba(0, 0, 0, 150),
                    );

                    // Draw NPC name in yellow with custom font
                    if let Some(font) = &self.npc_font {
                        draw_text_ex(&life.name, label_x, label_y, TextParams {
                            font: Some(font),
                            font_size: font_size as u16,
                            color: YELLOW,
                            ..Default::default()
                        });
                    } else {
                        draw_text(&life.name, label_x, label_y, font_size, YELLOW);
                    }
                }

                // Draw life info for debugging
                if flags::SHOW_DEBUG_UI {
                    let info = if life.life_type == "n" {
                        format!("NPC ID: {}", life.id)
                    } else {
                        format!("Mob: {}", life.id)
                    };
                    draw_text(&info, screen_x + 5.0, screen_y - 10.0, 10.0, YELLOW);
                }
            } else {
                // Draw placeholder for missing life texture
                if flags::SHOW_DEBUG_UI {
                    let color = if life.life_type == "n" {
                        GREEN
                    } else {
                        RED
                    };
                    draw_circle(screen_x, screen_y, 8.0, Color::from_rgba(color.r as u8, color.g as u8, color.b as u8, 150));
                    let info = if life.life_type == "n" {
                        if !life.name.is_empty() {
                            life.name.clone()
                        } else {
                            format!("NPC:{}", life.id)
                        }
                    } else {
                        format!("M:{}", life.id)
                    };
                    draw_text(&info, screen_x + 10.0, screen_y + 5.0, 10.0, color);
                }
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

    /// Render portals
    fn render_portals(&self, map: &MapData, camera_x: f32, camera_y: f32) {
        // Skip portal rendering if disabled via flag
        if !flags::RENDER_PORTALS {
            return;
        }

        // Get current time for animation (cross-platform, works in WASM)
        let now = get_time() as f32 * 1000.0; // Convert to milliseconds

        // Get screen dimensions for culling
        let screen_w = screen_width();
        let screen_h = screen_height();

        // Log portal rendering stats once
        static mut LOGGED_PORTAL_STATS: bool = false;
        if unsafe { !LOGGED_PORTAL_STATS } {
            let total_portals = map.portals.len();
            let with_textures = map.portals.iter().filter(|p| !p.textures.is_empty()).count();
            // info!("=== PORTAL RENDERING ===");
            // info!("  Total portals in map: {}", total_portals);
            // info!("  Portals with textures: {}", with_textures);
            // info!("  Portals without textures: {}", total_portals - with_textures);

            // Show first few portal details
            for (i, portal) in map.portals.iter().take(3).enumerate() {
                //     info!("  Portal {}: type={}, textures={}, pos=({},{})",
                //           i, portal.pt, portal.textures.len(), portal.x, portal.y);
            }
            // info!("========================");
            unsafe { LOGGED_PORTAL_STATS = true; }
        }

        for portal in &map.portals {
            let screen_x = portal.x as f32 - camera_x;
            let screen_y = portal.y as f32 - camera_y;

            // Screen culling - skip portals outside view (with 100px margin)
            if screen_x < -100.0 || screen_x > screen_w + 100.0
                || screen_y < -100.0 || screen_y > screen_h + 100.0 {
                continue;
            }

            // Render portal using its own textures
            if !portal.textures.is_empty() && !portal.origins.is_empty() {
                // Animate portals (8 fps)
                let frame_count = portal.textures.len();
                let frame_idx = ((now / 125.0) as usize) % frame_count; // 125ms per frame = 8 fps

                if let (Some(texture), Some(&(origin_x, origin_y))) =
                    (portal.textures.get(frame_idx), portal.origins.get(frame_idx)) {

                    // Apply origin offset for proper positioning
                    // Each frame has its own origin, ensuring stable vertical position
                    let draw_x = screen_x - origin_x as f32;
                    let draw_y = screen_y - origin_y as f32;

                    draw_texture(texture, draw_x, draw_y, WHITE);

                    // Draw portal name for debugging
                    if self.debug_portals && flags::SHOW_DEBUG_UI {
                        let name = if !portal.pn.is_empty() {
                            &portal.pn
                        } else {
                            "unnamed"
                        };
                        draw_text(name, screen_x - 20.0, screen_y - texture.height() - 5.0, 14.0, WHITE);
                    }
                    continue; // Texture rendered successfully
                }
            } else if portal.pt != 0 && portal.pt != 1 && portal.pt != 10 {
                // Only show debug for visible portal types (not sp, pi, or ph)
                // Do nothing for invisible portals
            }

            // Fall back to debug circle if no texture available
            if self.debug_portals {
                let color = match portal.pt {
                    0 => BLUE,      // Spawn point
                    2 => GREEN,     // Regular portal
                    3 => RED,       // Auto-enter
                    6 => YELLOW,    // Type 6 portal
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
