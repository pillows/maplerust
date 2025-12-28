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
    // Debug map loader
    map_input: String,
    map_input_active: bool,
    loading_new_map: bool,
    backspace_timer: f32,
    backspace_repeat_delay: f32,
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
            current_map_id: "100000000".to_string(), // Default starting map
            map_input: String::new(),
            map_input_active: false,
            loading_new_map: false,
            backspace_timer: 0.0,
            backspace_repeat_delay: 0.05, // Repeat every 50ms when held
        }
    }

    /// Load game assets including the map
    pub async fn load_assets(&mut self) {
        info!("Loading gameplay assets...");

        // Load font for NPC names
        self.map_renderer.load_font().await;

        self.load_map(&self.current_map_id.clone()).await;
        self.loaded = true;
        info!("Gameplay assets loaded successfully");
    }

    /// Load a specific map by ID
    async fn load_map(&mut self, map_id: &str) {
        info!("Loading map: {}", map_id);

        match MapLoader::load_map(map_id).await {
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

                self.current_map_id = map_id.to_string();
                self.map_data = Some(map);
                self.loading_new_map = false;
            }
            Err(e) => {
                error!("Failed to load map {}: {}", map_id, e);
                self.loading_new_map = false;
            }
        }
    }

    /// Trigger loading a new map from the debug input
    pub async fn load_map_from_input(&mut self) {
        if !self.map_input.is_empty() {
            self.loading_new_map = true;
            let map_id = self.map_input.clone();
            self.load_map(&map_id).await;
            self.map_input.clear();
            self.map_input_active = false;
        }
    }

    /// Check if we should load a new map and return the map ID
    pub fn should_load_new_map(&mut self) -> Option<String> {
        if self.loading_new_map && !self.map_input.is_empty() {
            Some(self.map_input.clone())
        } else {
            None
        }
    }

    /// Update game logic
    pub fn update(&mut self, dt: f32) {
        // Handle debug map input toggle with M key
        if DebugFlags::should_show_debug_ui() && is_key_pressed(KeyCode::M) {
            if self.map_input_active {
                // Close the input and trigger loading if valid
                if !self.map_input.is_empty() {
                    self.loading_new_map = true;
                    self.map_input_active = false;
                } else {
                    // Just close if empty
                    self.map_input_active = false;
                    self.map_input.clear();
                }
            } else {
                // Open the input
                self.map_input_active = true;
                self.map_input = self.current_map_id.clone();
            }
        }

        // Handle text input when map input is active
        if self.map_input_active {
            // Handle clipboard paste (Ctrl/Cmd + V)
            // Check if modifier key is held AND V is pressed (not just down, to avoid repeats)
            let modifier_held = if cfg!(target_os = "macos") {
                is_key_down(KeyCode::LeftSuper) || is_key_down(KeyCode::RightSuper)
            } else {
                is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl)
            };
            
            let paste_pressed = modifier_held && is_key_pressed(KeyCode::V);

            if paste_pressed {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    use clipboard::{ClipboardProvider, ClipboardContext};
                    match ClipboardContext::new() {
                        Ok(mut ctx) => {
                            match ctx.get_contents() {
                                Ok(contents) => {
                                    // Filter to only digits (no length limit)
                                    let filtered: String = contents.chars()
                                        .filter(|c| c.is_ascii_digit())
                                        .collect();
                                    if !filtered.is_empty() {
                                        self.map_input = filtered;
                                        info!("Pasted map ID: {}", self.map_input);
                                    } else {
                                        warn!("Clipboard contents had no digits");
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to get clipboard contents: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to create clipboard context: {}", e);
                        }
                    }
                }
                #[cfg(target_arch = "wasm32")]
                {
                    warn!("Clipboard paste not supported in WASM build");
                }
            }

            // Get character input - no length limit, can type map IDs of any length
            if let Some(character) = get_char_pressed() {
                if character.is_ascii_digit() {
                    self.map_input.push(character);
                }
            }

            // Handle backspace with hold support
            if is_key_down(KeyCode::Backspace) && !self.map_input.is_empty() {
                if is_key_pressed(KeyCode::Backspace) {
                    // First press - delete immediately
                    self.map_input.pop();
                    self.backspace_timer = 0.3; // Initial delay before repeat
                } else {
                    // Held down - use timer for repeat
                    self.backspace_timer -= dt;
                    if self.backspace_timer <= 0.0 {
                        self.map_input.pop();
                        self.backspace_timer = self.backspace_repeat_delay;
                    }
                }
            } else {
                self.backspace_timer = 0.0;
            }

            // Handle escape to close
            if is_key_pressed(KeyCode::Escape) {
                self.map_input_active = false;
                self.map_input.clear();
            }

            // Don't process game controls when input is active
            return;
        }

        // Check if we should load a map (triggered from elsewhere after Enter is pressed)
        if self.loading_new_map {
            return; // Don't process game logic while loading
        }

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
            let font_size = 12.0;
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

        // Extend panel if debug UI is enabled (need more space for map name)
        if DebugFlags::should_show_debug_ui() {
            panel_height = 200.0;
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
            let mut y_offset = panel_y + 105.0;
            let line_height = 20.0;
            
            let fps = get_fps();
            let fps_text = format!("FPS: {}", fps);
            draw_text(&fps_text, panel_x + 10.0, y_offset, 14.0, YELLOW);
            y_offset += line_height;

            // Map ID and name
            if let Some(map_data) = &self.map_data {
                let map_text = if !map_data.info.map_name.is_empty() {
                    format!("Map: {} ({})", self.current_map_id, map_data.info.map_name)
                } else {
                    format!("Map: {}", self.current_map_id)
                };
                draw_text(&map_text, panel_x + 10.0, y_offset, 14.0, YELLOW);
                y_offset += line_height;
            } else {
                let map_text = format!("Map: {} (loading...)", self.current_map_id);
                draw_text(&map_text, panel_x + 10.0, y_offset, 14.0, YELLOW);
                y_offset += line_height;
            }

            let pos_text = format!("Pos: ({:.0}, {:.0})", self.player_x, self.player_y);
            draw_text(&pos_text, panel_x + 10.0, y_offset, 14.0, YELLOW);
            y_offset += line_height;

            let cam_text = format!("Cam: ({:.0}, {:.0})", self.camera_x, self.camera_y);
            draw_text(&cam_text, panel_x + 10.0, y_offset, 14.0, YELLOW);
            y_offset += line_height;

            if flags::GOD_MODE {
                draw_text("GOD MODE", panel_x + 10.0, y_offset, 14.0, RED);
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

        // Debug map loader UI
        if DebugFlags::should_show_debug_ui() {
            self.draw_map_loader_ui();
        }

        // Controls hint at bottom
        let mut controls_text = "Controls: Arrow Keys or WASD to move".to_string();
        if flags::CAMERA_DEBUG_MODE {
            controls_text.push_str(" | Shift+Arrows: Camera");
        }
        if DebugFlags::should_show_debug_ui() {
            controls_text.push_str(" | M: Load Map");
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

    /// Draw the debug map loader UI
    fn draw_map_loader_ui(&self) {
        // Wider box to accommodate longer map IDs (no length limit)
        let box_width = 400.0;
        let box_height = 100.0;
        let box_x = screen_width() - box_width - 20.0;
        let box_y = 20.0;

        // Background
        draw_rectangle(
            box_x,
            box_y,
            box_width,
            box_height,
            Color::from_rgba(0, 0, 0, 200),
        );

        // Border
        let border_color = if self.map_input_active {
            YELLOW
        } else {
            GRAY
        };
        draw_rectangle_lines(box_x, box_y, box_width, box_height, 2.0, border_color);

        // Title
        draw_text("Map Loader (Press M)", box_x + 10.0, box_y + 25.0, 18.0, WHITE);

        // Current map
        let current_text = if let Some(map_data) = &self.map_data {
            if !map_data.info.map_name.is_empty() {
                format!("Current: {} ({})", self.current_map_id, map_data.info.map_name)
            } else {
                format!("Current: {}", self.current_map_id)
            }
        } else {
            format!("Current: {}", self.current_map_id)
        };
        draw_text(&current_text, box_x + 10.0, box_y + 50.0, 16.0, LIGHTGRAY);

        if self.map_input_active {
            // Input box
            let input_box_y = box_y + 60.0;
            draw_rectangle(
                box_x + 10.0,
                input_box_y,
                box_width - 20.0,
                25.0,
                Color::from_rgba(40, 40, 40, 255),
            );
            draw_rectangle_lines(
                box_x + 10.0,
                input_box_y,
                box_width - 20.0,
                25.0,
                1.0,
                YELLOW,
            );

            // Input text with cursor
            let input_display = format!("{}|", self.map_input);
            draw_text(&input_display, box_x + 15.0, input_box_y + 18.0, 16.0, WHITE);

            // Instructions
            let paste_key = if cfg!(target_os = "macos") { "Cmd" } else { "Ctrl" };
            let instructions = format!("Type or {}+V to paste, M to load, ESC to cancel", paste_key);
            draw_text(
                &instructions,
                box_x + 10.0,
                box_y + 95.0,
                11.0,
                LIGHTGRAY,
            );
        }

        // Show loading indicator
        if self.loading_new_map {
            draw_text("Loading...", box_x + box_width - 80.0, box_y + 25.0, 16.0, YELLOW);
        }
    }
}
