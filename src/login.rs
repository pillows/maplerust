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
        self.state == ButtonState::Pressed && is_mouse_button_released(MouseButton::Left)
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
        match load_png_from_node(&root_node, "Common/Banner/backBg") {
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

        // Handle mouse clicks on input fields
        let (mouse_x, mouse_y) = mouse_position();
        let input_x_base = center_x - 50.0;
        let input_width = 200.0;
        let input_height = 24.0;
        let username_anchor_y = center_y - 60.0;
        let password_anchor_y = center_y - 20.0;
        let checkbox_anchor_x = input_x_base - 80.0;
        let checkbox_anchor_y = center_y + 20.0;

        if is_mouse_button_pressed(MouseButton::Left) {
            // Check if clicked on username field
            if mouse_x >= input_x_base && mouse_x <= input_x_base + input_width
                && mouse_y >= username_anchor_y && mouse_y <= username_anchor_y + input_height
            {
                self.focused_field = FocusedField::Username;
            }
            // Check if clicked on password field
            else if mouse_x >= input_x_base && mouse_x <= input_x_base + input_width
                && mouse_y >= password_anchor_y && mouse_y <= password_anchor_y + input_height
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

        // Update button states based on mouse position
        self.login_button.update();
        self.new_button.update();
        self.quit_button.update();

        // Handle button clicks
        if self.login_button.is_clicked() {
            info!("Login button clicked! Username: {}", self.username);
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
        if let Some(bg) = &self.background {
            let bg_x = (screen_width() - bg.texture.width()) / 2.0;
            let bg_y = (screen_height() - bg.texture.height()) / 2.0;
            draw_texture(&bg.texture, bg_x, bg_y, WHITE);
        } else {
            // Draw a simple gradient background if no background loaded
            draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::from_rgba(230, 240, 250, 255));
        }

        let center_x = screen_width() / 2.0;
        let center_y = screen_height() / 2.0;

        // Define anchor positions where we want elements to appear
        let input_x_base = center_x - 50.0;
        let input_width = 200.0;
        let input_height = 24.0;

        // Username row (top) - anchor point for this row
        let username_anchor_y = center_y - 60.0;

        // Draw MapleID label with proper origin offset
        if let Some(label) = &self.maple_id_label {
            // Position label to the left of input field
            let label_draw_x = input_x_base - label.texture.width() - 10.0 - label.origin.x;
            // Vertically center the label with the input field
            let label_visual_y = username_anchor_y + (input_height / 2.0) - (label.texture.height() / 2.0);
            let label_draw_y = label_visual_y - label.origin.y;
            draw_texture(&label.texture, label_draw_x, label_draw_y, WHITE);
        }

        // Username input field
        let username_focused = self.focused_field == FocusedField::Username;
        draw_rectangle(input_x_base, username_anchor_y, input_width, input_height, WHITE);
        draw_rectangle_lines(
            input_x_base, username_anchor_y, input_width, input_height, 2.0,
            if username_focused { Color::from_rgba(100, 150, 255, 255) } else { Color::from_rgba(100, 100, 100, 255) }
        );

        // Draw username text
        draw_text(
            &self.username,
            input_x_base + 5.0,
            username_anchor_y + 17.0,
            16.0,
            BLACK,
        );

        // Password row (bottom) - anchor point for this row
        let password_anchor_y = center_y - 20.0;

        // Draw Password label with proper origin offset
        if let Some(label) = &self.password_label {
            // Position label to the left of input field
            let label_draw_x = input_x_base - label.texture.width() - 10.0 - label.origin.x;
            // Vertically center the label with the input field
            let label_visual_y = password_anchor_y + (input_height / 2.0) - (label.texture.height() / 2.0);
            let label_draw_y = label_visual_y - label.origin.y;
            draw_texture(&label.texture, label_draw_x, label_draw_y, WHITE);
        }

        // Password input field
        let password_focused = self.focused_field == FocusedField::Password;
        draw_rectangle(input_x_base, password_anchor_y, input_width, input_height, WHITE);
        draw_rectangle_lines(
            input_x_base, password_anchor_y, input_width, input_height, 2.0,
            if password_focused { Color::from_rgba(100, 150, 255, 255) } else { Color::from_rgba(100, 100, 100, 255) }
        );

        // Draw password text (masked with asterisks)
        let password_masked: String = self.password.chars().map(|_| '*').collect();
        draw_text(
            &password_masked,
            input_x_base + 5.0,
            password_anchor_y + 17.0,
            16.0,
            BLACK,
        );

        // Draw "Save ID" checkbox and label
        let checkbox_anchor_x = input_x_base - 80.0;
        let checkbox_anchor_y = center_y + 20.0;

        if let (Some(unchecked), Some(checked)) = (&self.checkbox_unchecked, &self.checkbox_checked) {
            let checkbox_tex = if self.save_id_checked { checked } else { unchecked };
            // Apply origin offset for checkbox too
            draw_texture(
                &checkbox_tex.texture,
                checkbox_anchor_x - checkbox_tex.origin.x,
                checkbox_anchor_y - checkbox_tex.origin.y,
                WHITE
            );
        }

        draw_text(
            "Save ID",
            checkbox_anchor_x + 25.0,
            checkbox_anchor_y + 15.0,
            18.0,
            Color::from_rgba(80, 80, 80, 255),
        );

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
