use macroquad::prelude::*;
use crate::assets::AssetManager;
use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzReader, WzObjectType, WzNodeCast};

const LOGIN_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/Login.img";
const LOGIN_CACHE_NAME: &str = "Login.img";

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
    was_pressed: bool, // Track if button was pressed in previous frame
    just_clicked: bool, // Set to true when a click is detected, reset after is_clicked() is called
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
            was_pressed: false,
            just_clicked: false,
        }
    }

    fn update(&mut self) {
        let (mouse_x, mouse_y) = mouse_position();

        // Get the actual draw position accounting for origin offset
        let (draw_x, draw_y) = if let Some(tex) = &self.normal {
            (self.x - tex.origin.x, self.y - tex.origin.y)
        } else {
            (self.x, self.y)
        };

        let mouse_in_bounds = mouse_x >= draw_x
            && mouse_x <= draw_x + self.width
            && mouse_y >= draw_y
            && mouse_y <= draw_y + self.height;

        if self.state == ButtonState::Disabled {
            return;
        }

        // Reset just_clicked at the start of each frame
        self.just_clicked = false;
        
        // Store previous pressed state BEFORE updating
        let was_pressed_before = self.was_pressed;
        
        // Check if button is currently pressed
        let currently_pressed = mouse_in_bounds && is_mouse_button_down(MouseButton::Left);
        
        // Detect click: was pressed before AND mouse button was just released AND mouse is in bounds
        if was_pressed_before && is_mouse_button_released(MouseButton::Left) && mouse_in_bounds {
            self.just_clicked = true;
        }
        
        if mouse_in_bounds {
            if is_mouse_button_down(MouseButton::Left) {
                self.state = ButtonState::Pressed;
            } else {
                self.state = ButtonState::MouseOver;
            }
        } else {
            self.state = ButtonState::Normal;
        }
        
        // Update was_pressed for next frame
        self.was_pressed = currently_pressed;
    }

    fn is_clicked(&mut self) -> bool {
        // Return the click state detected in update()
        let clicked = self.just_clicked;
        // Reset after reading
        if clicked {
            self.just_clicked = false;
        }
        clicked
    }

    fn draw(&self) {
        let tex_with_origin = match self.state {
            ButtonState::Normal => &self.normal,
            ButtonState::MouseOver => &self.mouse_over,
            ButtonState::Pressed => &self.pressed,
            ButtonState::Disabled => &self.disabled,
        };

        if let Some(two) = tex_with_origin {
            // Draw using origin offset
            let draw_x = self.x - two.origin.x;
            let draw_y = self.y - two.origin.y;
            draw_texture(&two.texture, draw_x, draw_y, WHITE);
        }
    }
}

/// Structure to hold texture with its origin point
struct TextureWithOrigin {
    texture: Texture2D,
    origin: Vec2,
}

