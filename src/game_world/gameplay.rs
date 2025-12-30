use macroquad::prelude::*;
use crate::character::CharacterData;
use crate::flags::{self, DebugFlags};
use crate::map::{MapData, MapLoader, MapRenderer, MobState, MobAI};
use crate::map::portal_loader::PortalCache;
use crate::game_world::bot_ai::BotAI;
use crate::audio::AudioManager;
use crate::cursor::{CursorManager, CursorState};

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
    portal_cache: PortalCache,
    current_map_id: String,
    target_portal_name: Option<String>, // Portal name to spawn at when entering new map
    bot_ai: BotAI, // Bot AI manager for mob movement
    mob_states: Vec<MobState>,
    on_ladder: bool,
    current_ladder_id: Option<i32>,
    drop_through_platform: bool, // True when jumping down through a platform
    foothold_min_x: f32, // Cached foothold extent
    foothold_max_x: f32,
    // Audio manager
    audio_manager: AudioManager,
    // BGM playback tracking
    bgm_pending: bool,
    // Debug map loader
    map_input: String,
    map_input_active: bool,
    loading_new_map: bool,
    backspace_timer: f32,
    backspace_repeat_delay: f32,
    // NPC interaction tracking
    last_npc_click_time: f32,
    last_npc_click_id: Option<String>,
    // Focus tracking
    window_focused: bool,
    last_dt: f32,
    // Cursor manager
    cursor_manager: CursorManager,
}

