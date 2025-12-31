use macroquad::prelude::*;
use crate::assets::AssetManager;
use crate::character::CharacterData;
use crate::cursor::CursorManager;
use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzReader, WzObjectType, WzNodeCast};

#[cfg(not(target_arch = "wasm32"))]
use memmap2::MmapOptions;

const LOGIN_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/UI/Login.img";
const LOGIN_CACHE_NAME: &str = "/01/UI/Login.img";
const BACKGROUND_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/Map/Back/login.img";
const BACKGROUND_CACHE_NAME: &str = "/01/Map/Back/login.img";

/// Structure to hold texture with its origin point
#[derive(Clone)]
struct TextureWithOrigin {
    texture: Texture2D,
    origin: Vec2,
}

/// Button state for UI interactions
#[derive(PartialEq, Clone, Copy)]
enum ButtonState {
    Normal,
    MouseOver,
    Pressed,
    Disabled,
}

/// Represents a clickable button with textures for different states
struct Button {
    normal: Option<TextureWithOrigin>,
    mouse_over: Option<TextureWithOrigin>,
    pressed: Option<TextureWithOrigin>,
    disabled: Option<TextureWithOrigin>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    state: ButtonState,
}

impl Button {
    fn new(x: f32, y: f32) -> Self {
        Self {
            normal: None,
            mouse_over: None,
            pressed: None,
            disabled: None,
            x,
            y,
            width: 0.0,
            height: 0.0,
            state: ButtonState::Normal,
        }
    }

    fn update(&mut self) {
        if self.state == ButtonState::Disabled {
            return;
        }

        let (mouse_x, mouse_y) = mouse_position();

        let (draw_x, draw_y) = if let Some(tex) = &self.normal {
            (self.x - tex.origin.x, self.y - tex.origin.y)
        } else {
            (self.x, self.y)
        };

        let mouse_in_bounds = mouse_x >= draw_x
            && mouse_x <= draw_x + self.width
            && mouse_y >= draw_y
            && mouse_y <= draw_y + self.height;

        if mouse_in_bounds {
            if is_mouse_button_down(MouseButton::Left) {
                self.state = ButtonState::Pressed;
            } else {
                self.state = ButtonState::MouseOver;
            }
        } else {
            self.state = ButtonState::Normal;
        }
    }

    fn is_clicked(&self) -> bool {
        if self.state == ButtonState::Disabled {
            return false;
        }

        let (mouse_x, mouse_y) = mouse_position();

        let (draw_x, draw_y) = if let Some(tex) = &self.normal {
            (self.x - tex.origin.x, self.y - tex.origin.y)
        } else {
            (self.x, self.y)
        };

        let mouse_in_bounds = mouse_x >= draw_x
            && mouse_x <= draw_x + self.width
            && mouse_y >= draw_y
            && mouse_y <= draw_y + self.height;

        mouse_in_bounds && is_mouse_button_released(MouseButton::Left)
    }

    fn draw(&self) {
        let tex_with_origin = match self.state {
            ButtonState::Normal => &self.normal,
            ButtonState::MouseOver => &self.mouse_over,
            ButtonState::Pressed => &self.pressed,
            ButtonState::Disabled => &self.disabled,
        };

        if let Some(two) = tex_with_origin {
            let draw_x = self.x - two.origin.x;
            let draw_y = self.y - two.origin.y;
            draw_texture(&two.texture, draw_x, draw_y, WHITE);
        }
    }
}

