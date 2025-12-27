use macroquad::prelude::*;
use crate::assets::AssetManager;
use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzReader, WzObjectType, WzNodeCast};

const LOGIN_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/UI/Login.img";
const LOGIN_CACHE_NAME: &str = "Login.img";
const BACKGROUND_URL: &str = "https://scribbles-public.s3.amazonaws.com/tutorial/01/Map/Back/login.img";
const BACKGROUND_CACHE_NAME: &str = "login_back.img";

/// Structure to hold texture with its origin point
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
        // Don't update if disabled
        if self.state == ButtonState::Disabled {
            return;
        }

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
        // Don't register clicks if disabled
        if self.state == ButtonState::Disabled {
            return false;
        }

        // Check if button was just released while mouse is over it
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
            // Draw using origin offset
            let draw_x = self.x - two.origin.x;
            let draw_y = self.y - two.origin.y;
            draw_texture(&two.texture, draw_x, draw_y, WHITE);
        }
    }
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
    background: Option<TextureWithOrigin>,
    frame: Option<TextureWithOrigin>,
    signboard: Option<TextureWithOrigin>,
    id_label: Option<TextureWithOrigin>,
    pw_label: Option<TextureWithOrigin>,
    login_button: Button,

    // Input state
    username: String,
    password: String,
    focused_field: FocusedField,

    // Positioning offsets for fine-tuning
    id_offset_x: f32,
    id_offset_y: f32,
    pw_offset_x: f32,
    pw_offset_y: f32,
    button_offset_x: f32,
    button_offset_y: f32,

    loaded: bool,

    // Loading screen assets
    loading_background: Option<TextureWithOrigin>,
    loading_circle_frames: Vec<TextureWithOrigin>,
    loading_cancel_button: Button,
    showing_loading: bool,
    loading_animation_time: f32,
    loading_current_frame: usize,
}

impl LoginState {
    pub fn new() -> Self {
        Self {
            background: None,
            frame: None,
            signboard: None,
            id_label: None,
            pw_label: None,
            login_button: Button::new(400.0, 300.0),
            username: String::new(),
            password: String::new(),
            focused_field: FocusedField::None,
            id_offset_x: -100.0,
            id_offset_y: -51.0,
            pw_offset_x: -100.0,
            pw_offset_y: -25.0,
            button_offset_x: 68.0,
            button_offset_y: -51.0,
            loaded: false,
            loading_background: None,
            loading_circle_frames: Vec::new(),
            loading_cancel_button: Button::new(400.0, 300.0),
            showing_loading: false,
            loading_animation_time: 0.0,
            loading_current_frame: 0,
        }
    }

    /// Load all login screen assets from Login.img
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

        info!("WZ file parsed successfully");