/// Load a single PNG texture with origin from an already-parsed WZ node
fn load_png_from_node(root_node: &WzNodeArc, path: &str) -> Result<TextureWithOrigin, String> {
    // Navigate to the PNG path
    let node = root_node
        .read()
        .unwrap()
        .at_path_parsed(path)
        .map_err(|e| format!("Failed to navigate to '{}': {:?}", path, e))?;

    // Extract PNG texture data
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

    // Load origin coordinates
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

/// Which input field is currently focused
#[derive(PartialEq, Clone, Copy)]
enum FocusedField {
    None,
    Username,
    Password,
}

/// Login screen state
pub struct LoginState {
    // Background
    background: Option<TextureWithOrigin>,

    // UI Labels
    maple_id_label: Option<TextureWithOrigin>,
    password_label: Option<TextureWithOrigin>,

    // Additional UI elements
    checkbox_unchecked: Option<TextureWithOrigin>,
    checkbox_checked: Option<TextureWithOrigin>,

    // Buttons
    login_button: Button,
    new_button: Button,
    quit_button: Button,

    // Input state
    username: String,
    password: String,
    focused_field: FocusedField,
    save_id_checked: bool,

    // State
    loaded: bool,
    
    // Debug: Fine-tuning mode for positioning input fields
    debug_mode: bool,
    username_center_x_ratio: f32,
    username_center_y_ratio: f32,
    password_center_x_ratio: f32,
    password_center_y_ratio: f32,
    selected_ratio: usize, // 0=username_x, 1=username_y, 2=password_x, 3=password_y
}

impl LoginState {
    pub fn new() -> Self {
        // Buttons will be positioned relative to screen center in draw method
        // For now, use placeholder positions - these will be overridden based on screen size
        Self {
            background: None,
            maple_id_label: None,
            password_label: None,
            checkbox_unchecked: None,
            checkbox_checked: None,
            login_button: Button::new(400.0, 300.0),
            new_button: Button::new(400.0, 340.0),
            quit_button: Button::new(400.0, 380.0),
            username: String::new(),
            password: String::new(),
            focused_field: FocusedField::None,
            save_id_checked: false,
            loaded: false,
            // Debug fine-tuning mode - press F1 to toggle
            debug_mode: false,
            username_center_x_ratio: 0.0,
            username_center_y_ratio: 0.0,
            password_center_x_ratio: 0.0,
            password_center_y_ratio: 0.0,
            selected_ratio: 0,
        }
    }

    /// Load all login screen assets from Login.img - optimized to parse WZ file only once
    pub async fn load_assets(&mut self) {
        info!("Loading login screen assets...");

        // Fetch and parse the WZ file once
        let bytes = match AssetManager::fetch_and_cache(LOGIN_URL, LOGIN_CACHE_NAME).await {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Failed to fetch Login.img: {}", e);
                return;
            }
        };

        info!("Parsing Login.img (size: {} bytes)...", bytes.len());

        // Guess Version / IV
        let wz_iv = match guess_iv_from_wz_img(&bytes) {
            Some(iv) => iv,
            None => {
                error!("Unable to guess version from Login.img");
                return;
            }
        };

        // Create Reader and WZ structure
        let reader = Arc::new(WzReader::new(bytes.clone()).with_iv(wz_iv));
        let wz_image = WzImage::new(&LOGIN_CACHE_NAME.into(), 0, bytes.len(), &reader);
        let root_node: WzNodeArc = WzNode::new(&LOGIN_CACHE_NAME.into(), wz_image, None).into();

        // Parse root node once
        if let Err(e) = root_node.write().unwrap().parse(&root_node) {
            error!("Failed to parse WZ root node: {:?}", e);
            return;
        }

        info!("WZ file parsed successfully, loading assets...");

        // Now load all assets from the parsed structure (paths without "Login/" prefix)
        // The root node is "Login.img" so we navigate directly to child nodes

        // Load background
        match load_png_from_node(&root_node, "Title_new/backgrd") {
            Ok(two) => {
                info!("Background loaded: {}x{}, origin: ({}, {})", two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                self.background = Some(two);
            }
            Err(e) => error!("Failed to load background: {}", e),
        }

        // Load MapleID label
        match load_png_from_node(&root_node, "Title_new/mapleID") {
            Ok(two) => {
                info!("MapleID label loaded: {}x{}, origin: ({}, {})", two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                self.maple_id_label = Some(two);
            }
            Err(e) => error!("Failed to load MapleID label: {}", e),
        }

        // Load Password label
        match load_png_from_node(&root_node, "Title_new/PW") {
            Ok(two) => {
                info!("Password label loaded: {}x{}, origin: ({}, {})", two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                self.password_label = Some(two);
            }
            Err(e) => error!("Failed to load Password label: {}", e),
        }

        // Load Login button states
        info!("Loading Login button...");
        match load_png_from_node(&root_node, "Title_new/BtLogin/normal/0") {
            Ok(two) => {
                info!("Login button normal loaded: {}x{}, origin: ({}, {})", two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                self.login_button.width = two.texture.width();
                self.login_button.height = two.texture.height();
                self.login_button.normal = Some(two);
            }
            Err(e) => error!("Failed to load Login button normal: {}", e),
        }

        self.login_button.mouse_over = load_png_from_node(&root_node, "Title_new/BtLogin/mouseOver/0").ok();
        self.login_button.pressed = load_png_from_node(&root_node, "Title_new/BtLogin/pressed/0").ok();
        self.login_button.disabled = load_png_from_node(&root_node, "Title_new/BtLogin/disabled/0").ok();

        // Load New Account button states
        info!("Loading New Account button...");
        match load_png_from_node(&root_node, "Title_new/BtNew/normal/0") {
            Ok(two) => {
                info!("New button normal loaded: {}x{}, origin: ({}, {})", two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                self.new_button.width = two.texture.width();
                self.new_button.height = two.texture.height();
                self.new_button.normal = Some(two);
            }
            Err(e) => error!("Failed to load New button normal: {}", e),
        }

        self.new_button.mouse_over = load_png_from_node(&root_node, "Title_new/BtNew/mouseOver/0").ok();
        self.new_button.pressed = load_png_from_node(&root_node, "Title_new/BtNew/pressed/0").ok();
        self.new_button.disabled = load_png_from_node(&root_node, "Title_new/BtNew/disabled/0").ok();

        // Load Quit button states
        info!("Loading Quit button...");
        match load_png_from_node(&root_node, "Title_new/BtQuit/normal/0") {
            Ok(two) => {
                info!("Quit button normal loaded: {}x{}, origin: ({}, {})", two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                self.quit_button.width = two.texture.width();
                self.quit_button.height = two.texture.height();
                self.quit_button.normal = Some(two);
            }
            Err(e) => error!("Failed to load Quit button normal: {}", e),
        }

        self.quit_button.mouse_over = load_png_from_node(&root_node, "Title_new/BtQuit/mouseOver/0").ok();
        self.quit_button.pressed = load_png_from_node(&root_node, "Title_new/BtQuit/pressed/0").ok();
        self.quit_button.disabled = load_png_from_node(&root_node, "Title_new/BtQuit/disabled/0").ok();

        // Load checkbox states
        info!("Loading checkbox...");
        self.checkbox_unchecked = load_png_from_node(&root_node, "Title_new/check/0").ok();
        self.checkbox_checked = load_png_from_node(&root_node, "Title_new/check/1").ok();

        self.loaded = true;
        info!("Login screen assets loaded successfully");
    }

    pub fn update(&mut self, _dt: f32) {
        if !self.loaded {
            return;
        }

        // Update button positions relative to screen center
        let center_x = screen_width() / 2.0;
        let center_y = screen_height() / 2.0;

        self.login_button.x = center_x + 150.0;
        self.login_button.y = center_y + 40.0;

        self.new_button.x = center_x + 150.0;
        self.new_button.y = center_y + 80.0;

        self.quit_button.x = center_x + 150.0;
        self.quit_button.y = center_y + 120.0;

        // Handle mouse clicks on input fields (using MapleID and PW textures as input fields)
        let (mouse_x, mouse_y) = mouse_position();

        // Calculate input field positions relative to background dimensions (same as in draw())
        let (bg_x, bg_y, bg_width, bg_height) = if let Some(bg) = &self.background {
            let bg_width = bg.texture.width();
            let bg_height = bg.texture.height();
            let bg_x = (screen_width() - bg_width) / 2.0;
            let bg_y = (screen_height() - bg_height) / 2.0;
            (bg_x, bg_y, bg_width, bg_height)
        } else {
            (0.0, 0.0, 0.0, 0.0)
        };

        // Username field position and dimensions (from MapleID texture)
        // Using same ratios as in draw() function
        let username_field_x = bg_x + bg_width * self.username_center_x_ratio;
        let username_field_y = bg_y + bg_height * self.username_center_y_ratio;
        let username_width = self.maple_id_label.as_ref().map(|t| t.texture.width()).unwrap_or(200.0);
        let username_height = self.maple_id_label.as_ref().map(|t| t.texture.height()).unwrap_or(30.0);

        // Password field position and dimensions (from PW texture)
        let password_field_x = bg_x + bg_width * self.password_center_x_ratio;
        let password_field_y = bg_y + bg_height * self.password_center_y_ratio;
        let password_width = self.password_label.as_ref().map(|t| t.texture.width()).unwrap_or(200.0);
        let password_height = self.password_label.as_ref().map(|t| t.texture.height()).unwrap_or(30.0);

        // Checkbox position (relative to background dimensions, same as in draw())
        let checkbox_anchor_x = bg_x + bg_width * 0.28;
        let checkbox_anchor_y = bg_y + bg_height * 0.91;

        if is_mouse_button_pressed(MouseButton::Left) {
            // Check if clicked on username field
            if mouse_x >= username_field_x && mouse_x <= username_field_x + username_width
                && mouse_y >= username_field_y && mouse_y <= username_field_y + username_height
            {
                self.focused_field = FocusedField::Username;
            }
            // Check if clicked on password field
            else if mouse_x >= password_field_x && mouse_x <= password_field_x + password_width
                && mouse_y >= password_field_y && mouse_y <= password_field_y + password_height
            {
                self.focused_field = FocusedField::Password;
            }
            // Check if clicked on checkbox
            else if mouse_x >= checkbox_anchor_x - 10.0 && mouse_x <= checkbox_anchor_x + 20.0
                && mouse_y >= checkbox_anchor_y && mouse_y <= checkbox_anchor_y + 30.0
            {
                self.save_id_checked = !self.save_id_checked;
            }
            else {
                self.focused_field = FocusedField::None;
            }
        }

        // Handle keyboard input
        if self.focused_field != FocusedField::None {
            // Get all pressed keys this frame
            let mut chars_to_add = String::new();

            // Check for character input
            if let Some(key) = get_last_key_pressed() {
                let is_shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);

                match key {
                    KeyCode::Backspace => {
                        match self.focused_field {
                            FocusedField::Username => { self.username.pop(); }
                            FocusedField::Password => { self.password.pop(); }
                            _ => {}
                        }
                    }
                    KeyCode::Tab => {
                        // Switch between fields
                        self.focused_field = match self.focused_field {
                            FocusedField::Username => FocusedField::Password,
                            FocusedField::Password => FocusedField::Username,
                            _ => FocusedField::Username,
                        };
                    }
                    KeyCode::Enter => {
                        if self.focused_field == FocusedField::Password {
                            info!("Login with username: {}", self.username);
                            // TODO: Handle login
                        }
                    }
                    _ => {
                        // Handle alphanumeric input
                        if let Some(c) = key_to_char(key, is_shift) {
                            chars_to_add.push(c);
                        }
                    }
                }
            }

            // Add typed characters
            if !chars_to_add.is_empty() {
                match self.focused_field {
                    FocusedField::Username => self.username.push_str(&chars_to_add),
                    FocusedField::Password => self.password.push_str(&chars_to_add),
                    _ => {}
                }
            }
        }

        // Debug fine-tuning mode controls
        if is_key_pressed(KeyCode::F1) {
            self.debug_mode = !self.debug_mode;
            info!("Debug fine-tuning mode: {}", if self.debug_mode { "ON" } else { "OFF" });
        }
        
        if self.debug_mode {
            // Select which ratio to adjust (1-4 keys)
            if is_key_pressed(KeyCode::Key1) {
                self.selected_ratio = 0;
                info!("Selected: Username X ratio");
            }
            if is_key_pressed(KeyCode::Key2) {
                self.selected_ratio = 1;
                info!("Selected: Username Y ratio");
            }
            if is_key_pressed(KeyCode::Key3) {
                self.selected_ratio = 2;
                info!("Selected: Password X ratio");
            }
            if is_key_pressed(KeyCode::Key4) {
                self.selected_ratio = 3;
                info!("Selected: Password Y ratio");
            }
            
            // Adjust selected ratio with arrow keys or WASD
            let adjust_amount = if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
                0.001 // Fine adjustment with Shift held
            } else {
                0.01 // Normal adjustment
            };
            
            let mut ratio_changed = false;
            if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
                match self.selected_ratio {
                    0 => { self.username_center_x_ratio += adjust_amount; ratio_changed = true; }
                    1 => { self.username_center_y_ratio += adjust_amount; ratio_changed = true; }
                    2 => { self.password_center_x_ratio += adjust_amount; ratio_changed = true; }
                    3 => { self.password_center_y_ratio += adjust_amount; ratio_changed = true; }
                    _ => {}
                }
            }
            if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
                match self.selected_ratio {
                    0 => { self.username_center_x_ratio -= adjust_amount; ratio_changed = true; }
                    1 => { self.username_center_y_ratio -= adjust_amount; ratio_changed = true; }
                    2 => { self.password_center_x_ratio -= adjust_amount; ratio_changed = true; }
                    3 => { self.password_center_y_ratio -= adjust_amount; ratio_changed = true; }
                    _ => {}
                }
            }
            if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
                match self.selected_ratio {
                    0 => { self.username_center_x_ratio += adjust_amount; ratio_changed = true; }
                    1 => { self.username_center_y_ratio += adjust_amount; ratio_changed = true; }
                    2 => { self.password_center_x_ratio += adjust_amount; ratio_changed = true; }
                    3 => { self.password_center_y_ratio += adjust_amount; ratio_changed = true; }
                    _ => {}
                }
            }
            if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
                match self.selected_ratio {
                    0 => { self.username_center_x_ratio -= adjust_amount; ratio_changed = true; }
                    1 => { self.username_center_y_ratio -= adjust_amount; ratio_changed = true; }
                    2 => { self.password_center_x_ratio -= adjust_amount; ratio_changed = true; }
                    3 => { self.password_center_y_ratio -= adjust_amount; ratio_changed = true; }
                    _ => {}
                }
            }
            
            // Clamp ratios to valid range
            self.username_center_x_ratio = self.username_center_x_ratio.clamp(0.0, 1.0);
            self.username_center_y_ratio = self.username_center_y_ratio.clamp(0.0, 1.0);
            self.password_center_x_ratio = self.password_center_x_ratio.clamp(0.0, 1.0);
            self.password_center_y_ratio = self.password_center_y_ratio.clamp(0.0, 1.0);
            
            // Print current values when they change
            if ratio_changed {
                println!("Username X: {:.4}, Y: {:.4} | Password X: {:.4}, Y: {:.4}",
                    self.username_center_x_ratio,
                    self.username_center_y_ratio,
                    self.password_center_x_ratio,
                    self.password_center_y_ratio
                );
            }
        }

        // Update button states based on mouse position
        self.login_button.update();
        self.new_button.update();
        self.quit_button.update();

        // Handle button clicks
        if self.login_button.is_clicked() {
            info!("Login button clicked!");
            info!("Username: {}", self.username);
            info!("Password: {}", self.password);
            info!("Save ID checked: {}", self.save_id_checked);
            // TODO: Handle login action
        }

        if self.new_button.is_clicked() {
            info!("New Account button clicked!");
            // TODO: Handle new account action
        }

        if self.quit_button.is_clicked() {
            info!("Quit button clicked!");
            // TODO: Handle quit action
        }
    }

    pub fn draw(&self) {
        clear_background(Color::from_rgba(255, 255, 255, 255));

        if !self.loaded {
            // Show loading message
            let text = "Loading Login Screen...";
            let font_size = 32.0;
            let text_dimensions = measure_text(text, None, font_size as u16, 1.0);
            draw_text(
                text,
                screen_width() / 2.0 - text_dimensions.width / 2.0,
                screen_height() / 2.0,
                font_size,
                DARKGRAY,
            );
            return;
        }

        // Draw background - centered on screen (if loaded)
        let (bg_x, bg_y, bg_width, bg_height) = if let Some(bg) = &self.background {
            let bg_width = bg.texture.width();
            let bg_height = bg.texture.height();
            let bg_x = (screen_width() - bg_width) / 2.0;
            let bg_y = (screen_height() - bg_height) / 2.0;
            draw_texture(&bg.texture, bg_x, bg_y, WHITE);
            (bg_x, bg_y, bg_width, bg_height)
        } else {
            // Draw a simple gradient background if no background loaded
            draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::from_rgba(230, 240, 250, 255));
            (0.0, 0.0, 0.0, 0.0)
        };

        // Calculate input field positions relative to background dimensions
        // Use stored ratios (can be adjusted in debug mode with F1)
        let username_center_x = bg_x + bg_width * self.username_center_x_ratio;
        let username_center_y = bg_y + bg_height * self.username_center_y_ratio;
        let password_center_x = bg_x + bg_width * self.password_center_x_ratio;
        let password_center_y = bg_y + bg_height * self.password_center_y_ratio;

        // Draw MapleID input field texture
        // Position it so the texture's origin aligns with the center of the white input box
        if let Some(field) = &self.maple_id_label {
            // Calculate where to draw the texture so its origin is at the center position
            let draw_x = username_center_x - field.origin.x;
            let draw_y = username_center_y - field.origin.y;
            draw_texture(&field.texture, draw_x, draw_y, WHITE);
            
            // Debug: Draw a small red dot at the center position to help with alignment
            draw_circle(username_center_x, username_center_y, 3.0, RED);
        }
        
        // Store field positions for text rendering (using center as reference)
        let username_field_x = username_center_x;
        let username_field_y = username_center_y;

        // Overlay username text on top of MapleID texture
        draw_text(
            &self.username,
            username_field_x + 5.0,
            username_field_y + 16.0,
            16.0,
            BLACK,
        );

        // Draw PW input field texture
        if let Some(field) = &self.password_label {
            // Position the texture so its origin is at the center of the white input box
            let draw_x = password_center_x - field.origin.x;
            let draw_y = password_center_y - field.origin.y;
            draw_texture(&field.texture, draw_x, draw_y, WHITE);
            
            // Debug: Draw a small red dot at the center position to help with alignment
            draw_circle(password_center_x, password_center_y, 3.0, RED);
        }
        
        // Store field positions for text rendering (using center as reference)
        let password_field_x = password_center_x;
        let password_field_y = password_center_y;

        // Overlay password text (masked) on top of PW texture
        let password_masked: String = self.password.chars().map(|_| '*').collect();
        draw_text(
            &password_masked,
            password_field_x + 5.0,
            password_field_y + 16.0,
            16.0,
            BLACK,
        );

        // Draw "Save ID" checkbox (if available in assets)
        // Position checkbox relative to background dimensions
        let checkbox_anchor_x = bg_x + bg_width * 0.28;
        let checkbox_anchor_y = bg_y + bg_height * 0.91;

        if let (Some(unchecked), Some(checked)) = (&self.checkbox_unchecked, &self.checkbox_checked) {
            let checkbox_tex = if self.save_id_checked { checked } else { unchecked };
            draw_texture(
                &checkbox_tex.texture,
                checkbox_anchor_x - checkbox_tex.origin.x,
                checkbox_anchor_y - checkbox_tex.origin.y,
                WHITE
            );
        }

        // Draw buttons
        self.login_button.draw();
        self.new_button.draw();
        self.quit_button.draw();

        // Draw debug info in top-left corner
        draw_text(
            &format!("Mouse: {:.0}, {:.0}", mouse_position().0, mouse_position().1),
            10.0,
            20.0,
            16.0,
            DARKGRAY,
        );
        draw_text(
            "Login Screen Ready",
            10.0,
            40.0,
            16.0,
            DARKGRAY,
        );
        
        // Draw debug fine-tuning info if enabled
        if self.debug_mode {
            let y_offset = 60.0;
            let line_height = 20.0;
            
            draw_text(
                "=== FINE-TUNING MODE (F1 to toggle) ===",
                10.0,
                y_offset,
                16.0,
                YELLOW,
            );
            
            draw_text(
                "1-4: Select ratio | Arrow/WASD: Adjust | Shift: Fine",
                10.0,
                y_offset + line_height,
                14.0,
                LIGHTGRAY,
            );
            
            let ratio_names = ["Username X", "Username Y", "Password X", "Password Y"];
            let ratios = [
                self.username_center_x_ratio,
                self.username_center_y_ratio,
                self.password_center_x_ratio,
                self.password_center_y_ratio,
            ];
            
            for i in 0..4 {
                let color = if i == self.selected_ratio { GREEN } else { WHITE };
                let marker = if i == self.selected_ratio { ">>> " } else { "    " };
                draw_text(
                    &format!("{}{}: {:.4}", marker, ratio_names[i], ratios[i]),
                    10.0,
                    y_offset + line_height * 2.0 + (i as f32 * line_height),
                    14.0,
                    color,
                );
            }
            
            draw_text(
                "Copy these values to code:",
                10.0,
                y_offset + line_height * 6.0,
                14.0,
                LIGHTGRAY,
            );
            draw_text(
                &format!("username_center_x_ratio: {:.4}", self.username_center_x_ratio),
                10.0,
                y_offset + line_height * 7.0,
                12.0,
                Color::from_rgba(0, 255, 255, 255), // Cyan
            );
            draw_text(
                &format!("username_center_y_ratio: {:.4}", self.username_center_y_ratio),
                10.0,
                y_offset + line_height * 8.0,
                12.0,
                Color::from_rgba(0, 255, 255, 255), // Cyan
            );
            draw_text(
                &format!("password_center_x_ratio: {:.4}", self.password_center_x_ratio),
                10.0,
                y_offset + line_height * 9.0,
                12.0,
                Color::from_rgba(0, 255, 255, 255), // Cyan
            );
            draw_text(
                &format!("password_center_y_ratio: {:.4}", self.password_center_y_ratio),
                10.0,
                y_offset + line_height * 10.0,
                12.0,
                Color::from_rgba(0, 255, 255, 255), // Cyan
            );
        }
    }
}