impl GameplayState {
    /// Create a new gameplay state
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
            portal_cache: PortalCache::new(),
            current_map_id: "100010000".to_string(), // Default starting map
            target_portal_name: None, // No target portal on initial spawn
            bot_ai: BotAI::new(),
            mob_states: Vec::new(),
            on_ladder: false,
            current_ladder_id: None,
            drop_through_platform: false,
            foothold_min_x: 0.0,
            foothold_max_x: 800.0,
            audio_manager: AudioManager::new(),
            bgm_pending: false,
            map_input: String::new(),
            map_input_active: false,
            loading_new_map: false,
            backspace_timer: 0.0,
            backspace_repeat_delay: 0.05, // Repeat every 50ms when held
            last_npc_click_time: -1.0,
            last_npc_click_id: None,
            window_focused: true,
            last_dt: 0.016, // Default to ~60fps
            cursor_manager: CursorManager::new(),
        }
    }

    /// Load game assets including the map
    pub async fn load_assets(&mut self) {
        info!("Loading gameplay assets...");

        // Load font for NPC names
        self.map_renderer.load_font().await;

        // Load custom MapleStory cursors
        self.cursor_manager.load_cursors().await;

        // Hide OS cursor so custom cursor can be used
        show_mouse(false);

        // Portal textures are now loaded during map parsing
        // Each portal has its own textures embedded in the Portal structure

        // Load map
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

                // Determine spawn position based on target portal or spawn point
                let mut spawn_x;
                let mut spawn_y;

                // If we have a target portal name (entered through a portal), use that
                // Otherwise, use the spawn portal (type 0)
                if let Some(ref target_portal_name) = self.target_portal_name {
                    // Find portal by name
                    if let Some(portal) = map.portals.iter().find(|p| p.pn == *target_portal_name) {
                        spawn_x = portal.x as f32;
                        spawn_y = portal.y as f32;
                        info!("Found target portal '{}' at: ({}, {})", target_portal_name, spawn_x, spawn_y);
                    } else {
                        // Fallback to spawn portal if target not found
                        warn!("Target portal '{}' not found, using spawn portal", target_portal_name);
                        if let Some(spawn_portal) = map.portals.iter().find(|p| p.pt == 0) {
                            spawn_x = spawn_portal.x as f32;
                            spawn_y = spawn_portal.y as f32;
                            info!("Found spawn portal at: ({}, {})", spawn_x, spawn_y);
                        } else {
                            spawn_x = ((map.info.vr_left + map.info.vr_right) / 2) as f32;
                            spawn_y = map.info.vr_top as f32 + 100.0;
                            info!("No spawn portal found, using default position: ({}, {})", spawn_x, spawn_y);
                        }
                    }
                } else {
                    // No target portal, use spawn portal (initial map load or debug map change)
                    if let Some(spawn_portal) = map.portals.iter().find(|p| p.pt == 0) {
                        spawn_x = spawn_portal.x as f32;
                        spawn_y = spawn_portal.y as f32;
                        info!("Found spawn portal at: ({}, {})", spawn_x, spawn_y);
                    } else {
                        // Default spawn position in center of map
                        spawn_x = ((map.info.vr_left + map.info.vr_right) / 2) as f32;
                        spawn_y = map.info.vr_top as f32 + 100.0;
                        info!("No spawn portal found, using default position: ({}, {})", spawn_x, spawn_y);
                    }
                }

                // Find the nearest foothold below the spawn point and place player on it
                // Don't clamp spawn position - use the actual portal/spawn location
                if let Some((foothold_y, _fh)) = map.find_foothold_below(spawn_x, spawn_y) {
                    self.player_x = spawn_x;
                    self.player_y = foothold_y - 30.0; // Subtract player height offset
                    self.player_vy = 0.0;
                    self.on_ground = true;
                    info!("Player placed on foothold at: ({}, {})", self.player_x, self.player_y);
                } else {
                    // No foothold found, use spawn position directly
                    self.player_x = spawn_x;
                    self.player_y = spawn_y;
                    self.player_vy = 0.0;
                    self.on_ground = false;
                    warn!("No foothold found below spawn point, player may fall");
                }

                // Initialize bot AI from map data
                self.bot_ai.initialize_from_map(&map);

                // Save viewport bounds before moving map
                let vr_left = map.info.vr_left as f32;
                let vr_right = map.info.vr_right as f32;
                let vr_top = map.info.vr_top as f32;
                let vr_bottom = map.info.vr_bottom as f32;

                // Log viewport bounds for debugging
                info!("Map viewport bounds:");
                info!("  VR_LEFT: {}, VR_RIGHT: {}", vr_left, vr_right);
                info!("  VR_TOP: {}, VR_BOTTOM: {}", vr_top, vr_bottom);
                info!("  Map width: {}, Map height: {}", vr_right - vr_left, vr_bottom - vr_top);

                // Calculate and cache foothold extent
                self.foothold_min_x = vr_left;
                self.foothold_max_x = vr_right;
                for fh in &map.footholds {
                    self.foothold_min_x = self.foothold_min_x.min(fh.x1.min(fh.x2) as f32);
                    self.foothold_max_x = self.foothold_max_x.max(fh.x1.max(fh.x2) as f32);
                }
                info!("Foothold extent: {} to {} (viewport: {} to {})",
                      self.foothold_min_x, self.foothold_max_x, vr_left, vr_right);

                // Stop any currently playing BGM before loading new map
                info!("Stopping previous map's BGM before loading new map");
                self.audio_manager.stop_bgm();

                // Store BGM name before moving map
                let bgm_name = map.info.bgm.clone();

                self.current_map_id = map_id.to_string();
                self.map_data = Some(map);
                self.loading_new_map = false;

                // Set BGM pending flag for playback in update method
                if !bgm_name.is_empty() {
                    info!("New map loaded with BGM: '{}' (will play after user interaction)", bgm_name);
                    self.bgm_pending = true;
                } else {
                    info!("New map loaded with no BGM");
                    self.bgm_pending = false;
                }

                // Initialize camera position - center on player but clamp to boundaries
                let target_camera_x = self.player_x - screen_width() / 2.0;
                let target_camera_y = self.player_y - screen_height() / 2.0;

                // Clamp X to foothold extent
                self.camera_x = target_camera_x
                    .max(self.foothold_min_x)
                    .min(self.foothold_max_x - screen_width());

                // Clamp Y only to bottom boundary (no top constraint)
                self.camera_y = target_camera_y
                    .min(vr_bottom - screen_height());

                // Clear target portal name after successful spawn
                self.target_portal_name = None;
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

    /// Handle BGM playback (async wrapper)
    pub async fn handle_bgm(&mut self) {
        // Resume audio context on first user interaction (required for browser autoplay policy)
        #[cfg(target_arch = "wasm32")]
        {
            use macroquad::prelude::*;
            // Check for any user interaction - check common keys and mouse buttons
            let any_key_pressed = is_key_pressed(KeyCode::Space) ||
                                  is_key_pressed(KeyCode::Enter) ||
                                  is_key_pressed(KeyCode::Escape) ||
                                  is_key_pressed(KeyCode::Left) ||
                                  is_key_pressed(KeyCode::Right) ||
                                  is_key_pressed(KeyCode::Up) ||
                                  is_key_pressed(KeyCode::Down) ||
                                  is_key_pressed(KeyCode::A) ||
                                  is_key_pressed(KeyCode::D) ||
                                  is_key_pressed(KeyCode::W) ||
                                  is_key_pressed(KeyCode::S);
            
            let any_mouse_pressed = is_mouse_button_pressed(MouseButton::Left) || 
                                    is_mouse_button_pressed(MouseButton::Right) || 
                                    is_mouse_button_pressed(MouseButton::Middle);
            
            if any_key_pressed || any_mouse_pressed {
                self.audio_manager.resume_audio_context().await;
            }
        }
        
        if self.bgm_pending {
            if let Some(ref map) = self.map_data {
                if !map.info.bgm.is_empty() {
                    info!("Playing BGM: {}", map.info.bgm);
                    self.audio_manager.play_bgm(&map.info.bgm).await;
                }
            }
            self.bgm_pending = false;
        }
    }

    /// Update game logic
    pub fn update(&mut self, dt: f32) {
        // Handle window focus - prevent large dt values when tab is inactive
        // Clamp dt to prevent physics issues when browser loses focus
        let clamped_dt = if dt > 0.1 {
            // If dt is very large (tab was inactive), use last known good dt
            self.last_dt.min(0.1)
        } else {
            dt
        };
        self.last_dt = clamped_dt;
        
        // Check window focus state (macroquad doesn't expose this directly, so we infer from dt)
        // If dt is reasonable, window is likely focused
        self.window_focused = dt < 0.1;
        
        // Resume audio context on first user interaction (any keypress or mouse click)
        #[cfg(target_arch = "wasm32")]
        {
            use macroquad::prelude::*;
            // Check for any user interaction - check common keys and mouse buttons
            let any_key_pressed = is_key_pressed(KeyCode::Space) ||
                                  is_key_pressed(KeyCode::Enter) ||
                                  is_key_pressed(KeyCode::Escape) ||
                                  is_key_pressed(KeyCode::Left) ||
                                  is_key_pressed(KeyCode::Right) ||
                                  is_key_pressed(KeyCode::Up) ||
                                  is_key_pressed(KeyCode::Down) ||
                                  is_key_pressed(KeyCode::A) ||
                                  is_key_pressed(KeyCode::D) ||
                                  is_key_pressed(KeyCode::W) ||
                                  is_key_pressed(KeyCode::S);
            
            let any_mouse_pressed = is_mouse_button_pressed(MouseButton::Left) || 
                                    is_mouse_button_pressed(MouseButton::Right);
            
            if any_key_pressed || any_mouse_pressed {
                // This will be handled asynchronously in handle_bgm
            }
        }
        // Portal textures are already loaded in each Portal structure during map parsing

        // Handle debug map input toggle with M key
        if DebugFlags::should_show_debug_ui() && is_key_pressed(KeyCode::M) {
            if self.map_input_active {
                // Close the input and trigger loading if valid
                if !self.map_input.is_empty() {
                    self.loading_new_map = true;
                    self.map_input_active = false;
                    // Clear target portal for debug map loading (use spawn portal)
                    self.target_portal_name = None;
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
                    self.backspace_timer -= clamped_dt;
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

        // Spacebar free-roam mode (no collision or gravity, full 2D movement)
        let free_roam = is_key_down(KeyCode::Space);

        // Basic player movement with debug speed multiplier
        let base_speed = if free_roam { 350.0 } else { 200.0 };
        let move_speed = DebugFlags::get_player_speed(base_speed);

        // Horizontal movement - no artificial boundaries
        // Player movement is limited by footholds, not viewport bounds
        if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            self.player_x -= move_speed * clamped_dt;
        }
        if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            self.player_x += move_speed * clamped_dt;
        }

        // Free-roam vertical movement (Space + Up/Down or W/S)
        if free_roam {
            if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
                self.player_y -= move_speed * clamped_dt;
            }
            if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
                self.player_y += move_speed * clamped_dt;
            }
        }
        
        // Handle NPC double-click interaction
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mouse_x, mouse_y) = mouse_position();
            let world_x = mouse_x + self.camera_x;
            let world_y = mouse_y + self.camera_y;
            
            // Check if click is on an NPC
            for life in &map.life {
                if life.life_type == "n" && !life.hide {
                    // Calculate NPC position (snapped to foothold if available)
                    let mut npc_x = life.x as f32;
                    let mut npc_y = life.y as f32;
                    
                    if life.foothold != 0 {
                        if let Some(fh) = map.footholds.iter().find(|fh| fh.id == life.foothold) {
                            let dx = fh.x2 - fh.x1;
                            let dy = fh.y2 - fh.y1;
                            let ix = npc_x as i32;
                            let fh_y = if dx != 0 {
                                (fh.y1 + ((ix - fh.x1) * dy) / dx) as f32
                            } else {
                                fh.y1 as f32
                            };
                            npc_y = fh_y;
                        }
                    }
                    
                    // Check if click is within NPC bounds (using texture size if available)
                    let npc_width = if let Some(tex) = &life.texture { tex.width() } else { 40.0 };
                    let npc_height = if let Some(tex) = &life.texture { tex.height() } else { 60.0 };
                    let npc_screen_x = npc_x - self.camera_x - life.origin_x as f32;
                    let npc_screen_y = npc_y - self.camera_y - life.origin_y as f32;
                    
                    if mouse_x >= npc_screen_x && mouse_x <= npc_screen_x + npc_width &&
                       mouse_y >= npc_screen_y && mouse_y <= npc_screen_y + npc_height {
                        
                        let current_time = get_time() as f32;
                        let double_click_threshold = 0.5; // 500ms
                        
                        // Check if this is a double-click on the same NPC
                        if let Some(last_id) = &self.last_npc_click_id {
                            if last_id == &life.id && 
                               (current_time - self.last_npc_click_time) < double_click_threshold {
                                // Double-click detected!
                                info!("NPC interaction created: {} (ID: {})", life.name, life.id);
                                // Reset click tracking to prevent triple-clicks from triggering again
                                self.last_npc_click_time = -1.0;
                                self.last_npc_click_id = None;
                                break; // Exit immediately after double-click
                            }
                        }
                        
                        // Update last click info
                        self.last_npc_click_time = current_time;
                        self.last_npc_click_id = Some(life.id.clone());
                        break; // Only handle first NPC clicked
                    }
                }
            }
        }

        // Portal interaction or ladder grab - Check if player is near a portal/ladder and presses Up
        if is_key_pressed(KeyCode::Up) && !free_roam {
            // Find nearby portals (within 40 pixels)
            let nearby_portal = map.portals.iter().find(|portal| {
                let dx = (portal.x - self.player_x as i32).abs();
                let dy = (portal.y - self.player_y as i32).abs();
                dx <= 40 && dy <= 40 && portal.pt != 0 // Not spawn points
            });

            if let Some(portal) = nearby_portal {
                // Portal found - trigger map change
                info!("Player activated portal: '{}' -> map {} portal '{}'",
                      portal.pn, portal.tm, portal.tn);

                // Only teleport if target map is valid (not 999999999)
                if portal.tm != 999999999 {
                    let target_map_id = format!("{:09}", portal.tm);
                    let target_portal_name = portal.tn.clone();
                    info!("Teleporting to map: {} at portal '{}'", target_map_id, target_portal_name);

                    // Set target portal name for spawning in the new map
                    self.target_portal_name = if !target_portal_name.is_empty() {
                        Some(target_portal_name)
                    } else {
                        None // Use spawn portal if no target portal specified
                    };

                    self.loading_new_map = true;
                    self.map_input = target_map_id;
                } else {
                    info!("Portal has no target map (tm = 999999999)");
                }
            } else {
                // No portal activated, try to grab a nearby ladder/rope
                let px = self.player_x as i32;
                let py = self.player_y as i32;

                // Find nearest ladder/rope within horizontal + vertical tolerance
                if let Some(ladder) = map.ladders.iter().find(|lad| {
                    let dx = (lad.x - px).abs();
                    let min_y = lad.y1.min(lad.y2);
                    let max_y = lad.y1.max(lad.y2);
                    dx <= 15 && py >= min_y - 20 && py <= max_y + 20
                }) {
                    // Snap player to ladder X and enter ladder state
                    self.player_x = ladder.x as f32;
                    self.player_vy = 0.0;
                    self.on_ladder = true;
                    self.current_ladder_id = Some(ladder.id);
                    self.on_ground = false;
                    info!("Player grabbed ladder/rope id={} at x={}", ladder.id, ladder.x);
                }
            }
        }

        // Handle vertical movement / physics
        if free_roam {
            // Free roam: no gravity or collision
            self.player_vy = 0.0;
            self.on_ground = false;
            self.on_ladder = false;
        } else if self.on_ladder {
            // Climbing ladder/rope: move with Up/Down, no gravity
            self.player_vy = 0.0;

            // Find the current ladder
            if let Some(ladder) = self.current_ladder_id.and_then(|id| {
                map.ladders.iter().find(|lad| lad.id == id)
            }) {
                let climb_speed = 140.0;

                if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
                    self.player_y -= climb_speed * clamped_dt;
                }
                if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
                    self.player_y += climb_speed * clamped_dt;
                }

                // Clamp within ladder segment
                let min_y = ladder.y1.min(ladder.y2) as f32;
                let max_y = ladder.y1.max(ladder.y2) as f32;
                self.player_y = self.player_y.max(min_y).min(max_y);

                // Jump (Option) to dismount
                if is_key_pressed(KeyCode::LeftAlt) || is_key_pressed(KeyCode::RightAlt) {
                    self.on_ladder = false;
                    self.current_ladder_id = None;
                    self.player_vy = -400.0;
                    self.on_ground = false;
                }

                // Move left/right to step off ladder
                if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A)
                    || is_key_down(KeyCode::Right) || is_key_down(KeyCode::D)
                {
                    self.on_ladder = false;
                    self.current_ladder_id = None;
                }
            } else {
                // Ladder disappeared or not found; exit ladder state
                self.on_ladder = false;
                self.current_ladder_id = None;
            }
        } else {
            // Normal physics with gravity and foothold snapping

            // Drop through platform with Alt/Option + Down
            let down_pressed = is_key_down(KeyCode::Down) || is_key_down(KeyCode::S);
            let jump_pressed = is_key_pressed(KeyCode::LeftAlt) || is_key_pressed(KeyCode::RightAlt);

            if jump_pressed && down_pressed && self.on_ground {
                // Drop through the current platform
                self.drop_through_platform = true;
                self.player_vy = 50.0; // Small downward velocity to start falling
                self.on_ground = false;
                info!("Player dropping through platform");
            } else if jump_pressed && self.on_ground {
                // Normal jump
                self.player_vy = -400.0; // Jump velocity
                self.on_ground = false;
            }

            // Apply gravity
            if flags::ENABLE_COLLISION && !flags::GOD_MODE {
                let gravity = 800.0;
                self.player_vy += gravity * clamped_dt;
            }

            // Update vertical position
            self.player_y += self.player_vy * clamped_dt;

            // Check collision with footholds (only for vertical positioning, not horizontal limits)
            if flags::ENABLE_COLLISION {
                // Skip foothold collision when dropping through platform
                if !self.drop_through_platform {
                    // Try to find foothold at current position
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
                    // While dropping through, keep falling
                    self.on_ground = false;

                    // Reset drop_through flag after falling a bit (30 pixels)
                    // This allows collision with platforms below the original one
                    if self.player_vy > 100.0 {
                        self.drop_through_platform = false;
                    }
                }
            } else {
                // No collision mode - gravity still applies
                self.on_ground = false;
            }
        }

        // No horizontal clamping - footholds naturally define walkable area
        // If you walk off a platform, you fall (and hit bottom boundary eventually)

        // Bottom boundary - if player hits bottom, stop them and set on ground
        let vr_top = map.info.vr_top as f32;
        let vr_bottom = map.info.vr_bottom as f32;
        
        // Bottom boundary - prevent falling through the bottom of the world
        if self.player_y >= vr_bottom {
            info!("Player Y ({}) >= VR_BOTTOM ({}), clamping to floor", self.player_y, vr_bottom);
            self.player_y = vr_bottom;
            self.player_vy = 0.0;
            self.on_ground = true;
        }

        // NOTE: VRTop is removed - it's a CAMERA boundary, not a player boundary
        // Players should be free to jump/climb to any height allowed by footholds/ladders
        // The camera is clamped to VRTop separately (see camera update code below)

        // Update bot AI
        self.bot_ai.update(clamped_dt, map);

        // Update cursor animation
        self.cursor_manager.update(clamped_dt);

        // Update cursor state based on mouse position (check NPC hover)
        let (mouse_x, mouse_y) = mouse_position();
        let world_x = mouse_x + self.camera_x;
        let world_y = mouse_y + self.camera_y;

        let mut cursor_state = CursorState::Default;
        for life in &map.life {
            if life.life_type == "n" && !life.hide {
                // Calculate NPC position (snapped to foothold if available)
                let mut npc_x = life.x as f32;
                let mut npc_y = life.y as f32;

                if life.foothold != 0 {
                    if let Some(fh) = map.footholds.iter().find(|fh| fh.id == life.foothold) {
                        let dx = fh.x2 - fh.x1;
                        let dy = fh.y2 - fh.y1;
                        let ix = npc_x as i32;
                        let fh_y = if dx != 0 {
                            (fh.y1 + ((ix - fh.x1) * dy) / dx) as f32
                        } else {
                            fh.y1 as f32
                        };
                        npc_y = fh_y;
                    }
                }

                // Check if mouse is within NPC bounds
                let npc_width = if let Some(tex) = &life.texture { tex.width() } else { 40.0 };
                let npc_height = if let Some(tex) = &life.texture { tex.height() } else { 60.0 };
                let npc_min_x = npc_x - life.origin_x as f32;
                let npc_max_x = npc_min_x + npc_width;
                let npc_min_y = npc_y - life.origin_y as f32;
                let npc_max_y = npc_min_y + npc_height;

                if world_x >= npc_min_x && world_x <= npc_max_x &&
                   world_y >= npc_min_y && world_y <= npc_max_y {
                    cursor_state = CursorState::NpcHover;
                    break;
                }
            }
        }
        self.cursor_manager.set_state(cursor_state);

        // Camera follows player (unless in camera debug mode)
        if !flags::CAMERA_DEBUG_MODE {
            // Center camera on player
            let target_camera_x = self.player_x - screen_width() / 2.0;
            let target_camera_y = self.player_y - screen_height() / 2.0;

            // Clamp camera X to foothold extent (not just viewport)
            // This allows camera to follow player to actual platform edges
            self.camera_x = target_camera_x
                .max(self.foothold_min_x)
                .min(self.foothold_max_x - screen_width());

            // Camera Y: Only clamp to bottom boundary, not top
            // VRTop is a guideline but camera should follow player everywhere
            let vr_bottom = map.info.vr_bottom as f32;
            let screen_h = screen_height();
            let max_camera_y = vr_bottom - screen_h;

            // Only clamp to bottom - no top constraint so camera can follow player upwards
            self.camera_y = target_camera_y.min(max_camera_y);
        } else{
            // Camera debug mode - move camera independently with arrow keys + Shift
            let camera_speed = 300.0;
            if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
                if is_key_down(KeyCode::Left) {
                    self.camera_x -= camera_speed * clamped_dt;
                }
                if is_key_down(KeyCode::Right) {
                    self.camera_x += camera_speed * clamped_dt;
                }
                if is_key_down(KeyCode::Up) {
                    self.camera_y -= camera_speed * clamped_dt;
                }
                if is_key_down(KeyCode::Down) {
                    self.camera_y += camera_speed * clamped_dt;
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
            self.map_renderer.render(map, self.camera_x, self.camera_y, Some(&self.bot_ai));

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

            // Check if player is near a portal and show indicator
            let nearby_portal = map.portals.iter().find(|portal| {
                let dx = (portal.x - self.player_x as i32).abs();
                let dy = (portal.y - self.player_y as i32).abs();
                dx <= 40 && dy <= 40 && portal.pt != 0 && portal.tm != 999999999
            });

            if nearby_portal.is_some() {
                // Draw "Press ↑ to enter" indicator above player
                let indicator_text = "Press ↑";
                let font_size = 14.0;
                let text_dims = measure_text(indicator_text, None, font_size as u16, 1.0);
                let text_x = player_screen_x - text_dims.width / 2.0;
                let text_y = player_screen_y - 45.0;

                // Draw background box
                let padding = 4.0;
                draw_rectangle(
                    text_x - padding,
                    text_y - text_dims.height - padding,
                    text_dims.width + padding * 2.0,
                    text_dims.height + padding * 2.0,
                    Color::from_rgba(0, 0, 0, 200),
                );

                // Draw text
                draw_text(indicator_text, text_x, text_y, font_size, YELLOW);
            }

            // Render map foregrounds (in front of player)
            self.map_renderer.render_foreground(map, self.camera_x, self.camera_y, Some(&self.bot_ai));
        } else {
            let text = "No map loaded";
            draw_text(text, 20.0, 40.0, 20.0, RED);
        }

        // Draw UI
        self.draw_ui();

        // Draw custom MapleStory cursor (drawn last so it's on top)
        self.cursor_manager.draw();
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
        let mut controls_text = "Controls: A/D or ← → to move | Alt to jump | ↑ on portal to enter".to_string();
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
