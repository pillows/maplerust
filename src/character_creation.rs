use macroquad::prelude::*;
use crate::assets::AssetManager;
use crate::character::CharacterData;
use crate::cursor::CursorManager;
use crate::flags;
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

/// Movable UI elements
#[derive(PartialEq, Clone, Copy, Debug)]
enum MovableElement {
    CharNamePanel,
    CharJobPanel,
    CharSetPanel,
    StatTable,
}

/// Drag state for movable elements
struct DragState {
    active: bool,
    element: Option<MovableElement>,
    offset: Vec2,
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

/// Character creation state
pub struct CharacterCreationState {
    background_sky: Option<TextureWithOrigin>,
    background_scene: Option<TextureWithOrigin>,

    // UI Panels
    char_name_panel: Option<TextureWithOrigin>,
    char_job_panel: Option<TextureWithOrigin>,
    char_set_panel: Option<TextureWithOrigin>,
    stat_table: Option<TextureWithOrigin>,

    // Buttons
    yes_button: Button,
    no_button: Button,
    left_button: Button,
    right_button: Button,
    check_button: Button,

    // Character state
    character_name: String,
    selected_job: usize,

    // Character anchor point (where character setting assets align to)
    character_anchor: Vec2,

    // Settings
    movable_mode: bool,

    // Drag state
    drag_state: DragState,

    // Position overrides for movable elements
    char_name_pos: Option<Vec2>,
    char_job_pos: Option<Vec2>,
    char_set_pos: Option<Vec2>,
    stat_table_pos: Option<Vec2>,

    loaded: bool,

    // Transition animation
    transition_alpha: f32,
    transition_duration: f32,
    transition_time: f32,
    is_transitioning_in: bool,

    // Creation state
    character_created: bool,
    transition_to_char_select: bool,
    
    // Cursor
    cursor_manager: CursorManager,
    