/// Convert KeyCode to character, considering shift state
fn key_to_char(key: KeyCode, shift: bool) -> Option<char> {
    match key {
        KeyCode::A => Some(if shift { 'A' } else { 'a' }),
        KeyCode::B => Some(if shift { 'B' } else { 'b' }),
        KeyCode::C => Some(if shift { 'C' } else { 'c' }),
        KeyCode::D => Some(if shift { 'D' } else { 'd' }),
        KeyCode::E => Some(if shift { 'E' } else { 'e' }),
        KeyCode::F => Some(if shift { 'F' } else { 'f' }),
        KeyCode::G => Some(if shift { 'G' } else { 'g' }),
        KeyCode::H => Some(if shift { 'H' } else { 'h' }),
        KeyCode::I => Some(if shift { 'I' } else { 'i' }),
        KeyCode::J => Some(if shift { 'J' } else { 'j' }),
        KeyCode::K => Some(if shift { 'K' } else { 'k' }),
        KeyCode::L => Some(if shift { 'L' } else { 'l' }),
        KeyCode::M => Some(if shift { 'M' } else { 'm' }),
        KeyCode::N => Some(if shift { 'N' } else { 'n' }),
        KeyCode::O => Some(if shift { 'O' } else { 'o' }),
        KeyCode::P => Some(if shift { 'P' } else { 'p' }),
        KeyCode::Q => Some(if shift { 'Q' } else { 'q' }),
        KeyCode::R => Some(if shift { 'R' } else { 'r' }),
        KeyCode::S => Some(if shift { 'S' } else { 's' }),
        KeyCode::T => Some(if shift { 'T' } else { 't' }),
        KeyCode::U => Some(if shift { 'U' } else { 'u' }),
        KeyCode::V => Some(if shift { 'V' } else { 'v' }),
        KeyCode::W => Some(if shift { 'W' } else { 'w' }),
        KeyCode::X => Some(if shift { 'X' } else { 'x' }),
        KeyCode::Y => Some(if shift { 'Y' } else { 'y' }),
        KeyCode::Z => Some(if shift { 'Z' } else { 'z' }),
        KeyCode::Key0 => Some(if shift { ')' } else { '0' }),
        KeyCode::Key1 => Some(if shift { '!' } else { '1' }),
        KeyCode::Key2 => Some(if shift { '@' } else { '2' }),
        KeyCode::Key3 => Some(if shift { '#' } else { '3' }),
        KeyCode::Key4 => Some(if shift { '$' } else { '4' }),
        KeyCode::Key5 => Some(if shift { '%' } else { '5' }),
        KeyCode::Key6 => Some(if shift { '^' } else { '6' }),
        KeyCode::Key7 => Some(if shift { '&' } else { '7' }),
        KeyCode::Key8 => Some(if shift { '*' } else { '8' }),
        KeyCode::Key9 => Some(if shift { '(' } else { '9' }),
        KeyCode::Space => Some(' '),
        KeyCode::Minus => Some(if shift { '_' } else { '-' }),
        KeyCode::Equal => Some(if shift { '+' } else { '=' }),
        KeyCode::Period => Some(if shift { '>' } else { '.' }),
        KeyCode::Comma => Some(if shift { '<' } else { ',' }),
        _ => None,
    }
}

/// Run the login screen loop
pub async fn run_login_loop() {
    let mut login_state = LoginState::new();

    // Load assets before entering the loop
    login_state.load_assets().await;

    loop {
        let dt = get_frame_time();

        login_state.update(dt);
        login_state.draw();

        next_frame().await;
    }
}
