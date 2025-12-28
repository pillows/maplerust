use macroquad::prelude::*;
use crate::character::CharacterData;
use crate::flags::{self, DebugFlags};
use crate::map::{MapData, MapLoader, MapRenderer};

/// Gameplay state for when the player is in the game world
pub struct GameplayState {
    character: CharacterData,
    camera_x: f32,
    camera_y: f32,
    player_x: f32,
    player_y: f32,
    player_vy: f32,  // Vertical velocity for gravity
    on_ground: bool,
    loaded: bool,
    map_data: Option<MapData>,
    map_renderer: MapRenderer,
    current_map_id: String,
}

impl GameplayState {
    pub fn new(character: CharacterData) -> Self {
        info!("Starting game with character: {}", character.name);
        Self {
            character,
            camera_x: 0.0,
            camera_y: 0.0,
            player_x: 400.0,
            player_y: 100.0,
            player_vy: 0.0,
            on_ground: false,
            loaded: false,
            map_data: None,
            map_renderer: MapRenderer::new(),
            current_map_id: "000010000".to_string(), // Default starting map
        }
    }

    /// Load game assets including the map
    pub async fn load_assets(&mut self) {
        info!("Loading gameplay assets...");

        // Load the starting map
        match MapLoader::load_map(&self.current_map_id).await {
            Ok(map) => {
                info!("Map loaded successfully!");

                // Find spawn portal (portal type 0) to position player
                if let Some(spawn_portal) = map.portals.iter().find(|p| p.pt == 0) {
                    self.player_x = spawn_portal.x as f32;
                    self.player_y = spawn_portal.y as f32;
                    info!("Player spawned at portal: ({}, {})", self.player_x, self.player_y);
                } else {
                    // Default spawn position in center of map
                    self.player_x = ((map.info.vr_left + map.info.vr_right) / 2) as f32;
                    self.player_y = map.info.vr_top as f32 + 100.0;
                    info!("No spawn portal found, using default position");
                }

                self.map_data = Some(map);
            }
            Err(e) => {
                error!("Failed to load map: {}", e);
            }
        }

        self.loaded = true;
        info!("Gameplay assets loaded successfully");
    }