/// Load a single PNG texture with origin from an already-parsed WZ node
fn load_png_from_node(root_node: &WzNodeArc, path: &str) -> Result<TextureWithOrigin, String> {
    let node = root_node
        .read()
        .unwrap()
        .at_path_parsed(path)
        .map_err(|e| format!("Failed to navigate to '{}': {:?}", path, e))?;

    let texture = {
        let node_read = node.read().unwrap();
        match &node_read.object_type {
            WzObjectType::Property(wz_reader::property::WzSubProperty::PNG(png_data)) => {
                let dynamic_img = png_data
                    .extract_png()
                    .map_err(|e| format!("Failed to extract PNG: {:?}", e))?;
                let rgba_img = dynamic_img.to_rgba8();
                let width = rgba_img.width() as u16;
                let height = rgba_img.height() as u16;
                let bytes = rgba_img.into_raw();
                Texture2D::from_rgba8(width, height, &bytes)
            }
            _ => return Err(format!("Node at path '{}' is not a PNG", path)),
        }
    };

    let origin_path = format!("{}/origin", path);
    let origin = {
        let root_read = root_node.read().unwrap();
        root_read
            .at_path_parsed(&origin_path)
            .ok()
            .and_then(|origin_node| {
                origin_node
                    .read()
                    .unwrap()
                    .try_as_vector2d()
                    .map(|vec| vec2(vec.0 as f32, vec.1 as f32))
            })
            .unwrap_or(vec2(0.0, 0.0))
    };

    Ok(TextureWithOrigin { texture, origin })
}

/// Character selection state
pub struct CharacterSelectionState {
    background_sky: Option<TextureWithOrigin>,
    background_scene: Option<TextureWithOrigin>,
    select_button: Button,
    delete_button: Button,
    new_button: Button,
    page_left_button: Button,
    page_right_button: Button,
    char_info_panels: Vec<TextureWithOrigin>,
    loaded: bool,

    // Character data
    characters: Vec<CharacterData>,
    selected_character_index: Option<usize>,

    // Transition state
    should_transition_to_char_create: bool,
    should_transition_to_game: bool,

    // Transition animation
    transition_alpha: f32,
    transition_duration: f32,
    transition_time: f32,
    is_transitioning_in: bool,

    // Cursor manager
    cursor_manager: CursorManager,
    
    // Double-click tracking
    last_click_time: f32,
    last_click_index: Option<usize>,
}

impl CharacterSelectionState {
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

    pub fn new() -> Self {
        Self {
            background_sky: None,
            background_scene: None,
            select_button: Button::new(400.0, 300.0),
            delete_button: Button::new(400.0, 300.0),
            new_button: Button::new(400.0, 300.0),
            page_left_button: Button::new(400.0, 300.0),
            page_right_button: Button::new(400.0, 300.0),
            char_info_panels: Vec::new(),
            loaded: false,
            characters: CharacterData::load_all(),
            selected_character_index: None,
            should_transition_to_char_create: false,
            should_transition_to_game: false,
            transition_alpha: 0.0,
            transition_duration: 0.5,
            transition_time: 0.0,
            is_transitioning_in: true,
            cursor_manager: CursorManager::new(),
            last_click_time: -1.0,
            last_click_index: None,
        }
    }

    /// Reload character data from disk
    pub fn reload_characters(&mut self) {
        self.characters = CharacterData::load_all();
        info!("Loaded {} characters", self.characters.len());
    }

    /// Check if should transition to character creation screen
    pub fn should_transition_to_char_create(&self) -> bool {
        self.should_transition_to_char_create
    }

    /// Check if should transition to game screen
    pub fn should_transition_to_game(&self) -> bool {
        self.should_transition_to_game
    }

    /// Get the selected character
    pub fn get_selected_character(&self) -> Option<&CharacterData> {
        self.selected_character_index
            .and_then(|idx| self.characters.get(idx))
    }

    /// Load all character selection screen assets from Login.img
    pub async fn load_assets(&mut self) {
        info!("Loading character selection screen assets...");

        // Load cursor
        self.cursor_manager.load_cursors().await;
        if self.cursor_manager.is_loaded() {
            show_mouse(false);
        }

        let bytes = match AssetManager::fetch_and_cache(LOGIN_URL, LOGIN_CACHE_NAME).await {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Failed to fetch Login.img: {}", e);
                return;
            }
        };