        // Load frame asset
        match load_png_from_node(&root_node, "Common/frame") {
            Ok(two) => {
                info!("Frame loaded: {}x{}, origin: ({}, {})",
                    two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                self.frame = Some(two);
            }
            Err(e) => error!("Failed to load frame: {}", e),
        }

        // Load signboard asset
        match load_png_from_node(&root_node, "Title/signboard") {
            Ok(two) => {
                info!("Signboard loaded: {}x{}, origin: ({}, {})",
                    two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                self.signboard = Some(two);
            }
            Err(e) => error!("Failed to load signboard: {}", e),
        }

        // Load ID label
        match load_png_from_node(&root_node, "Title/ID") {
            Ok(two) => {
                info!("ID label loaded: {}x{}, origin: ({}, {})",
                    two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                self.id_label = Some(two);
            }
            Err(e) => error!("Failed to load ID label: {}", e),
        }

        // Load PW label
        match load_png_from_node(&root_node, "Title/PW") {
            Ok(two) => {
                info!("PW label loaded: {}x{}, origin: ({}, {})",
                    two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                self.pw_label = Some(two);
            }
            Err(e) => error!("Failed to load PW label: {}", e),
        }

        // Load login button states
        info!("Loading login button...");
        match load_png_from_node(&root_node, "Title/BtLogin/normal/0") {
            Ok(two) => {
                info!("Login button normal loaded: {}x{}, origin: ({}, {})",
                    two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                self.login_button.width = two.texture.width();
                self.login_button.height = two.texture.height();
                self.login_button.normal = Some(two);
            }
            Err(e) => error!("Failed to load login button normal: {}", e),
        }

        self.login_button.mouse_over = load_png_from_node(&root_node, "Title/BtLogin/mouseOver/0").ok();
        self.login_button.pressed = load_png_from_node(&root_node, "Title/BtLogin/pressed/0").ok();

        // Load Notice/Loading assets
        info!("Loading Notice/Loading assets...");

        // Load loading background
        match load_png_from_node(&root_node, "Notice/Loading/backgrnd") {
            Ok(two) => {
                info!("Loading background loaded: {}x{}, origin: ({}, {})",
                    two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                self.loading_background = Some(two);
            }
            Err(e) => error!("Failed to load loading background: {}", e),
        }

        // Load loading circle animation frames (0-15)
        for i in 0..16 {
            let path = format!("Notice/Loading/circle/{}", i);
            match load_png_from_node(&root_node, &path) {
                Ok(two) => {
                    info!("Loading circle frame {} loaded: {}x{}, origin: ({}, {})",
                        i, two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                    self.loading_circle_frames.push(two);
                }
                Err(e) => error!("Failed to load loading circle frame {}: {}", i, e),
            }
        }

        // Load loading cancel button states
        info!("Loading cancel button...");
        match load_png_from_node(&root_node, "Notice/Loading/BtCancel/normal/0") {
            Ok(two) => {
                info!("Cancel button normal loaded: {}x{}, origin: ({}, {})",
                    two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                self.loading_cancel_button.width = two.texture.width();
                self.loading_cancel_button.height = two.texture.height();
                self.loading_cancel_button.normal = Some(two);
            }
            Err(e) => error!("Failed to load cancel button normal: {}", e),
        }

        self.loading_cancel_button.mouse_over = load_png_from_node(&root_node, "Notice/Loading/BtCancel/mouseOver/0").ok();
        self.loading_cancel_button.pressed = load_png_from_node(&root_node, "Notice/Loading/BtCancel/pressed/0").ok();
        self.loading_cancel_button.disabled = load_png_from_node(&root_node, "Notice/Loading/BtCancel/disabled/0").ok();

        // Load background from login.img (Map/Back/login.img)
        info!("Loading background assets...");
        let bg_bytes = match AssetManager::fetch_and_cache(BACKGROUND_URL, BACKGROUND_CACHE_NAME).await {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Failed to fetch background: {}", e);
                self.loaded = true;
                return;
            }
        };

        info!("Parsing background (size: {} bytes)...", bg_bytes.len());

        let bg_wz_iv = match guess_iv_from_wz_img(&bg_bytes) {
            Some(iv) => iv,
            None => {
                error!("Unable to guess version from background IMG file");
                self.loaded = true;
                return;
            }
        };

        let bg_reader = Arc::new(WzReader::new(bg_bytes.clone()).with_iv(bg_wz_iv));
        let bg_wz_image = WzImage::new(&BACKGROUND_CACHE_NAME.into(), 0, bg_bytes.len(), &bg_reader);
        let bg_root_node: WzNodeArc = WzNode::new(&BACKGROUND_CACHE_NAME.into(), bg_wz_image, None).into();

        if let Err(e) = bg_root_node.write().unwrap().parse(&bg_root_node) {
            error!("Failed to parse background WZ root node: {:?}", e);
            self.loaded = true;
            return;
        }

        // Load background asset at back/11
        match load_png_from_node(&bg_root_node, "back/11") {
            Ok(two) => {
                info!("Background loaded: {}x{}, origin: ({}, {})",
                    two.texture.width(), two.texture.height(), two.origin.x, two.origin.y);
                self.background = Some(two);
            }
            Err(e) => error!("Failed to load background: {}", e),
        }

        self.loaded = true;
        info!("Login screen assets loaded successfully");
    }

    pub fn update(&mut self, dt: f32) {
        if !self.loaded {
            return;
        }

        // Update loading animation if showing
        if self.showing_loading {
            // Animate through circle frames (approximately 12 fps for smooth animation)
            const FRAME_DURATION: f32 = 1.0 / 12.0;
            self.loading_animation_time += dt;

            if self.loading_animation_time >= FRAME_DURATION {
                self.loading_animation_time -= FRAME_DURATION;
                self.loading_current_frame = (self.loading_current_frame + 1) % self.loading_circle_frames.len();
            }

            // Update cancel button position (centered relative to loading background)
            let center_x = screen_width() / 2.0;
            let center_y = screen_height() / 2.0;

            // Calculate loading background position to align cancel button properly
            if let Some(bg) = &self.loading_background {
                // Calculate where the loading background is actually drawn
                let bg_bottom_y = if bg.origin.x == 0.0 && bg.origin.y == 0.0 {
                    center_y - (bg.texture.height() / 2.0) + bg.texture.height()
                } else {
                    center_y - bg.origin.y + bg.texture.height()
                };

                // Position cancel button centered horizontally
                // Button draws at (x - origin.x, y - origin.y), so we need to account for that
                if let Some(btn_tex) = &self.loading_cancel_button.normal {
                    // To center the button: draw_x should be center_x - width/2
                    // Since draw_x = x - origin.x, we need: x - origin.x = center_x - width/2
                    // Therefore: x = center_x - width/2 + origin.x
                    self.loading_cancel_button.x = center_x - (btn_tex.texture.width() / 2.0) + btn_tex.origin.x;

                    // For Y: position INSIDE the background box, near the bottom
                    // Target draw position: bg_bottom_y - button_height - padding
                    let target_draw_y = bg_bottom_y - btn_tex.texture.height() - 10.0; // 10px padding from bottom
                    // Since draw_y = y - origin.y, we need: y = target_draw_y + origin.y
                    self.loading_cancel_button.y = target_draw_y + btn_tex.origin.y;
                } else {
                    self.loading_cancel_button.x = center_x;
                    self.loading_cancel_button.y = bg_bottom_y - 30.0;
                }
            } else {
                // Fallback if background not loaded
                self.loading_cancel_button.x = center_x;
                self.loading_cancel_button.y = center_y + 55.0;
            }

            // Update cancel button state
            self.loading_cancel_button.update();

            // Check if cancel button was clicked
            if self.loading_cancel_button.is_clicked() {
                info!("Cancel button clicked - dismissing loading screen");
                self.showing_loading = false;
                self.loading_cancel_button.state = ButtonState::Disabled;
            }

            // Don't process login UI interactions while loading
            return;
        }

        let center_x = screen_width() / 2.0;
        let center_y = screen_height() / 2.0;

        // Update button position relative to screen center
        self.login_button.x = center_x + self.button_offset_x;
        self.login_button.y = center_y + self.button_offset_y;

        // Update button state based on mouse position (only if not showing loading)
        if !self.showing_loading {
            self.login_button.update();
        }

        // Only handle input if not showing loading screen
        if self.showing_loading {
            return;
        }

        // Handle mouse clicks on input fields
        let (mouse_x, mouse_y) = mouse_position();

        // Calculate ID field bounds (align with signboard cutout)
        let id_field_bounds = if let Some(id_label) = &self.id_label {
            let draw_x = center_x - id_label.origin.x + self.id_offset_x;
            let draw_y = center_y - id_label.origin.y + self.id_offset_y;
            Some((draw_x, draw_y, id_label.texture.width(), id_label.texture.height()))
        } else {
            None
        };

        // Calculate PW field bounds (align with signboard cutout)
        let pw_field_bounds = if let Some(pw_label) = &self.pw_label {
            let draw_x = center_x - pw_label.origin.x + self.pw_offset_x;
            let draw_y = center_y - pw_label.origin.y + self.pw_offset_y;
            Some((draw_x, draw_y, pw_label.texture.width(), pw_label.texture.height()))
        } else {
            None
        };

        if is_mouse_button_pressed(MouseButton::Left) {
            // Check if clicked on ID field
            if let Some((x, y, w, h)) = id_field_bounds {
                if mouse_x >= x && mouse_x <= x + w && mouse_y >= y && mouse_y <= y + h {
                    info!("ID field clicked!");
                    self.focused_field = FocusedField::Username;
                }
            }

            // Check if clicked on PW field
            if let Some((x, y, w, h)) = pw_field_bounds {
                if mouse_x >= x && mouse_x <= x + w && mouse_y >= y && mouse_y <= y + h {
                    info!("PW field clicked!");
                    self.focused_field = FocusedField::Password;
                }
            }

            // Unfocus if clicked elsewhere
            let clicked_on_field = id_field_bounds.map_or(false, |(x, y, w, h)| {
                mouse_x >= x && mouse_x <= x + w && mouse_y >= y && mouse_y <= y + h
            }) || pw_field_bounds.map_or(false, |(x, y, w, h)| {
                mouse_x >= x && mouse_x <= x + w && mouse_y >= y && mouse_y <= y + h
            });

            if !clicked_on_field {
                self.focused_field = FocusedField::None;
            }
        }

        // Handle keyboard input
        if self.focused_field != FocusedField::None {
            // Handle backspace
            if is_key_pressed(KeyCode::Backspace) {
                match self.focused_field {
                    FocusedField::Username => { self.username.pop(); }
                    FocusedField::Password => { self.password.pop(); }
                    _ => {}
                }
            }

            // Handle Tab for field switching
            if is_key_pressed(KeyCode::Tab) {
                self.focused_field = match self.focused_field {
                    FocusedField::Username => FocusedField::Password,
                    FocusedField::Password => FocusedField::Username,
                    _ => FocusedField::Username,
                };
            }

            // Handle Enter for login
            if is_key_pressed(KeyCode::Enter) {
                if self.focused_field == FocusedField::Password {
                    info!("Login with username: {}", self.username);
                }
            }

            // Get character input
            if let Some(key) = get_last_key_pressed() {
                let is_shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
                if let Some(c) = key_to_char(key, is_shift) {
                    match self.focused_field {
                        FocusedField::Username => self.username.push(c),
                        FocusedField::Password => self.password.push(c),
                        _ => {}
                    }
                }
            }
        }

        // Handle button clicks
        if self.login_button.is_clicked() {
            info!("Login button clicked!");
            info!("Username: {}", self.username);
            info!("Password: {}", self.password);

            // Show loading screen
            self.showing_loading = true;
            self.loading_animation_time = 0.0;
            self.loading_current_frame = 0;

            // Reset cancel button to normal state
            self.loading_cancel_button.state = ButtonState::Normal;
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

        let center_x = screen_width() / 2.0;
        let center_y = screen_height() / 2.0;

        // Draw background first (so it appears behind everything)
        if let Some(bg) = &self.background {
            // Position background so its origin point is at screen center
            let draw_x = center_x - bg.origin.x;
            let draw_y = center_y - bg.origin.y;

            draw_texture(&bg.texture, draw_x, draw_y, WHITE);
        }

        // Draw frame centered on screen
        if let Some(frame) = &self.frame {
            // Position frame so its origin point is at screen center
            let draw_x = center_x - frame.origin.x;
            let draw_y = center_y - frame.origin.y;

            draw_texture(&frame.texture, draw_x, draw_y, WHITE);
        }

        // Draw signboard
        if let Some(signboard) = &self.signboard {
            // Position signboard so its origin point is at screen center
            let draw_x = center_x - signboard.origin.x;
            let draw_y = center_y - signboard.origin.y;

            draw_texture(&signboard.texture, draw_x, draw_y, WHITE);
        }

        // Draw ID label/input
        if let Some(id_label) = &self.id_label {
            let draw_x = center_x - id_label.origin.x + self.id_offset_x;
            let draw_y = center_y - id_label.origin.y + self.id_offset_y;

            // Show label when empty
            let show_label = self.username.is_empty();

            if show_label {
                draw_texture(&id_label.texture, draw_x, draw_y, WHITE);
            } else {
                // Draw username text input
                let font_size = 16.0;
                let texture_height = id_label.texture.height();
                let text_y = draw_y + (texture_height / 2.0) + (font_size / 3.0);

                draw_text(
                    &self.username,
                    draw_x + 5.0,
                    text_y,
                    font_size,
                    BLACK,
                );

                // Draw cursor when focused
                if self.focused_field == FocusedField::Username {
                    let text_width = measure_text(&self.username, None, font_size as u16, 1.0).width;
                    let cursor_x = draw_x + 5.0 + text_width;
                    draw_line(cursor_x, text_y - font_size * 0.75, cursor_x, text_y + font_size * 0.25, 2.0, BLACK);
                }
            }
        }

        // Draw PW label/input
        if let Some(pw_label) = &self.pw_label {
            let draw_x = center_x - pw_label.origin.x + self.pw_offset_x;
            let draw_y = center_y - pw_label.origin.y + self.pw_offset_y;

            // Show label when empty
            let show_label = self.password.is_empty();

            if show_label {
                draw_texture(&pw_label.texture, draw_x, draw_y, WHITE);
            } else {
                // Draw password text input (masked)
                let password_masked: String = self.password.chars().map(|_| '*').collect();
                let font_size = 16.0;
                let texture_height = pw_label.texture.height();
                let text_y = draw_y + (texture_height / 2.0) + (font_size / 3.0);

                draw_text(
                    &password_masked,
                    draw_x + 5.0,
                    text_y,
                    font_size,
                    BLACK,
                );

                // Draw cursor when focused
                if self.focused_field == FocusedField::Password {
                    let text_width = measure_text(&password_masked, None, font_size as u16, 1.0).width;
                    let cursor_x = draw_x + 5.0 + text_width;
                    draw_line(cursor_x, text_y - font_size * 0.75, cursor_x, text_y + font_size * 0.25, 2.0, BLACK);
                }
            }
        }

        // Draw login button
        self.login_button.draw();

        // Draw loading screen overlay if active (on top of everything)
        if self.showing_loading {
            // Draw loading background centered on screen
            if let Some(bg) = &self.loading_background {
                // Center the background: if origin is (0,0), use texture center instead
                let draw_x = if bg.origin.x == 0.0 && bg.origin.y == 0.0 {
                    center_x - (bg.texture.width() / 2.0)
                } else {
                    center_x - bg.origin.x
                };
                let draw_y = if bg.origin.x == 0.0 && bg.origin.y == 0.0 {
                    center_y - (bg.texture.height() / 2.0)
                } else {
                    center_y - bg.origin.y
                };
                draw_texture(&bg.texture, draw_x, draw_y, WHITE);
            }

            // Draw current circle animation frame ON TOP of loading background
            if !self.loading_circle_frames.is_empty() {
                let frame = &self.loading_circle_frames[self.loading_current_frame];
                // Center the circle: if origin is (0,0), use texture center instead
                let draw_x = if frame.origin.x == 0.0 && frame.origin.y == 0.0 {
                    center_x - (frame.texture.width() / 2.0)
                } else {
                    center_x - frame.origin.x
                };
                let draw_y = if frame.origin.x == 0.0 && frame.origin.y == 0.0 {
                    center_y - (frame.texture.height() / 2.0)
                } else {
                    center_y - frame.origin.y
                };
                draw_texture(&frame.texture, draw_x, draw_y, WHITE);
            }

            // Draw cancel button
            self.loading_cancel_button.draw();
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