    /// Update game logic
    pub fn update(&mut self, dt: f32) {
        if !self.loaded || self.map_data.is_none() {
            return;
        }

        let map = self.map_data.as_ref().unwrap();

        // Basic player movement with debug speed multiplier
        let base_speed = 200.0;
        let move_speed = DebugFlags::get_player_speed(base_speed);

        // Horizontal movement
        if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            self.player_x -= move_speed * dt;
        }
        if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            self.player_x += move_speed * dt;
        }

        // Jumping
        if (is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W)) && self.on_ground {
            self.player_vy = -400.0; // Jump velocity
            self.on_ground = false;
        }

        // Apply gravity
        if !flags::ENABLE_COLLISION || !flags::GOD_MODE {
            let gravity = 800.0;
            self.player_vy += gravity * dt;
        }

        // Update vertical position
        self.player_y += self.player_vy * dt;

        // Check collision with footholds
        if flags::ENABLE_COLLISION {
            if let Some(fh) = map.find_foothold_at(self.player_x, self.player_y + 30.0) {
                // Calculate Y on the foothold
                let dx = fh.x2 - fh.x1;
                let dy = fh.y2 - fh.y1;
                let ix = self.player_x as i32;

                let fh_y = if dx != 0 {
                    (fh.y1 + ((ix - fh.x1) * dy) / dx) as f32
                } else {
                    fh.y1 as f32
                };

                // Snap player to foothold if falling through it
                if self.player_y + 30.0 >= fh_y && self.player_vy >= 0.0 {
                    self.player_y = fh_y - 30.0;
                    self.player_vy = 0.0;
                    self.on_ground = true;
                } else {
                    self.on_ground = false;
                }
            } else {
                self.on_ground = false;
            }
        } else {
            // No collision - simple boundary check
            if self.player_y > map.info.vr_bottom as f32 - 60.0 {
                self.player_y = map.info.vr_bottom as f32 - 60.0;
                self.player_vy = 0.0;
                self.on_ground = true;
            }
        }

        // Clamp player to map bounds
        self.player_x = self.player_x.max(map.info.vr_left as f32).min(map.info.vr_right as f32);
        self.player_y = self.player_y.max(map.info.vr_top as f32).min(map.info.vr_bottom as f32);

        // Camera follows player (unless in camera debug mode)
        if !flags::CAMERA_DEBUG_MODE {
            let map_width = (map.info.vr_right - map.info.vr_left) as f32;
            let map_height = (map.info.vr_bottom - map.info.vr_top) as f32;

            // Center on player
            self.camera_x = self.player_x - screen_width() / 2.0;
            self.camera_y = self.player_y - screen_height() / 2.0;

            // Clamp camera to map bounds
            self.camera_x = self.camera_x.max(map.info.vr_left as f32)
                .min((map.info.vr_right as f32 - screen_width()).max(map.info.vr_left as f32));
            self.camera_y = self.camera_y.max(map.info.vr_top as f32)
                .min((map.info.vr_bottom as f32 - screen_height()).max(map.info.vr_top as f32));
        } else {
            // Camera debug mode - move camera independently with arrow keys + Shift
            let camera_speed = 300.0;
            if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
                if is_key_down(KeyCode::Left) {
                    self.camera_x -= camera_speed * dt;
                }
                if is_key_down(KeyCode::Right) {
                    self.camera_x += camera_speed * dt;
                }
                if is_key_down(KeyCode::Up) {
                    self.camera_y -= camera_speed * dt;
                }
                if is_key_down(KeyCode::Down) {
                    self.camera_y += camera_speed * dt;
                }
            }
        }
    }

    /// Draw the game
    pub fn draw(&self) {
        clear_background(Color::from_rgba(135, 206, 235, 255)); // Sky blue

        if !self.loaded {
            let text = "Loading Map...";
            let font_size = 32.0;
            let text_dimensions = measure_text(text, None, font_size as u16, 1.0);
            draw_text(
                text,
                screen_width() / 2.0 - text_dimensions.width / 2.0,
                screen_height() / 2.0,
                font_size,
                WHITE,
            );
            return;
        }

        if let Some(ref map) = self.map_data {
            // Render map backgrounds (behind player)
            self.map_renderer.render(map, self.camera_x, self.camera_y);

            // Draw player (simple square for now)
            let player_screen_x = self.player_x - self.camera_x;
            let player_screen_y = self.player_y - self.camera_y;
            draw_rectangle(player_screen_x - 15.0, player_screen_y - 30.0, 30.0, 60.0, BLUE);

            // Draw player hitbox if enabled
            if flags::SHOW_HITBOXES {
                draw_rectangle_lines(
                    player_screen_x - 15.0,
                    player_screen_y - 30.0,
                    30.0,
                    60.0,
                    2.0,
                    YELLOW,
                );
            }

            // Render map foregrounds (in front of player)
            self.map_renderer.render_foreground(map, self.camera_x, self.camera_y);
        } else {
            let text = "No map loaded";
            draw_text(text, 20.0, 40.0, 20.0, RED);
        }

        // Draw UI
        self.draw_ui();
    }

    /// Draw the game UI
    fn draw_ui(&self) {
        // Draw character info panel at top-left
        let panel_x = 10.0;
        let panel_y = 10.0;
        let panel_width = 200.0;
        let mut panel_height = 100.0;

        // Extend panel if debug UI is enabled
        if DebugFlags::should_show_debug_ui() {
            panel_height = 180.0;
        }

        // Background
        draw_rectangle(
            panel_x,
            panel_y,
            panel_width,
            panel_height,
            Color::from_rgba(0, 0, 0, 180),
        );

        // Character name
        draw_text(
            &self.character.name,
            panel_x + 10.0,
            panel_y + 25.0,
            20.0,
            WHITE,
        );

        // Level
        let level_text = format!("Level: {}", self.character.level);
        draw_text(&level_text, panel_x + 10.0, panel_y + 45.0, 16.0, WHITE);

        // HP
        let hp_text = format!("HP: {}/{}", self.character.hp, self.character.hp);
        draw_text(&hp_text, panel_x + 10.0, panel_y + 65.0, 16.0, GREEN);

        // MP
        let mp_text = format!("MP: {}/{}", self.character.mp, self.character.mp);
        draw_text(&mp_text, panel_x + 10.0, panel_y + 85.0, 16.0, BLUE);

        // Debug info
        if DebugFlags::should_show_debug_ui() {
            let fps = get_fps();
            let fps_text = format!("FPS: {}", fps);
            draw_text(&fps_text, panel_x + 10.0, panel_y + 105.0, 14.0, YELLOW);

            let pos_text = format!("Pos: ({:.0}, {:.0})", self.player_x, self.player_y);
            draw_text(&pos_text, panel_x + 10.0, panel_y + 125.0, 14.0, YELLOW);

            let cam_text = format!("Cam: ({:.0}, {:.0})", self.camera_x, self.camera_y);
            draw_text(&cam_text, panel_x + 10.0, panel_y + 145.0, 14.0, YELLOW);

            if flags::GOD_MODE {
                draw_text("GOD MODE", panel_x + 10.0, panel_y + 165.0, 14.0, RED);
            }
        }

        // Show hitboxes if enabled
        if flags::SHOW_HITBOXES {
            let player_screen_x = self.player_x - self.camera_x;
            let player_screen_y = self.player_y - self.camera_y;
            draw_rectangle_lines(
                player_screen_x - 15.0,
                player_screen_y - 30.0,
                30.0,
                60.0,
                2.0,
                RED,
            );
        }

        // Controls hint at bottom
        let mut controls_text = "Controls: Arrow Keys or WASD to move".to_string();
        if flags::CAMERA_DEBUG_MODE {
            controls_text.push_str(" | Shift+Arrows: Camera");
        }
        let font_size = 16.0;
        let text_dimensions = measure_text(&controls_text, None, font_size as u16, 1.0);
        draw_text(
            &controls_text,
            screen_width() / 2.0 - text_dimensions.width / 2.0,
            screen_height() - 20.0,
            font_size,
            WHITE,
        );
    }
}