        info!("Parsing Login.img (size: {} bytes)...", bytes.len());

        let wz_iv = match guess_iv_from_wz_img(&bytes) {
            Some(iv) => iv,
            None => {
                error!("Unable to guess version from Login.img");
                return;
            }
        };

        let byte_len = bytes.len();
        let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
        let wz_image = WzImage::new(&LOGIN_CACHE_NAME.into(), 0, byte_len, &reader);
        let root_node: WzNodeArc = WzNode::new(&LOGIN_CACHE_NAME.into(), wz_image, None).into();

        if let Err(e) = root_node.write().unwrap().parse(&root_node) {
            error!("Failed to parse WZ root node: {:?}", e);
            return;
        }

        info!("WZ file parsed successfully");

        // Load background from login.img (Map/Back/login.img)
        info!("Loading background assets...");
        let bg_bytes = match AssetManager::fetch_and_cache(BACKGROUND_URL, BACKGROUND_CACHE_NAME).await {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Failed to fetch background: {}", e);
                Vec::new()
            }
        };

        if !bg_bytes.is_empty() {
            info!("Parsing background (size: {} bytes)...", bg_bytes.len());

            let bg_wz_iv = match guess_iv_from_wz_img(&bg_bytes) {
                Some(iv) => iv,
                None => {
                    error!("Unable to guess version from background IMG file");
                    [0; 4]
                }
            };

            let bg_byte_len = bg_bytes.len();
            let bg_reader = Arc::new(WzReader::from_buff(&bg_bytes).with_iv(bg_wz_iv));
            let bg_wz_image = WzImage::new(&BACKGROUND_CACHE_NAME.into(), 0, bg_byte_len, &bg_reader);
            let bg_root_node: WzNodeArc = WzNode::new(&BACKGROUND_CACHE_NAME.into(), bg_wz_image, None).into();

            // Parse the background node
            if bg_root_node.write().unwrap().parse(&bg_root_node).is_ok() {
                // Load sky gradient (back/1) - sits behind everything
                match load_png_from_node(&bg_root_node, "back/1") {
                    Ok(two) => {
                        info!("Sky gradient loaded: {}x{}, origin: ({}, {})",
                            two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                        self.background_sky = Some(two);
                    }
                    Err(e) => error!("Failed to load sky gradient: {}", e),
                }

                // Load scene background (back/13) - sits on top of sky
                match load_png_from_node(&bg_root_node, "back/13") {
                    Ok(two) => {
                        info!("Scene background loaded: {}x{}, origin: ({}, {})",
                            two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                        self.background_scene = Some(two);
                    }
                    Err(e) => error!("Failed to load scene background: {}", e),
                }
            }
        }

        // Load character info panels
        for panel_id in &[1, 3] {
            match load_png_from_node(&root_node, &format!("CharSelect/charInfo{}", panel_id)) {
                Ok(two) => {
                    info!("Character info panel {} loaded: {}x{}, origin: ({}, {})",
                        panel_id, two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                    self.char_info_panels.push(two);
                }
                Err(e) => error!("Failed to load character info panel {}: {}", panel_id, e),
            }
        }

        // Load Select button
        info!("Loading Select button...");
        match load_png_from_node(&root_node, "CharSelect/BtSelect/normal/0") {
            Ok(two) => {
                info!("Select button normal loaded: {}x{}, origin: ({}, {})",
                    two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                self.select_button.width = two.texture.width();
                self.select_button.height = two.texture.height();
                self.select_button.normal = Some(two);
            }
            Err(e) => error!("Failed to load select button normal: {}", e),
        }
        self.select_button.mouse_over = load_png_from_node(&root_node, "CharSelect/BtSelect/mouseOver/0").ok();
        self.select_button.pressed = load_png_from_node(&root_node, "CharSelect/BtSelect/pressed/0").ok();
        self.select_button.disabled = load_png_from_node(&root_node, "CharSelect/BtSelect/disabled/0").ok();

        // Load Delete button
        info!("Loading Delete button...");
        match load_png_from_node(&root_node, "CharSelect/BtDelete/normal/0") {
            Ok(two) => {
                info!("Delete button normal loaded: {}x{}, origin: ({}, {})",
                    two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                self.delete_button.width = two.texture.width();
                self.delete_button.height = two.texture.height();
                self.delete_button.normal = Some(two);
            }
            Err(e) => error!("Failed to load delete button normal: {}", e),
        }
        self.delete_button.mouse_over = load_png_from_node(&root_node, "CharSelect/BtDelete/mouseOver/0").ok();
        self.delete_button.pressed = load_png_from_node(&root_node, "CharSelect/BtDelete/pressed/0").ok();
        self.delete_button.disabled = load_png_from_node(&root_node, "CharSelect/BtDelete/disabled/0").ok();

        // Load New button
        info!("Loading New character button...");
        match load_png_from_node(&root_node, "CharSelect/BtNew/normal/0") {
            Ok(two) => {
                info!("New button normal loaded: {}x{}, origin: ({}, {})",
                    two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                self.new_button.width = two.texture.width();
                self.new_button.height = two.texture.height();
                self.new_button.normal = Some(two);
            }
            Err(e) => error!("Failed to load new button normal: {}", e),
        }
        self.new_button.mouse_over = load_png_from_node(&root_node, "CharSelect/BtNew/mouseOver/0").ok();
        self.new_button.pressed = load_png_from_node(&root_node, "CharSelect/BtNew/pressed/0").ok();
        self.new_button.disabled = load_png_from_node(&root_node, "CharSelect/BtNew/disabled/0").ok();

        // Load Page Left button
        info!("Loading Page Left button...");
        match load_png_from_node(&root_node, "CharSelect/pageL/0/0") {
            Ok(two) => {
                info!("Page Left button loaded: {}x{}, origin: ({}, {})",
                    two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                self.page_left_button.width = two.texture.width();
                self.page_left_button.height = two.texture.height();
                self.page_left_button.normal = Some(two);
            }
            Err(e) => error!("Failed to load page left button: {}", e),
        }
        self.page_left_button.mouse_over = load_png_from_node(&root_node, "CharSelect/pageL/1/0").ok();

        // Load Page Right button
        info!("Loading Page Right button...");
        match load_png_from_node(&root_node, "CharSelect/pageR/0/0") {
            Ok(two) => {
                info!("Page Right button loaded: {}x{}, origin: ({}, {})",
                    two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                self.page_right_button.width = two.texture.width();
                self.page_right_button.height = two.texture.height();
                self.page_right_button.normal = Some(two);
            }
            Err(e) => error!("Failed to load page right button: {}", e),
        }
        self.page_right_button.mouse_over = load_png_from_node(&root_node, "CharSelect/pageR/1/0").ok();

        self.loaded = true;
        info!("Character selection screen assets loaded successfully");
    }

    pub fn update(&mut self, dt: f32) {
        if !self.loaded {
            return;
        }

        // Update transition animation
        if self.is_transitioning_in {
            self.transition_time += dt;
            self.transition_alpha = (self.transition_time / self.transition_duration).min(1.0);

            if self.transition_alpha >= 1.0 {
                self.is_transitioning_in = false;
            }
        }

        let center_x = screen_width() / 2.0;
        let center_y = screen_height() / 2.0;

        // Position buttons at the bottom
        let button_y = screen_height() - 50.0;

        // Center the three main buttons
        self.select_button.x = center_x;
        self.select_button.y = button_y;

        self.delete_button.x = center_x - 120.0;
        self.delete_button.y = button_y;

        self.new_button.x = center_x + 120.0;
        self.new_button.y = button_y;

        // Page navigation buttons on the left and right sides
        self.page_left_button.x = 30.0;
        self.page_left_button.y = center_y;

        self.page_right_button.x = screen_width() - 30.0;
        self.page_right_button.y = center_y;

        // Update button states
        self.select_button.update();
        self.delete_button.update();
        self.new_button.update();
        self.page_left_button.update();
        self.page_right_button.update();

        // Check for character panel clicks (with double-click support)
        if is_mouse_button_released(MouseButton::Left) && !self.characters.is_empty() {
            let (mouse_x, mouse_y) = mouse_position();
            let panel_x = 150.0;
            let mut panel_y = 150.0;
            let current_time = get_time() as f32;

            for (idx, _character) in self.characters.iter().enumerate().take(3) {
                if let Some(panel) = self.char_info_panels.first() {
                    let draw_x = panel_x - panel.origin.x;
                    let draw_y = panel_y - panel.origin.y;

                    // Check if mouse is within panel bounds
                    if mouse_x >= draw_x && mouse_x <= draw_x + panel.texture.width()
                        && mouse_y >= draw_y && mouse_y <= draw_y + panel.texture.height() {
                        
                        // Check for double-click
                        if self.last_click_index == Some(idx) && 
                           (current_time - self.last_click_time) < 0.5 {
                            // Double-click! Enter game
                            info!("Double-clicked character {}: {}", idx, self.characters[idx].name);
                            self.selected_character_index = Some(idx);
                            self.should_transition_to_game = true;
                        } else {
                            // Single click - select
                            self.selected_character_index = Some(idx);
                            info!("Selected character {}: {}", idx, self.characters[idx].name);
                        }
                        
                        self.last_click_time = current_time;
                        self.last_click_index = Some(idx);
                        break;
                    }
                }
                panel_y += 120.0;
            }
        }

        // Check for button clicks
        if self.select_button.is_clicked() {
            if let Some(character) = self.get_selected_character() {
                info!("Select button clicked! Entering game with character: {}", character.name);
                self.should_transition_to_game = true;
            } else {
                info!("Select button clicked but no character selected");
            }
        }
        if self.delete_button.is_clicked() {
            if let Some(idx) = self.selected_character_index {
                let character_name = self.characters[idx].name.clone();
                info!("Delete button clicked! Deleting character: {}", character_name);
                if let Err(e) = CharacterData::delete(&character_name) {
                    error!("Failed to delete character: {}", e);
                } else {
                    self.reload_characters();
                    self.selected_character_index = None;
                }
            } else {
                info!("Delete button clicked but no character selected");
            }
        }
        if self.new_button.is_clicked() {
            info!("New character button clicked! Transitioning to character creation");
            self.should_transition_to_char_create = true;
        }
        if self.page_left_button.is_clicked() {
            info!("Page left button clicked!");
        }
        if self.page_right_button.is_clicked() {
            info!("Page right button clicked!");
        }

        // Update cursor animation
        self.cursor_manager.update(dt);
    }

    pub fn draw(&self) {
        clear_background(BLACK);

        if !self.loaded {
            let text = "Loading Character Selection...";
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

        let center_x = screen_width() / 2.0;
        let center_y = screen_height() / 2.0;

        // Calculate alpha for fade-in effect
        let alpha = (self.transition_alpha * 255.0) as u8;
        let color = Color::from_rgba(255, 255, 255, alpha);

        // Draw sky gradient (back/1) - tiled across the screen
        if let Some(sky) = &self.background_sky {
            // Tile the sky gradient across the screen
            let sky_width = sky.texture.width();
            let sky_height = sky.texture.height();

            // Calculate how many tiles we need
            let tiles_x = (screen_width() / sky_width).ceil() as i32 + 1;
            let tiles_y = (screen_height() / sky_height).ceil() as i32 + 1;

            for tile_x in 0..tiles_x {
                for tile_y in 0..tiles_y {
                    let draw_x = tile_x as f32 * sky_width;
                    let draw_y = tile_y as f32 * sky_height;
                    draw_texture(&sky.texture, draw_x, draw_y, color);
                }
            }
        }

        // Draw scene background (back/13) on top of sky
        if let Some(scene) = &self.background_scene {
            // Center the scene background
            let draw_x = center_x - scene.origin.x;
            let draw_y = center_y - scene.origin.y;
            draw_texture(&scene.texture, draw_x, draw_y, color);
        }

        // Draw character info panels on the left side
        if !self.char_info_panels.is_empty() {
            let panel_x = 150.0; // Left side position
            let mut panel_y = 150.0; // Starting Y position

            // Draw up to 3 character slots vertically
            for (idx, character) in self.characters.iter().enumerate().take(3) {
                if let Some(panel) = self.char_info_panels.first() {
                    let draw_x = panel_x - panel.origin.x;
                    let draw_y = panel_y - panel.origin.y;

                    // Highlight selected character
                    let panel_color = if Some(idx) == self.selected_character_index {
                        Color::from_rgba(255, 255, 150, alpha)
                    } else {
                        color
                    };

                    draw_texture(&panel.texture, draw_x, draw_y, panel_color);

                    // Draw character name
                    let font_size = 18.0;
                    let text_color = Color::from_rgba(255, 255, 255, alpha);
                    draw_text(
                        &character.name,
                        draw_x + 10.0,
                        draw_y + 30.0,
                        font_size,
                        text_color,
                    );

                    // Draw character level
                    let level_text = format!("Lv. {}", character.level);
                    draw_text(
                        &level_text,
                        draw_x + 10.0,
                        draw_y + 50.0,
                        16.0,
                        text_color,
                    );

                    panel_y += 120.0; // Space between panels
                }
            }

            // Draw empty slots if less than 3 characters
            for _ in self.characters.len()..3 {
                if let Some(panel) = self.char_info_panels.first() {
                    let draw_x = panel_x - panel.origin.x;
                    let draw_y = panel_y - panel.origin.y;
                    draw_texture(&panel.texture, draw_x, draw_y, color);

                    // Draw "Empty" text
                    let font_size = 18.0;
                    let text_color = Color::from_rgba(150, 150, 150, alpha);
                    draw_text(
                        "Empty Slot",
                        draw_x + 10.0,
                        draw_y + 30.0,
                        font_size,
                        text_color,
                    );

                    panel_y += 120.0;
                }
            }
        }

        // Helper function to draw a button with fade
        let draw_button_with_fade = |button: &Button, alpha: u8| {
            if let Some(tex_with_origin) = match button.state {
                ButtonState::Normal => &button.normal,
                ButtonState::MouseOver => &button.mouse_over,
                ButtonState::Pressed => &button.pressed,
                ButtonState::Disabled => &button.disabled,
            } {
                let draw_x = button.x - tex_with_origin.origin.x;
                let draw_y = button.y - tex_with_origin.origin.y;
                let color = Color::from_rgba(255, 255, 255, alpha);
                draw_texture(&tex_with_origin.texture, draw_x, draw_y, color);
            }
        };

        // Draw all buttons with fade-in effect
        draw_button_with_fade(&self.select_button, alpha);
        draw_button_with_fade(&self.delete_button, alpha);
        draw_button_with_fade(&self.new_button, alpha);
        draw_button_with_fade(&self.page_left_button, alpha);
        draw_button_with_fade(&self.page_right_button, alpha);

        // Draw custom cursor (always on top)
        self.cursor_manager.draw();
    }
}

/// Run the character selection screen loop
pub async fn run_character_selection_loop() {
    let mut state = CharacterSelectionState::new();
    state.load_assets().await;

    loop {
        let dt = get_frame_time();
        state.update(dt);
        state.draw();
        next_frame().await;
    }
}
