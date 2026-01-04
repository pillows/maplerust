use macroquad::prelude::*;
use crate::character::CharacterData;
use crate::flags::{self, DebugFlags};
use crate::map::{MapData, MapLoader, MapRenderer, MobState, MobAI, Foothold};
use crate::map::portal_loader::PortalCache;
use crate::game_world::bot_ai::BotAI;
use crate::audio::AudioManager;
use crate::cursor::{CursorManager, CursorState};
use crate::character_info_ui::StatusBarUI;
use crate::minimap::MiniMap;
use crate::ui_windows::{InventoryWindow, EquipWindow, UserInfoWindow};
use crate::cash_shop::CashShop;
use crate::key_config::KeyConfig;
use crate::chat_balloon::ChatBalloonSystem;
use crate::game_menu::{GameMenu, MenuAction};
use crate::character_renderer::{CharacterRenderer, CharacterState};
use crate::npc_dialog::{NpcDialogSystem, DialogType};
use crate::npc_script::{NpcScriptEngine, NpcScriptCommand};
use crate::social_windows::{ChannelWindow, MegaphoneWindow, MemoWindow, MessengerWindow};
use futures;

/// Gameplay state for when the player is in the game world
pub struct GameplayState {
    character: CharacterData,
    camera_x: f32,
    camera_y: f32,
    player_x: f32,
    player_y: f32,
    player_vy: f32,  // Vertical velocity for gravity
    on_ground: bool,
    facing_right: bool, // Track player facing direction
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
    // Mob damage cooldown
    damage_cooldown: f32,
    loading_new_map: bool,
    backspace_timer: f32,
    backspace_repeat_delay: f32,
    // NPC interaction tracking
    last_npc_click_time: f32,
    last_npc_click_id: Option<String>,
    // Character double-click tracking
    last_player_click_time: f32,
    // Focus tracking
    window_focused: bool,
    last_dt: f32,
    // Cursor manager
    cursor_manager: CursorManager,
    // Status bar UI
    status_bar: StatusBarUI,
    // MiniMap UI
    minimap: MiniMap,
    // UI Windows
    inventory_window: InventoryWindow,
    equip_window: EquipWindow,
    user_info_window: UserInfoWindow,
    // New UI components
    cash_shop: CashShop,
    key_config: KeyConfig,
    chat_balloon: ChatBalloonSystem,
    game_menu: GameMenu,
    character_renderer: CharacterRenderer,
    npc_dialog: NpcDialogSystem,
    npc_script_engine: NpcScriptEngine,
    // Social windows
    channel_window: ChannelWindow,
    megaphone_window: MegaphoneWindow,
    memo_window: MemoWindow,
    messenger_window: MessengerWindow,
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
            facing_right: true,
            loaded: false,
            map_data: None,
            map_renderer: MapRenderer::new(),
            portal_cache: PortalCache::new(),
            current_map_id: "100000000".to_string(), // Default starting map
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
            damage_cooldown: 0.0,
            backspace_timer: 0.0,
            backspace_repeat_delay: 0.05, // Repeat every 50ms when held
            last_npc_click_time: -1.0,
            last_npc_click_id: None,
            last_player_click_time: -1.0,
            window_focused: true,
            last_dt: 0.016, // Default to ~60fps
            cursor_manager: CursorManager::new(),
            status_bar: StatusBarUI::new(),
            minimap: MiniMap::new(),
            inventory_window: InventoryWindow::new(),
            equip_window: EquipWindow::new(),
            user_info_window: UserInfoWindow::new(),
            cash_shop: CashShop::new(),
            key_config: KeyConfig::new(),
            chat_balloon: ChatBalloonSystem::new(),
            game_menu: GameMenu::new(),
            character_renderer: CharacterRenderer::new(),
            npc_dialog: NpcDialogSystem::new(),
            npc_script_engine: NpcScriptEngine::new(),
            channel_window: ChannelWindow::new(),
            megaphone_window: MegaphoneWindow::new(),
            memo_window: MemoWindow::new(),
            messenger_window: MessengerWindow::new(),
        }
    }

    /// Load game assets including the map
    pub async fn load_assets(&mut self) {
        // info!("GameplayState::load_assets() - starting parallel load");

        // Load font, cursors, and status bar in parallel
        let font_load = self.map_renderer.load_font();
        let cursor_load = self.cursor_manager.load_cursors();
        let status_bar_load = self.status_bar.load_assets();
        let minimap_load = self.minimap.load_assets();
        let cash_shop_load = self.cash_shop.load_assets();
        let key_config_load = self.key_config.load_assets();
        let chat_balloon_load = self.chat_balloon.load_assets();
        let game_menu_load = self.game_menu.load_assets();
        let inventory_load = self.inventory_window.load_assets();
        let equip_load = self.equip_window.load_assets();
        let user_info_load = self.user_info_window.load_assets();
        let character_renderer_load = self.character_renderer.load_assets();
        let npc_dialog_load = self.npc_dialog.load_assets();
        let channel_load = self.channel_window.load();
        let megaphone_load = self.megaphone_window.load();
        let memo_load = self.memo_window.load();
        let messenger_load = self.messenger_window.load();

        // info!("Waiting for UI assets to load in parallel...");
        // Wait for all UI assets to load
        let _ = futures::join!(font_load, cursor_load, status_bar_load, minimap_load, 
                               cash_shop_load, key_config_load, chat_balloon_load, game_menu_load,
                               inventory_load, equip_load, user_info_load, character_renderer_load, npc_dialog_load,
                               channel_load, megaphone_load, memo_load, messenger_load);

        // info!("UI assets loaded. Font: ok, Cursors: {}, StatusBar: {}",
        //       self.cursor_manager.is_loaded(),
        //       self.status_bar.is_loaded());

        // Hide OS cursor if custom cursors loaded
        if self.cursor_manager.is_loaded() {
            show_mouse(false);
        } else {
            show_mouse(true);
        }

        // Load map
        self.load_map(&self.current_map_id.clone()).await;
        self.loaded = true;
    }

    /// Load a specific map by ID
    async fn load_map(&mut self, map_id: &str) {
        // Close all UI windows when changing maps
        self.inventory_window.visible = false;
        self.equip_window.visible = false;
        self.user_info_window.visible = false;
        self.key_config.hide();
        self.game_menu.hide();
        self.npc_dialog.close_dialog();
        self.cash_shop.hide();

        match MapLoader::load_map(map_id).await {
            Ok(map) => {

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
                    } else {
                        // Fallback to spawn portal if target not found
                        if let Some(spawn_portal) = map.portals.iter().find(|p| p.pt == 0) {
                            spawn_x = spawn_portal.x as f32;
                            spawn_y = spawn_portal.y as f32;
                        } else {
                            spawn_x = ((map.info.vr_left + map.info.vr_right) / 2) as f32;
                            spawn_y = map.info.vr_top as f32 + 100.0;
                        }
                    }
                } else {
                    // No target portal, use spawn portal (initial map load or debug map change)
                    if let Some(spawn_portal) = map.portals.iter().find(|p| p.pt == 0) {
                        spawn_x = spawn_portal.x as f32;
                        spawn_y = spawn_portal.y as f32;
                    } else {
                        // Default spawn position in center of map
                        spawn_x = ((map.info.vr_left + map.info.vr_right) / 2) as f32;
                        spawn_y = map.info.vr_top as f32 + 100.0;
                    }
                }

                // Find the nearest foothold below the spawn point and place player on it
                // Don't clamp spawn position - use the actual portal/spawn location
                if let Some((foothold_y, _fh)) = map.find_foothold_below(spawn_x, spawn_y) {
                    self.player_x = spawn_x;
                    self.player_y = foothold_y; // Player Y is at feet level (foothold)
                    self.player_vy = 0.0;
                    self.on_ground = true;
                } else {
                    // No foothold found, use spawn position directly
                    self.player_x = spawn_x;
                    self.player_y = spawn_y;
                    self.player_vy = 0.0;
                    self.on_ground = false;
                }

                // Initialize bot AI from map data
                self.bot_ai.initialize_from_map(&map);

                // Save viewport bounds before moving map
                let vr_left = map.info.vr_left as f32;
                let vr_right = map.info.vr_right as f32;
                let vr_bottom = map.info.vr_bottom as f32;

                // Calculate and cache foothold extent
                self.foothold_min_x = vr_left;
                self.foothold_max_x = vr_right;
                for fh in &map.footholds {
                    self.foothold_min_x = self.foothold_min_x.min(fh.x1.min(fh.x2) as f32);
                    self.foothold_max_x = self.foothold_max_x.max(fh.x1.max(fh.x2) as f32);
                }

                // Stop any currently playing BGM before loading new map
                self.audio_manager.stop_bgm();

                // Store BGM name before moving map
                let bgm_name = map.info.bgm.clone();

                self.current_map_id = map_id.to_string();
                
                // Find the lowest foothold Y to ensure platform is visible (before moving map)
                let lowest_foothold_y = map.footholds.iter()
                    .map(|fh| fh.y1.max(fh.y2) as f32)
                    .fold(f32::NEG_INFINITY, f32::max);
                
                // Get VR bounds before moving map
                let vr_top = map.info.vr_top as f32;
                
                self.map_data = Some(map);
                self.loading_new_map = false;

                // Set BGM pending flag for playback in update method
                self.bgm_pending = !bgm_name.is_empty();

                // Initialize camera position - center on player but clamp to boundaries
                let target_camera_x = self.player_x - screen_width() / 2.0;
                let target_camera_y = self.player_y - screen_height() / 2.0;

                // Clamp X to foothold extent
                self.camera_x = target_camera_x
                    .max(self.foothold_min_x)
                    .min(self.foothold_max_x - screen_width());

                // Calculate maximum camera Y to ensure lowest platform is visible
                // Add some margin (100px) to show platform clearly
                let max_camera_y_from_foothold = if lowest_foothold_y.is_finite() {
                    lowest_foothold_y - screen_height() + 100.0
                } else {
                    vr_bottom - screen_height()
                };
                
                // Clamp Y to ensure platform is visible, but don't go above VRTop
                self.camera_y = target_camera_y
                    .max(vr_top)  // Don't go above map top
                    .min(vr_bottom - screen_height())  // Don't go below map bottom
                    .min(max_camera_y_from_foothold);  // Ensure lowest platform is visible

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
    #[inline(never)]
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

        // Handle M key - toggle minimap OR map loader depending on flags
        if is_key_pressed(KeyCode::M) {
            if flags::SHOW_MAP_LOADER {
                // Toggle map loader input
                if self.map_input_active {
                    if !self.map_input.is_empty() {
                        self.loading_new_map = true;
                        self.map_input_active = false;
                        self.target_portal_name = None;
                    } else {
                        self.map_input_active = false;
                        self.map_input.clear();
                    }
                } else {
                    self.map_input_active = true;
                    self.map_input = self.current_map_id.clone();
                }
            } else {
                // Toggle minimap visibility
                self.minimap.toggle();
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

        // Only allow player movement when chat is not focused, menu is not open, and NPC dialog is not open
        let can_move = !self.status_bar.is_chat_focused() && !self.game_menu.is_visible() && !self.npc_dialog.is_visible();

        // Track if movement was blocked by a wall (used to prevent auto-snapping to platforms)
        let mut movement_blocked = false;

        // Debug: Log movement blockers
        if !can_move {
            // Draw debug info on screen
        }

        if can_move {
            // Detect vertical footholds as walls - only block movement INTO them
            let player_half_width = 15.0;
            let player_top = self.player_y - 50.0;
            let player_bottom = self.player_y;
            
            // Find the foothold the player is currently standing on (if any)
            let current_foothold = if self.on_ground {
                map.find_foothold_at(self.player_x, self.player_y)
            } else {
                None
            };
            
            let mut wall_left: Option<f32> = None;
            let mut wall_right: Option<f32> = None;
            
            // Only check for walls if player is trying to move
            let moving_left = is_key_down(KeyCode::Left) || is_key_down(KeyCode::A);
            let moving_right = is_key_down(KeyCode::Right) || is_key_down(KeyCode::D);
            
            if moving_left || moving_right {
                // Calculate where player would be after movement
                let new_x_if_left = if moving_left { self.player_x - move_speed * clamped_dt } else { self.player_x };
                let new_x_if_right = if moving_right { self.player_x + move_speed * clamped_dt } else { self.player_x };
                
                for fh in &map.footholds {
                    // Skip the foothold the player is currently on
                    if let Some(current_fh) = current_foothold {
                        if fh.id == current_fh.id {
                            continue;
                        }
                        
                        // Skip vertical footholds that are connected to the current foothold
                        // These are part of the platform structure, not barriers
                        if fh.prev == current_fh.id || fh.next == current_fh.id ||
                           current_fh.prev == fh.id || current_fh.next == fh.id {
                            continue;
                        }
                    }
                    
                    let dx = (fh.x2 - fh.x1).abs();
                    let dy = (fh.y2 - fh.y1).abs();
                    
                    // Vertical wall: must be very close to vertical (dx < 2px) and have significant vertical extent (dy > 15px)
                    let is_vertical = dx < 2 && dy > 15;
                    
                    if is_vertical {
                        let wall_x = ((fh.x1 + fh.x2) / 2) as f32;
                        let wall_top = fh.y1.min(fh.y2) as f32;
                        let wall_bottom = fh.y1.max(fh.y2) as f32;
                        
                        // Check if wall is in player's vertical range
                        if player_top < wall_bottom && player_bottom > wall_top {
                            // Wall is to the left - only block if moving left and would cross it
                            if wall_x < self.player_x && moving_left {
                                // Player's left edge at new position
                                let new_left_edge = new_x_if_left - player_half_width;
                                // Player's left edge at current position
                                let current_left_edge = self.player_x - player_half_width;
                                // Only block if wall is between current and new position
                                if wall_x >= new_left_edge && wall_x < current_left_edge {
                                    wall_left = Some(wall_left.map_or(wall_x, |l| l.max(wall_x)));
                                }
                            }
                            // Wall is to the right - only block if moving right and would cross it
                            else if wall_x > self.player_x && moving_right {
                                // Player's right edge at new position
                                let new_right_edge = new_x_if_right + player_half_width;
                                // Player's right edge at current position
                                let current_right_edge = self.player_x + player_half_width;
                                // Only block if wall is between current and new position
                                if wall_x <= new_right_edge && wall_x > current_right_edge {
                                    wall_right = Some(wall_right.map_or(wall_x, |r| r.min(wall_x)));
                                }
                            }
                        }
                    }
                }
            }
            
            // Apply horizontal movement - only block if a wall is detected
            if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
                let new_x = self.player_x - move_speed * clamped_dt;
                // Only block if there's a wall on the left
                // Account for player width (half width on each side)
                let player_half_width = 15.0;
                if let Some(wall_x) = wall_left {
                    // Block movement if player would cross the wall
                    // Allow player to get close but not pass through
                    let blocked_x = wall_x + player_half_width;
                    if new_x < blocked_x {
                        self.player_x = blocked_x;
                        movement_blocked = true;
                    } else {
                        self.player_x = new_x;
                    }
                } else {
                    // No wall detected - allow normal movement
                    // Still respect map boundaries as a safety measure
                    self.player_x = new_x.max(self.foothold_min_x);
                }
            }
            if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
                let new_x = self.player_x + move_speed * clamped_dt;
                // Only block if there's a wall on the right
                // Account for player width (half width on each side)
                let player_half_width = 15.0;
                if let Some(wall_x) = wall_right {
                    // Block movement if player would cross the wall
                    // Allow player to get close but not pass through
                    let blocked_x = wall_x - player_half_width;
                    if new_x > blocked_x {
                        self.player_x = blocked_x;
                        movement_blocked = true;
                    } else {
                        self.player_x = new_x;
                    }
                } else {
                    // No wall detected - allow normal movement
                    // Still respect map boundaries as a safety measure
                    self.player_x = new_x.min(self.foothold_max_x);
                }
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
        }
        
        // Handle player double-click (show UserInfo window)
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mouse_x, mouse_y) = mouse_position();
            let player_screen_x = self.player_x - self.camera_x;
            let player_screen_y = self.player_y - self.camera_y;
            
            // Check if click is on the player character (hitbox around feet position)
            let player_half_width = 20.0;
            let player_height = 50.0;
            if mouse_x >= player_screen_x - player_half_width && mouse_x <= player_screen_x + player_half_width &&
               mouse_y >= player_screen_y - player_height && mouse_y <= player_screen_y + 10.0 {
                
                let current_time = get_time() as f32;
                let double_click_threshold = 0.5; // 500ms
                
                if (current_time - self.last_player_click_time) < double_click_threshold {
                    // Double-click detected! Show UserInfo window
                    info!("Player double-clicked, showing UserInfo window");
                    self.user_info_window.show();
                    self.last_player_click_time = -1.0; // Reset to prevent triple-click
                } else {
                    self.last_player_click_time = current_time;
                }
            }
        }
        
        // Handle NPC double-click interaction
        // Extract NPC interaction data first to avoid borrow conflicts
        let npc_interaction_data = if is_mouse_button_pressed(MouseButton::Left) {
            let (mouse_x, mouse_y) = mouse_position();
            let world_x = mouse_x + self.camera_x;
            let world_y = mouse_y + self.camera_y;

            // Check if click is on an NPC
            let mut npc_data: Option<(i32, String, Option<Texture2D>)> = None;
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
                                // Double-click detected! Save NPC data
                                info!("NPC interaction created: {} (ID: {})", life.name, life.id);
                                let npc_id = life.id.parse::<i32>().unwrap_or(0);
                                npc_data = Some((npc_id, life.name.clone(), life.texture.clone()));
                                break;
                            }
                        }

                        // Update last click info
                        self.last_npc_click_time = current_time;
                        self.last_npc_click_id = Some(life.id.clone());
                        break; // Only handle first NPC clicked
                    }
                }
            }
            npc_data
        } else {
            None
        };

        // Store NPC interaction to execute after map borrow is released
        let pending_npc_command = if let Some((npc_id, npc_name, npc_texture)) = npc_interaction_data {
            let cmd = self.npc_script_engine.start_npc(npc_id);
            self.last_npc_click_time = -1.0;
            self.last_npc_click_id = None;
            Some((cmd, npc_name, npc_texture))
        } else {
            None
        };

        // Portal interaction or ladder grab - Check if player is near a portal/ladder and presses Up
        if can_move && is_key_pressed(KeyCode::Up) && !free_roam {
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
                    // info!("Portal has no target map (tm = 999999999)");
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

        // Also check for grabbing ladder with Down key (when standing on top of a ladder)
        if can_move && !self.on_ladder && self.on_ground && (is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S)) && !free_roam {
            let px = self.player_x as i32;
            let py = self.player_y as i32;

            // Find ladder that starts near player's feet (player is standing on top of it)
            if let Some(ladder) = map.ladders.iter().find(|lad| {
                let dx = (lad.x - px).abs();
                let top_y = lad.y1.min(lad.y2);
                // Player's feet are at py + 30, ladder top should be close
                dx <= 15 && (py + 30 - top_y).abs() <= 20
            }) {
                // Snap player to ladder X and enter ladder state
                self.player_x = ladder.x as f32;
                self.player_vy = 0.0;
                self.on_ladder = true;
                self.current_ladder_id = Some(ladder.id);
                self.on_ground = false;
                info!("Player grabbed ladder/rope from top id={} at x={}", ladder.id, ladder.x);
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
                let min_y = ladder.y1.min(ladder.y2) as f32;
                let max_y = ladder.y1.max(ladder.y2) as f32;

                if can_move {
                    if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
                        self.player_y -= climb_speed * clamped_dt;
                        
                        // Exit at top of ladder
                        if self.player_y <= min_y {
                            self.player_y = min_y - 30.0; // Place player above ladder
                            self.on_ladder = false;
                            self.current_ladder_id = None;
                            self.on_ground = false; // Will snap to foothold on next frame
                            info!("Player exited ladder at top");
                        }
                    }
                    if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
                        self.player_y += climb_speed * clamped_dt;
                        
                        // Exit at bottom of ladder
                        if self.player_y >= max_y {
                            self.player_y = max_y;
                            self.on_ladder = false;
                            self.current_ladder_id = None;
                            self.on_ground = false; // Will snap to foothold on next frame
                            info!("Player exited ladder at bottom");
                        }
                    }

                    // Clamp within ladder segment while climbing
                    if self.on_ladder {
                        self.player_y = self.player_y.max(min_y).min(max_y);
                    }

                    // Jump (Alt) to dismount - use is_key_pressed to only trigger once
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
                }
            } else {
                // Ladder disappeared or not found; exit ladder state
                self.on_ladder = false;
                self.current_ladder_id = None;
            }
        } else {
            // Normal physics with gravity and foothold snapping

            if can_move {
                // Drop through platform with Alt/Option + Down
                // Use is_key_down for Alt to allow holding it
                let down_pressed = is_key_down(KeyCode::Down) || is_key_down(KeyCode::S);
                let alt_pressed = is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt);

                if alt_pressed && down_pressed && self.on_ground {
                    // Check if there's a platform below before allowing drop-through
                    // Find current foothold (player_y is at feet/foothold level)
                    if let Some(current_fh) = map.find_foothold_at(self.player_x, self.player_y) {
                        let current_fh_y = {
                            let dx = current_fh.x2 - current_fh.x1;
                            let dy = current_fh.y2 - current_fh.y1;
                            let ix = self.player_x as i32;
                            if dx != 0 {
                                (current_fh.y1 + ((ix - current_fh.x1) * dy) / dx) as f32
                            } else {
                                current_fh.y1 as f32
                            }
                        };
                        
                        // Look for a platform strictly below the current one (at least 15 pixels below)
                        if let Some((below_y, _)) = map.find_foothold_strictly_below(self.player_x, current_fh_y, 15.0) {
                            // Drop through to the platform below
                            self.drop_through_platform = true;
                            self.player_vy = 100.0; // Downward velocity to start falling
                            self.on_ground = false;
                            info!("Player dropping through platform from y={} to y={}", current_fh_y, below_y);
                        }
                    }
                } else if alt_pressed && self.on_ground && !down_pressed {
                    // Normal jump (Alt without Down)
                    self.player_vy = -400.0; // Jump velocity
                    self.on_ground = false;
                }
            }

            // Apply gravity
            if flags::ENABLE_COLLISION && !flags::GOD_MODE {
                let gravity = 800.0;
                self.player_vy += gravity * clamped_dt;
            }

            // Update vertical position
            self.player_y += self.player_vy * clamped_dt;

            // Check collision with footholds (only for vertical positioning, not horizontal limits)
            // Don't auto-snap to platforms when movement is blocked by a wall
            if flags::ENABLE_COLLISION {
                if !self.drop_through_platform {
                    // Simple collision: find foothold at position or below
                    // Only snap to footholds at the player's current X position to prevent teleportation
                    if let Some(fh) = map.find_foothold_at(self.player_x, self.player_y) {
                        let fh_y = map.get_foothold_y_at(fh, self.player_x);
                        if self.player_y >= fh_y && self.player_vy >= 0.0 {
                            self.player_y = fh_y;
                            self.player_vy = 0.0;
                            self.on_ground = true;
                        } else {
                            self.on_ground = false;
                        }
                    } else if self.player_vy >= 0.0 && !movement_blocked {
                        // Only check below if player is falling AND not blocked by a wall
                        // This prevents auto-teleportation when hitting a wall
                        if let Some((fh_y, _)) = map.find_foothold_below(self.player_x, self.player_y) {
                            // Only snap if player is actually falling and close to the foothold
                            // Don't snap if player is far above (prevents teleportation)
                            if self.player_y >= fh_y - 5.0 {
                                self.player_y = fh_y;
                                self.player_vy = 0.0;
                                self.on_ground = true;
                            } else {
                                self.on_ground = false;
                            }
                        } else {
                            self.on_ground = false;
                        }
                    } else {
                        // Player is jumping/rising or blocked by wall - don't snap to platforms
                        self.on_ground = false;
                    }
                } else {
                    self.on_ground = false;
                    if self.player_vy > 100.0 {
                        self.drop_through_platform = false;
                    }
                }
            } else {
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

        // Update damage cooldown
        if self.damage_cooldown > 0.0 {
            self.damage_cooldown -= clamped_dt;
        }

        // Check mob collision for damage
        if self.damage_cooldown <= 0.0 {
            for mob in self.bot_ai.get_mobs() {
                let mob_half_width = 20.0;
                let mob_height = 40.0;
                let player_half_width = 15.0;
                let player_height = 45.0;
                
                // Simple AABB collision
                let mob_left = mob.x - mob_half_width;
                let mob_right = mob.x + mob_half_width;
                let mob_top = mob.y - mob_height;
                let mob_bottom = mob.y;
                
                let player_left = self.player_x - player_half_width;
                let player_right = self.player_x + player_half_width;
                let player_top = self.player_y - player_height;
                let player_bottom = self.player_y;
                
                if player_right > mob_left && player_left < mob_right &&
                   player_bottom > mob_top && player_top < mob_bottom {
                    // Collision! Take 1 damage
                    if self.character.hp > 0 {
                        self.character.hp = self.character.hp.saturating_sub(1);
                        self.damage_cooldown = 1.0; // 1 second invincibility
                        info!("Player hit by mob! HP: {}", self.character.hp);
                    }
                    break;
                }
            }
        }

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
            // Account for status bar UI at bottom (approximately 70px)
            let ui_bottom_margin = 70.0;
            let max_camera_y = vr_bottom - screen_h + ui_bottom_margin;

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

        // Update status bar UI
        self.status_bar.update(clamped_dt, &self.character);

        // Check for sent chat message and show balloon with player name
        if let Some(message) = self.status_bar.take_last_sent_message() {
            self.chat_balloon.show_player_chat_with_name(&self.character.name, &message, self.player_x, self.player_y);
        }

        // Update chat balloon player position
        self.chat_balloon.update_player_position(self.player_x, self.player_y);

        // Update minimap
        self.minimap.update();

        // Update UI windows
        self.inventory_window.update();
        self.equip_window.update();
        self.user_info_window.update();

        // Update new UI components
        self.cash_shop.update();
        self.key_config.update();
        self.chat_balloon.update(clamped_dt);
        self.game_menu.update();
        self.npc_dialog.update();

        // Handle NPC dialog responses
        use crate::npc_dialog::DialogResponse;
        let response = self.npc_dialog.take_response();
        if response != DialogResponse::None {
            // Get current NPC info from last clicked NPC
            let (npc_name, npc_texture) = if let Some(map) = &self.map_data {
                if let Some(npc_id) = &self.last_npc_click_id {
                    if let Some(life) = map.life.iter().find(|l| &l.id == npc_id && l.life_type == "n") {
                        (life.name.clone(), life.texture.clone())
                    } else {
                        (String::new(), None)
                    }
                } else {
                    (String::new(), None)
                }
            } else {
                (String::new(), None)
            };

            let cmd = self.npc_script_engine.handle_response(response);
            self.execute_script_command_with_npc(cmd, npc_name, npc_texture);
        }

        // Update social windows
        self.channel_window.update();
        self.megaphone_window.update();
        self.memo_window.update();
        self.messenger_window.update();

        // Update character renderer
        let character_state = if self.on_ladder {
            CharacterState::Stand  // Standing on ladder
        } else if !self.on_ground && self.player_vy < 0.0 {
            CharacterState::Jump
        } else if !self.on_ground && self.player_vy > 0.0 {
            CharacterState::Fall
        } else {
            if can_move && (is_key_down(KeyCode::Left) || is_key_down(KeyCode::Right) || 
                            is_key_down(KeyCode::A) || is_key_down(KeyCode::D)) {
                CharacterState::Move
            } else {
                CharacterState::Stand
            }
        };
        
        // Update facing direction only when actively pressing a direction key
        if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            self.facing_right = false;
        } else if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            self.facing_right = true;
        }
        self.character_renderer.update(clamped_dt, character_state, self.facing_right);

        // Handle game menu actions
        match self.game_menu.take_action() {
            MenuAction::Inventory => self.inventory_window.toggle(),
            MenuAction::Equip => self.equip_window.toggle(),
            MenuAction::KeyConfig => self.key_config.toggle(),
            MenuAction::Channel => self.channel_window.toggle(),
            MenuAction::Messenger => self.messenger_window.toggle(),
            MenuAction::Quit => {
                // TODO: Implement quit confirmation
                info!("Quit requested from menu");
            }
            _ => {}
        }

        // ESC key - close active UI windows (priority order)
        if is_key_pressed(KeyCode::Escape) {
            if self.cash_shop.is_visible() {
                self.cash_shop.hide();
            } else if self.npc_dialog.is_visible() {
                self.npc_dialog.close_dialog();
            } else if self.game_menu.is_visible() {
                self.game_menu.hide();
            } else if self.megaphone_window.is_visible() {
                self.megaphone_window.hide();
            } else if self.channel_window.is_visible() {
                self.channel_window.hide();
            } else if self.messenger_window.is_visible() {
                self.messenger_window.hide();
            } else if self.memo_window.is_visible() {
                self.memo_window.hide();
            } else if self.inventory_window.visible {
                self.inventory_window.visible = false;
            } else if self.equip_window.visible {
                self.equip_window.visible = false;
            } else if self.key_config.is_visible() {
                self.key_config.hide();
            } else if self.user_info_window.visible {
                self.user_info_window.visible = false;
            }
        }

        // Handle keyboard shortcuts for UI windows (only when chat is not focused and cash shop is not open)
        if !self.status_bar.is_chat_focused() && !self.cash_shop.is_visible() {
            // I key - toggle inventory
            if is_key_pressed(KeyCode::I) {
                self.inventory_window.toggle();
            }
            // E key - toggle equipment
            if is_key_pressed(KeyCode::E) {
                self.equip_window.toggle();
            }
            // K key - toggle key config
            if is_key_pressed(KeyCode::K) {
                self.key_config.toggle();
            }
            // O key - toggle messenger
            if is_key_pressed(KeyCode::O) {
                self.messenger_window.toggle();
            }
            // T key - toggle megaphone
            if is_key_pressed(KeyCode::T) {
                self.megaphone_window.toggle();
            }
        }

        // Handle status bar button clicks
        if self.status_bar.bt_cashshop_clicked() {
            self.cash_shop.show();
        }
        if self.status_bar.bt_keysetting_clicked() {
            self.key_config.toggle();
        }
        if self.status_bar.bt_menu_clicked() {
            // Get menu button center-top position and show menu above it
            let (btn_x, btn_y) = self.status_bar.get_menu_button_pos();
            self.game_menu.toggle_at(btn_x, btn_y);
        }
        if self.status_bar.bt_channel_clicked() {
            self.channel_window.toggle();
        }

        // Execute pending NPC command (after all map-dependent code)
        if let Some((cmd, npc_name, npc_texture)) = pending_npc_command {
            self.execute_script_command_with_npc(cmd, npc_name, npc_texture);
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
            self.map_renderer.render(map, self.camera_x, self.camera_y, Some(&self.bot_ai), Some(&self.character_renderer));

            // Draw player using character renderer
            let player_screen_x = self.player_x - self.camera_x;
            let player_screen_y = self.player_y - self.camera_y;
            
            // Determine character state for rendering (same logic as update)
            let character_state = if self.on_ladder {
                CharacterState::Stand
            } else if !self.on_ground && self.player_vy < 0.0 {
                CharacterState::Jump
            } else if !self.on_ground && self.player_vy > 0.0 {
                CharacterState::Fall
            } else {
                let can_move = !self.status_bar.is_chat_focused() && !self.game_menu.is_visible();
                if can_move && (is_key_down(KeyCode::Left) || is_key_down(KeyCode::Right) || 
                                is_key_down(KeyCode::A) || is_key_down(KeyCode::D)) {
                    CharacterState::Move
                } else {
                    CharacterState::Stand
                }
            };
            
            self.character_renderer.draw(player_screen_x, player_screen_y, character_state);

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
            
            // Draw chat balloons (above NPCs/mobs)
            self.chat_balloon.draw(self.camera_x, self.camera_y);
            
            // Draw NPC dialog window (on top of everything)
            self.npc_dialog.draw(self.camera_x, self.camera_y);
        } else {
            let text = "No map loaded";
            draw_text(text, 20.0, 40.0, 20.0, RED);
        }

        // Draw UI
        self.draw_ui();

        // Draw status bar UI
        self.status_bar.draw(&self.character);

        // Draw minimap
        if let Some(map) = &self.map_data {
            self.minimap.draw(self.player_x, self.player_y, map, self.camera_x, self.camera_y);
        }

        // Draw UI windows
        self.inventory_window.draw();
        self.equip_window.draw();
        self.user_info_window.draw(&self.character.name, self.character.level);

        // Draw new UI windows
        self.key_config.draw();
        self.game_menu.draw();
        
        // Draw social windows
        self.channel_window.draw();
        self.megaphone_window.draw();
        self.memo_window.draw();
        self.messenger_window.draw();

        // Draw CashShop (full screen overlay, drawn on top of everything except cursor)
        self.cash_shop.draw();

        // Always show player coordinates at top-left for debugging
        let coords_text = format!("X: {:.0}  Y: {:.0}  Ground: {}", self.player_x, self.player_y, self.on_ground);
        draw_rectangle(5.0, 5.0, 200.0, 20.0, Color::from_rgba(0, 0, 0, 180));
        draw_text(&coords_text, 10.0, 20.0, 14.0, YELLOW);

        // Draw custom MapleStory cursor (drawn last so it's on top)
        self.cursor_manager.draw();
    }

    /// Draw the game UI
    fn draw_ui(&self) {
        // Draw FPS counter in top-right corner (always visible if SHOW_FPS is enabled)
        if flags::SHOW_FPS {
            let fps = get_fps();
            let fps_text = format!("FPS: {}", fps);
            let text_width = measure_text(&fps_text, None, 14, 1.0).width;
            draw_text(&fps_text, screen_width() - text_width - 10.0, 20.0, 14.0, YELLOW);
        }

        // Only show debug panel if debug UI is enabled
        if DebugFlags::should_show_debug_ui() {
            let panel_x = 10.0;
            let panel_y = 10.0;
            let panel_width = 200.0;
            let panel_height = 200.0;

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
            let mut y_offset = panel_y + 105.0;
            let line_height = 20.0;

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

            let ground_text = format!("On Ground: {} | VY: {:.1}", self.on_ground, self.player_vy);
            draw_text(&ground_text, panel_x + 10.0, y_offset, 14.0, YELLOW);
            y_offset += line_height;

            // Show current foothold info
            if let Some(ref map) = self.map_data {
                if let Some(fh) = map.find_foothold_at(self.player_x, self.player_y) {
                    let fh_y = map.get_foothold_y_at(fh, self.player_x);
                    let fh_text = format!("FH: id={} y={:.0}", fh.id, fh_y);
                    draw_text(&fh_text, panel_x + 10.0, y_offset, 14.0, GREEN);
                } else {
                    draw_text("FH: None", panel_x + 10.0, y_offset, 14.0, RED);
                }
                y_offset += line_height;
            }

            let cam_text = format!("Cam: ({:.0}, {:.0})", self.camera_x, self.camera_y);
            draw_text(&cam_text, panel_x + 10.0, y_offset, 14.0, YELLOW);
            y_offset += line_height;

            // Show VR bounds
            if let Some(ref map) = self.map_data {
                let vr_text = format!("VR: L={} R={}", map.info.vr_left, map.info.vr_right);
                draw_text(&vr_text, panel_x + 10.0, y_offset, 14.0, GRAY);
                y_offset += line_height;
            }

            if flags::GOD_MODE {
                draw_text("GOD MODE", panel_x + 10.0, y_offset, 14.0, RED);
                y_offset += line_height;
            }

            // Movement debug
            let chat_focused = self.status_bar.is_chat_focused();
            let menu_visible = self.game_menu.is_visible();
            let npc_visible = self.npc_dialog.is_visible();
            let can_move = !chat_focused && !menu_visible && !npc_visible;
            let move_color = if can_move { GREEN } else { RED };
            let move_text = format!("Move: {} (chat:{} menu:{} npc:{})", 
                can_move, chat_focused, menu_visible, npc_visible);
            draw_text(&move_text, panel_x + 10.0, y_offset, 14.0, move_color);
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

        // Debug map loader UI - always show if SHOW_MAP_LOADER is enabled
        if flags::SHOW_MAP_LOADER {
            self.draw_map_loader_ui();
        }
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

    /// Execute NPC script command with NPC info
    fn execute_script_command_with_npc(&mut self, cmd: NpcScriptCommand, npc_name: String, npc_texture: Option<Texture2D>) {
        match cmd {
            NpcScriptCommand::ShowDialog { text, dialog_type } => {
                self.npc_dialog.show_dialog_typed(
                    &text,
                    &npc_name,
                    npc_texture,
                    dialog_type
                );
            }
            NpcScriptCommand::ShowSelection { text, options } => {
                self.npc_dialog.show_selection(
                    &text,
                    &npc_name,
                    npc_texture,
                    options
                );
            }
            NpcScriptCommand::Close => {
                self.npc_dialog.close_dialog();
            }
            NpcScriptCommand::GiveItem(id, qty) => {
                info!("Script: Give item {} x{}", id, qty);
                // TODO: Implement inventory system interaction
                self.npc_dialog.close_dialog();
            }
            NpcScriptCommand::GiveMeso(amount) => {
                info!("Script: Give {} meso", amount);
                // TODO: Implement meso addition
                self.npc_dialog.close_dialog();
            }
            NpcScriptCommand::GiveExp(amount) => {
                info!("Script: Give {} exp", amount);
                // TODO: Implement exp addition
                self.npc_dialog.close_dialog();
            }
            NpcScriptCommand::TakeItem(id, qty) => {
                info!("Script: Take item {} x{}", id, qty);
                // TODO: Implement inventory system interaction
                self.npc_dialog.close_dialog();
            }
            NpcScriptCommand::Warp(map_id) => {
                info!("Script: Warp to map {}", map_id);
                // TODO: Implement map warp
                self.npc_dialog.close_dialog();
            }
            NpcScriptCommand::ShowStyle { text, style_type, available_styles } => {
                info!("Script: Show style dialog {:?} with {} options", style_type, available_styles.len());
                // TODO: Implement style dialog in Phase 4
                self.npc_dialog.close_dialog();
            }
            NpcScriptCommand::None => {}
        }
    }
}