    // Name input
    name_input_active: bool,
}

impl CharacterCreationState {
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
            char_name_panel: None,
            char_job_panel: None,
            char_set_panel: None,
            stat_table: None,
            yes_button: Button::new(400.0, 300.0),
            no_button: Button::new(400.0, 300.0),
            left_button: Button::new(400.0, 300.0),
            right_button: Button::new(400.0, 300.0),
            check_button: Button::new(400.0, 300.0),
            character_name: String::new(),
            selected_job: 0,
            character_anchor: vec2(0.0, 0.0),
            movable_mode: flags::DEFAULT_MOVABLE_MODE,
            drag_state: DragState {
                active: false,
                element: None,
                offset: vec2(0.0, 0.0),
            },
            char_name_pos: None,
            char_job_pos: None,
            char_set_pos: None,
            stat_table_pos: None,
            loaded: false,
            transition_alpha: 0.0,
            transition_duration: 0.5,
            transition_time: 0.0,
            is_transitioning_in: true,
            character_created: false,
            transition_to_char_select: false,
            cursor_manager: CursorManager::new(),
            name_input_active: true,
        }
    }

    /// Check if should transition back to character selection
    pub fn should_transition_to_char_select(&self) -> bool {
        self.transition_to_char_select
    }

    /// Load all character creation screen assets from Login.img
    pub async fn load_assets(&mut self) {
        info!("Loading character creation screen assets...");
        
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

        // Load background from login.img
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
                // Load sky gradient (back/1)
                match load_png_from_node(&bg_root_node, "back/1") {
                    Ok(two) => {
                        info!("Sky gradient loaded: {}x{}, origin: ({}, {})",
                            two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                        self.background_sky = Some(two);
                    }
                    Err(e) => error!("Failed to load sky gradient: {}", e),
                }

                // Load scene background (back/14)
                match load_png_from_node(&bg_root_node, "back/14") {
                    Ok(two) => {
                        info!("Scene background loaded: {}x{}, origin: ({}, {})",
                            two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                        self.background_scene = Some(two);
                    }
                    Err(e) => error!("Failed to load scene background: {}", e),
                }
            }
        }

        // Load UI panels
        match load_png_from_node(&root_node, "NewChar/charName") {
            Ok(two) => {
                info!("Character name panel loaded: {}x{}", two.texture.width(), two.texture.height());
                self.char_name_panel = Some(two);
            }
            Err(e) => error!("Failed to load character name panel: {}", e),
        }

        match load_png_from_node(&root_node, "NewChar/charJob") {
            Ok(two) => {
                info!("Character job panel loaded: {}x{}", two.texture.width(), two.texture.height());
                self.char_job_panel = Some(two);
            }
            Err(e) => error!("Failed to load character job panel: {}", e),
        }

        match load_png_from_node(&root_node, "NewChar/charSet") {
            Ok(two) => {
                info!("Character set panel loaded: {}x{}", two.texture.width(), two.texture.height());
                self.char_set_panel = Some(two);
            }
            Err(e) => error!("Failed to load character set panel: {}", e),
        }

        match load_png_from_node(&root_node, "NewChar/statTb") {
            Ok(two) => {
                info!("Stats table loaded: {}x{}", two.texture.width(), two.texture.height());
                self.stat_table = Some(two);
            }
            Err(e) => error!("Failed to load stats table: {}", e),
        }

        // Load Yes button
        info!("Loading Yes button...");
        match load_png_from_node(&root_node, "NewChar/BtYes/normal/0") {
            Ok(two) => {
                self.yes_button.width = two.texture.width();
                self.yes_button.height = two.texture.height();
                self.yes_button.normal = Some(two);
            }
            Err(e) => error!("Failed to load yes button normal: {}", e),
        }
        self.yes_button.mouse_over = load_png_from_node(&root_node, "NewChar/BtYes/mouseOver/0").ok();
        self.yes_button.pressed = load_png_from_node(&root_node, "NewChar/BtYes/pressed/0").ok();
        self.yes_button.disabled = load_png_from_node(&root_node, "NewChar/BtYes/disabled/0").ok();

        // Load No button
        info!("Loading No button...");
        match load_png_from_node(&root_node, "NewChar/BtNo/normal/0") {
            Ok(two) => {
                self.no_button.width = two.texture.width();
                self.no_button.height = two.texture.height();
                self.no_button.normal = Some(two);
            }
            Err(e) => error!("Failed to load no button normal: {}", e),
        }
        self.no_button.mouse_over = load_png_from_node(&root_node, "NewChar/BtNo/mouseOver/0").ok();
        self.no_button.pressed = load_png_from_node(&root_node, "NewChar/BtNo/pressed/0").ok();
        self.no_button.disabled = load_png_from_node(&root_node, "NewChar/BtNo/disabled/0").ok();

        // Load Left button
        info!("Loading Left button...");
        match load_png_from_node(&root_node, "NewChar/BtLeft/normal/0") {
            Ok(two) => {
                self.left_button.width = two.texture.width();
                self.left_button.height = two.texture.height();
                self.left_button.normal = Some(two);
            }
            Err(e) => error!("Failed to load left button normal: {}", e),
        }
        self.left_button.mouse_over = load_png_from_node(&root_node, "NewChar/BtLeft/mouseOver/0").ok();
        self.left_button.pressed = load_png_from_node(&root_node, "NewChar/BtLeft/pressed/0").ok();
        self.left_button.disabled = load_png_from_node(&root_node, "NewChar/BtLeft/disabled/0").ok();

        // Load Right button
        info!("Loading Right button...");
        match load_png_from_node(&root_node, "NewChar/BtRight/normal/0") {
            Ok(two) => {
                self.right_button.width = two.texture.width();
                self.right_button.height = two.texture.height();
                self.right_button.normal = Some(two);
            }
            Err(e) => error!("Failed to load right button normal: {}", e),
        }
        self.right_button.mouse_over = load_png_from_node(&root_node, "NewChar/BtRight/mouseOver/0").ok();
        self.right_button.pressed = load_png_from_node(&root_node, "NewChar/BtRight/pressed/0").ok();
        self.right_button.disabled = load_png_from_node(&root_node, "NewChar/BtRight/disabled/0").ok();

        // Load Check button
        info!("Loading Check button...");
        match load_png_from_node(&root_node, "NewChar/BtCheck/normal/0") {
            Ok(two) => {
                self.check_button.width = two.texture.width();
                self.check_button.height = two.texture.height();
                self.check_button.normal = Some(two);
            }
            Err(e) => error!("Failed to load check button normal: {}", e),
        }
        self.check_button.mouse_over = load_png_from_node(&root_node, "NewChar/BtCheck/mouseOver/0").ok();
        self.check_button.pressed = load_png_from_node(&root_node, "NewChar/BtCheck/pressed/0").ok();
        self.check_button.disabled = load_png_from_node(&root_node, "NewChar/BtCheck/disabled/0").ok();

        self.loaded = true;
        info!("Character creation screen assets loaded successfully");
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

        // Set character anchor point (next to mushroom head character)
        // This determines where character setting assets will align
        self.character_anchor = vec2(center_x + 80.0, center_y);

        // Toggle movable mode with 'M' key
        if is_key_pressed(KeyCode::M) {
            self.movable_mode = !self.movable_mode;
            info!("Movable mode: {}", if self.movable_mode { "ON" } else { "OFF" });
        }

        // Handle dragging of movable elements
        if self.movable_mode {
            let (mouse_x, mouse_y) = mouse_position();
            let mouse_pos = vec2(mouse_x, mouse_y);

            // Start dragging
            if is_mouse_button_pressed(MouseButton::Left) && !self.drag_state.active {
                self.drag_state.element = self.get_element_at_position(mouse_pos, center_x, center_y);
                if let Some(element) = self.drag_state.element {
                    self.drag_state.active = true;
                    let element_pos = self.get_element_position(element, center_x, center_y);
                    self.drag_state.offset = mouse_pos - element_pos;
                    info!("Started dragging {:?}", element);
                }
            }

            // Update drag position
            if self.drag_state.active && is_mouse_button_down(MouseButton::Left) {
                if let Some(element) = self.drag_state.element {
                    let new_pos = mouse_pos - self.drag_state.offset;
                    self.set_element_position(element, new_pos);
                }
            }

            // End dragging
            if is_mouse_button_released(MouseButton::Left) && self.drag_state.active {
                info!("Stopped dragging {:?}", self.drag_state.element);
                self.drag_state.active = false;
                self.drag_state.element = None;
            }
        }

        // Position buttons at the bottom
        let button_y = screen_height() - 50.0;

        self.yes_button.x = center_x - 60.0;
        self.yes_button.y = button_y;

        self.no_button.x = center_x + 60.0;
        self.no_button.y = button_y;

        self.left_button.x = center_x - 180.0;
        self.left_button.y = center_y;

        self.right_button.x = center_x + 180.0;
        self.right_button.y = center_y;

        self.check_button.x = center_x + 200.0;
        self.check_button.y = center_y - 100.0;

        // Update button states
        self.yes_button.update();
        self.no_button.update();
        self.left_button.update();
        self.right_button.update();
        self.check_button.update();

        // Check for button clicks
        if self.yes_button.is_clicked() && !self.character_created {
            // Validate character name if validation is enabled
            let name_valid = if flags::ENABLE_CHARACTER_VALIDATION {
                let name_len = self.character_name.len();
                name_len >= flags::MIN_CHARACTER_NAME_LENGTH &&
                name_len <= flags::MAX_CHARACTER_NAME_LENGTH
            } else {
                !self.character_name.is_empty()
            };

            if name_valid {
                info!("Yes button clicked! Creating character: {}", self.character_name);

                // Create and save character data
                let character = CharacterData::new(self.character_name.clone(), self.selected_job);
                match character.save() {
                    Ok(_) => {
                        info!("Character '{}' saved successfully!", self.character_name);
                        self.character_created = true;
                        self.transition_to_char_select = true;
                    }
                    Err(e) => {
                        error!("Failed to save character: {}", e);
                    }
                }
            } else {
                warn!("Character name validation failed: '{}' (length: {})",
                      self.character_name, self.character_name.len());
            }
        }
        if self.no_button.is_clicked() {
            info!("No button clicked! Canceling character creation");
            self.transition_to_char_select = true;
        }
        if self.left_button.is_clicked() {
            info!("Left button clicked!");
        }
        if self.right_button.is_clicked() {
            info!("Right button clicked!");
        }
        if self.check_button.is_clicked() {
            info!("Check button clicked!");
        }

        // Handle text input for character name
        if let Some(key) = get_last_key_pressed() {
            if key == KeyCode::Backspace && !self.character_name.is_empty() {
                self.character_name.pop();
            }
        }

        // Get typed characters
        while let Some(ch) = get_char_pressed() {
            let max_length = if flags::ENABLE_CHARACTER_VALIDATION {
                flags::MAX_CHARACTER_NAME_LENGTH
            } else {
                12 // Default fallback
            };

            if ch.is_alphanumeric() && self.character_name.len() < max_length {
                self.character_name.push(ch);
            }
        }
        
        // Update cursor
        self.cursor_manager.update(dt);
    }

    /// Get which element is at the given mouse position
    fn get_element_at_position(&self, mouse_pos: Vec2, center_x: f32, center_y: f32) -> Option<MovableElement> {
        // Check in reverse draw order (top-most first)
        let elements = [
            (MovableElement::CharNamePanel, &self.char_name_panel, self.char_name_pos, vec2(center_x, 80.0)),
            (MovableElement::CharJobPanel, &self.char_job_panel, self.char_job_pos, vec2(center_x - 200.0, center_y - 100.0)),
            (MovableElement::CharSetPanel, &self.char_set_panel, self.char_set_pos, vec2(self.character_anchor.x, center_y - 100.0)),
            (MovableElement::StatTable, &self.stat_table, self.stat_table_pos, vec2(center_x, center_y + 100.0)),
        ];

        for (element, texture_opt, pos_override, default_pos) in elements.iter() {
            if let Some(tex) = texture_opt {
                let pos = pos_override.unwrap_or(*default_pos);
                let draw_x = pos.x - tex.origin.x;
                let draw_y = pos.y - tex.origin.y;

                if mouse_pos.x >= draw_x && mouse_pos.x <= draw_x + tex.texture.width()
                    && mouse_pos.y >= draw_y && mouse_pos.y <= draw_y + tex.texture.height() {
                    return Some(*element);
                }
            }
        }

        None
    }

    /// Get the current position of an element
    fn get_element_position(&self, element: MovableElement, center_x: f32, center_y: f32) -> Vec2 {
        match element {
            MovableElement::CharNamePanel => {
                self.char_name_pos.unwrap_or(vec2(center_x, 80.0))
            }
            MovableElement::CharJobPanel => {
                self.char_job_pos.unwrap_or(vec2(center_x - 200.0, center_y - 100.0))
            }
            MovableElement::CharSetPanel => {
                self.char_set_pos.unwrap_or(vec2(self.character_anchor.x, center_y - 100.0))
            }
            MovableElement::StatTable => {
                self.stat_table_pos.unwrap_or(vec2(center_x, center_y + 100.0))
            }
        }
    }

    /// Set the position of an element
    fn set_element_position(&mut self, element: MovableElement, pos: Vec2) {
        match element {
            MovableElement::CharNamePanel => self.char_name_pos = Some(pos),
            MovableElement::CharJobPanel => self.char_job_pos = Some(pos),
            MovableElement::CharSetPanel => self.char_set_pos = Some(pos),
            MovableElement::StatTable => self.stat_table_pos = Some(pos),
        }
    }

    pub fn draw(&self) {
        clear_background(BLACK);

        if !self.loaded {
            let text = "Loading Character Creation...";
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

        // Draw sky gradient (back/1) - tiled
        if let Some(sky) = &self.background_sky {
            let sky_width = sky.texture.width();
            let sky_height = sky.texture.height();

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

        // Draw scene background (back/13)
        if let Some(scene) = &self.background_scene {
            let draw_x = center_x - scene.origin.x;
            let draw_y = center_y - scene.origin.y;
            draw_texture(&scene.texture, draw_x, draw_y, color);
        }

        // Draw UI panels
        if let Some(panel) = &self.char_name_panel {
            let pos = self.char_name_pos.unwrap_or(vec2(center_x, 80.0));
            let draw_x = pos.x - panel.origin.x;
            let draw_y = pos.y - panel.origin.y;
            draw_texture(&panel.texture, draw_x, draw_y, color);

            // Draw character name text
            if !self.character_name.is_empty() {
                let font_size = 20.0;
                let text_color = Color::from_rgba(0, 0, 0, alpha);
                let text_dimensions = measure_text(&self.character_name, None, font_size as u16, 1.0);
                draw_text(
                    &self.character_name,
                    draw_x + panel.texture.width() / 2.0 - text_dimensions.width / 2.0,
                    draw_y + panel.texture.height() / 2.0 + font_size / 3.0,
                    font_size,
                    text_color,
                );
            }
        }

        if let Some(panel) = &self.char_job_panel {
            let pos = self.char_job_pos.unwrap_or(vec2(center_x - 200.0, center_y - 100.0));
            let draw_x = pos.x - panel.origin.x;
            let draw_y = pos.y - panel.origin.y;
            draw_texture(&panel.texture, draw_x, draw_y, color);
        }

        if let Some(panel) = &self.char_set_panel {
            // Use character anchor point to position this panel (or override position if set)
            let pos = self.char_set_pos.unwrap_or(vec2(self.character_anchor.x, center_y - 100.0));
            let draw_x = pos.x - panel.origin.x;
            let draw_y = pos.y - panel.origin.y;
            draw_texture(&panel.texture, draw_x, draw_y, color);
        }

        if let Some(panel) = &self.stat_table {
            let pos = self.stat_table_pos.unwrap_or(vec2(center_x, center_y + 100.0));
            let draw_x = pos.x - panel.origin.x;
            let draw_y = pos.y - panel.origin.y;
            draw_texture(&panel.texture, draw_x, draw_y, color);
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
        draw_button_with_fade(&self.yes_button, alpha);
        draw_button_with_fade(&self.no_button, alpha);
        draw_button_with_fade(&self.left_button, alpha);
        draw_button_with_fade(&self.right_button, alpha);
        draw_button_with_fade(&self.check_button, alpha);

        // Draw movable mode indicator
        if self.movable_mode {
            let text = "MOVABLE MODE (Press M to toggle)";
            let font_size = 20.0;
            let text_dimensions = measure_text(text, None, font_size as u16, 1.0);

            // Draw background for text
            let padding = 10.0;
            draw_rectangle(
                screen_width() / 2.0 - text_dimensions.width / 2.0 - padding,
                10.0,
                text_dimensions.width + padding * 2.0,
                text_dimensions.height + padding,
                Color::from_rgba(0, 0, 0, 180),
            );

            draw_text(
                text,
                screen_width() / 2.0 - text_dimensions.width / 2.0,
                10.0 + font_size,
                font_size,
                YELLOW,
            );

            // If dragging, show which element is being dragged
            if self.drag_state.active {
                if let Some(element) = self.drag_state.element {
                    let drag_text = format!("Dragging: {:?}", element);
                    let drag_font_size = 18.0;
                    let drag_dimensions = measure_text(&drag_text, None, drag_font_size as u16, 1.0);
                    draw_text(
                        &drag_text,
                        screen_width() / 2.0 - drag_dimensions.width / 2.0,
                        50.0,
                        drag_font_size,
                        GREEN,
                    );
                }
            }
        }
        
        // Draw name input prompt
        let input_text = if self.character_name.is_empty() {
            "Enter your character name..."
        } else {
            &self.character_name
        };
        let input_color = if self.character_name.is_empty() { GRAY } else { WHITE };
        draw_text(input_text, center_x - 100.0, 120.0, 16.0, input_color);
        
        // Draw cursor
        self.cursor_manager.draw();
    }
}
