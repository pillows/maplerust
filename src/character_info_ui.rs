use macroquad::prelude::*;
use crate::assets::AssetManager;
use crate::character::CharacterData;
use std::collections::HashMap;
use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzReader, WzNodeCast};

const STATUSBAR_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/UI/StatusBar2.img";
const STATUSBAR_CACHE: &str = "/01/UI/StatusBar2.img";

/// Structure to hold texture with its origin point
struct TextureWithOrigin {
    texture: Texture2D,
    origin: Vec2,
}

/// Button state for UI interactions
#[derive(PartialEq, Clone, Copy, Default)]
enum ButtonState {
    #[default]
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
        let texture = match self.state {
            ButtonState::MouseOver if self.mouse_over.is_some() => &self.mouse_over,
            ButtonState::Pressed if self.pressed.is_some() => &self.pressed,
            ButtonState::Disabled if self.disabled.is_some() => &self.disabled,
            _ => &self.normal,
        };

        if let Some(tex) = texture {
            draw_texture(
                &tex.texture,
                self.x - tex.origin.x,
                self.y - tex.origin.y,
                WHITE,
            );
        }
    }
}

impl Default for Button {
    fn default() -> Self {
        Self::new(0.0, 0.0)
    }
}

/// Chat message structure
struct ChatMessage {
    target: String,  // "all", "party", etc.
    sender: String,
    text: String,
    timestamp: f64,
}

/// Chat state management
struct ChatState {
    messages: Vec<ChatMessage>,
    input_buffer: String,
    scroll_offset: usize,
    max_visible_lines: usize,
}

impl ChatState {
    fn new() -> Self {
        Self {
            messages: Vec::new(),
            input_buffer: String::new(),
            scroll_offset: 0,
            max_visible_lines: 5,
        }
    }
}

/// Main status bar UI structure
pub struct StatusBarUI {
    // Background layers
    background: Option<TextureWithOrigin>,
    lv_backtrnd: Option<TextureWithOrigin>,
    lv_cover: Option<TextureWithOrigin>,
    gauge_backgrd: Option<TextureWithOrigin>,
    gauge_cover: Option<TextureWithOrigin>,

    // Chat elements
    chat_space: Option<TextureWithOrigin>,
    chat_space2: Option<TextureWithOrigin>,
    chat_cover: Option<TextureWithOrigin>,
    chat_open_button: Button,
    chat_close_button: Button,
    scroll_up: Button,
    scroll_down: Button,

    // Chat targets (expedition, association, guild, party, friend, all, base)
    chat_targets: HashMap<String, TextureWithOrigin>,
    current_chat_target: String,

    // Buttons
    bt_chat: Button,
    bt_claim: Button,
    bt_character: Button,
    bt_stat: Button,
    bt_quest: Button,
    notice: Option<TextureWithOrigin>,

    // Level number sprites (0-9)
    lv_numbers: Vec<TextureWithOrigin>,

    // Gauges (hp, mp, exp) with animation frames
    gauge_hp: Vec<TextureWithOrigin>,
    gauge_mp: Vec<TextureWithOrigin>,
    gauge_exp: Vec<TextureWithOrigin>,

    // Chat state
    chat_state: ChatState,

    // UI state
    is_chat_open: bool,
    position: Vec2,  // Bottom-left anchor
    loaded: bool,

    // Animation
    gauge_frame: usize,
    gauge_timer: f32,
    caret_timer: f32,  // For blinking caret
    caret_visible: bool,
}

impl StatusBarUI {
    pub fn new() -> Self {
        Self {
            background: None,
            lv_backtrnd: None,
            lv_cover: None,
            gauge_backgrd: None,
            gauge_cover: None,
            chat_space: None,
            chat_space2: None,
            chat_cover: None,
            chat_open_button: Button::new(0.0, 0.0),
            chat_close_button: Button::new(0.0, 0.0),
            scroll_up: Button::new(0.0, 0.0),
            scroll_down: Button::new(0.0, 0.0),
            chat_targets: HashMap::new(),
            current_chat_target: "all".to_string(),
            bt_chat: Button::new(0.0, 0.0),
            bt_claim: Button::new(0.0, 0.0),
            bt_character: Button::new(0.0, 0.0),
            bt_stat: Button::new(0.0, 0.0),
            bt_quest: Button::new(0.0, 0.0),
            notice: None,
            lv_numbers: Vec::new(),
            gauge_hp: Vec::new(),
            gauge_mp: Vec::new(),
            gauge_exp: Vec::new(),
            chat_state: ChatState::new(),
            is_chat_open: true,
            position: Vec2::new(0.0, 0.0),
            loaded: false,
            gauge_frame: 0,
            gauge_timer: 0.0,
            caret_timer: 0.0,
            caret_visible: true,
        }
    }

    /// Load all status bar assets from WZ
    pub async fn load_assets(&mut self) {
        info!("Loading status bar UI...");

        match Self::load_statusbar_from_wz().await {
            Ok(ui_data) => {
                // Unpack loaded data
                self.background = ui_data.background;
                self.lv_backtrnd = ui_data.lv_backtrnd;
                self.lv_cover = ui_data.lv_cover;
                self.gauge_backgrd = ui_data.gauge_backgrd;
                self.gauge_cover = ui_data.gauge_cover;
                self.chat_space = ui_data.chat_space;
                self.chat_space2 = ui_data.chat_space2;
                self.chat_cover = ui_data.chat_cover;
                self.chat_targets = ui_data.chat_targets;
                self.lv_numbers = ui_data.lv_numbers;
                self.gauge_hp = ui_data.gauge_hp;
                self.gauge_mp = ui_data.gauge_mp;
                self.gauge_exp = ui_data.gauge_exp;
                self.notice = ui_data.notice;

                // Set up buttons with loaded textures
                self.chat_open_button = ui_data.chat_open_button;
                self.chat_close_button = ui_data.chat_close_button;
                self.scroll_up = ui_data.scroll_up;
                self.scroll_down = ui_data.scroll_down;
                self.bt_chat = ui_data.bt_chat;
                self.bt_claim = ui_data.bt_claim;
                self.bt_character = ui_data.bt_character;
                self.bt_stat = ui_data.bt_stat;
                self.bt_quest = ui_data.bt_quest;

                self.loaded = true;
                info!("✓ Status bar UI loaded successfully");
            }
            Err(e) => {
                error!("Failed to load status bar UI: {}", e);
                self.loaded = false;
            }
        }
    }

    /// Load status bar from WZ file - returns a temporary structure with all loaded data
    async fn load_statusbar_from_wz() -> Result<StatusBarData, String> {
        // Fetch the WZ file
        let bytes = AssetManager::fetch_and_cache(STATUSBAR_URL, STATUSBAR_CACHE).await?;

        // Parse WZ file
        let wz_iv = guess_iv_from_wz_img(&bytes)
            .ok_or_else(|| "Unable to guess version from StatusBar2.img".to_string())?;

        let byte_len = bytes.len();
        let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
        let cache_name_ref: wz_reader::WzNodeName = STATUSBAR_CACHE.to_string().into();
        let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
        let root_node: WzNodeArc = WzNode::new(&STATUSBAR_CACHE.to_string().into(), wz_image, None).into();

        root_node.write().unwrap().parse(&root_node)
            .map_err(|e| format!("Failed to parse StatusBar2.img: {:?}", e))?;

        // Load all UI elements
        let mut data = StatusBarData::default();

        // Load backgrounds
        data.background = Self::load_texture(&root_node, "background").await.ok();
        data.lv_backtrnd = Self::load_texture(&root_node, "lvBacktrnd").await.ok();
        data.lv_cover = Self::load_texture(&root_node, "lvCover").await.ok();
        data.gauge_backgrd = Self::load_texture(&root_node, "gaugeBackgrd").await.ok();
        data.gauge_cover = Self::load_texture(&root_node, "gaugeCover").await.ok();
        data.chat_space = Self::load_texture(&root_node, "chatSpace").await.ok();
        data.chat_space2 = Self::load_texture(&root_node, "chatSpace2").await.ok();
        data.chat_cover = Self::load_texture(&root_node, "chatCover").await.ok();
        data.notice = Self::load_texture(&root_node, "notice").await.ok();

        // Load chat targets
        for target in ["expedition", "association", "guild", "party", "friend", "all", "base"] {
            let path = format!("chatTarget/{}", target);
            if let Ok(texture) = Self::load_texture(&root_node, &path).await {
                data.chat_targets.insert(target.to_string(), texture);
            }
        }

        // Load level numbers (0-9)
        for num in 0..=9 {
            let path = format!("lvNumber/{}", num);
            if let Ok(texture) = Self::load_texture(&root_node, &path).await {
                data.lv_numbers.push(texture);
            }
        }

        // Load gauges
        data.gauge_hp = Self::load_gauge_animation(&root_node, "gauge/hp").await?;
        data.gauge_mp = Self::load_gauge_animation(&root_node, "gauge/mp").await?;
        data.gauge_exp = Self::load_gauge_animation(&root_node, "gauge/exp").await?;

        // Load buttons
        data.chat_open_button = Self::load_button(&root_node, "chatOpen", 0.0, 0.0).await?;
        data.chat_close_button = Self::load_button(&root_node, "chatClose", 0.0, 0.0).await?;
        data.scroll_up = Self::load_button(&root_node, "scrollUp", 0.0, 0.0).await?;
        data.scroll_down = Self::load_button(&root_node, "scrollDown", 0.0, 0.0).await?;
        data.bt_chat = Self::load_button(&root_node, "BtChat", 0.0, 0.0).await?;
        data.bt_claim = Self::load_button(&root_node, "BtClaim", 0.0, 0.0).await?;
        data.bt_character = Self::load_button(&root_node, "BtCharacter", 0.0, 0.0).await?;
        data.bt_stat = Self::load_button(&root_node, "BtStat", 0.0, 0.0).await?;
        data.bt_quest = Self::load_button(&root_node, "BtQuest", 0.0, 0.0).await?;

        Ok(data)
    }

    /// Load a single texture from a WZ node path
    async fn load_texture(root_node: &WzNodeArc, path: &str) -> Result<TextureWithOrigin, String> {
        let node = root_node
            .read()
            .unwrap()
            .at_path_parsed(path)
            .map_err(|e| format!("Path '{}' not found: {:?}", path, e))?;

        let node_read = node.read().unwrap();

        // Parse the node to ensure PNG is loaded
        drop(node_read);
        node.write().unwrap().parse(&node)
            .map_err(|e| format!("Failed to parse node at '{}': {:?}", path, e))?;

        let node_read = node.read().unwrap();

        // Get PNG data
        let png = node_read.try_as_png()
            .ok_or_else(|| format!("Node at '{}' is not a PNG", path))?;

        let png_data = png.extract_png()
            .map_err(|e| format!("Failed to extract PNG at '{}': {:?}", path, e))?;

        // Convert DynamicImage to RGBA8 bytes
        let rgba_img = png_data.to_rgba8();
        let width = rgba_img.width() as u16;
        let height = rgba_img.height() as u16;
        let bytes = rgba_img.into_raw();

        // Create texture
        let texture = Texture2D::from_rgba8(width, height, &bytes);

        // Get origin
        let origin = if let Some(origin_node) = node_read.children.get("origin") {
            let origin_read = origin_node.read().unwrap();
            if let Some(vec) = origin_read.try_as_vector2d() {
                Vec2::new(vec.0 as f32, vec.1 as f32)
            } else {
                Vec2::ZERO
            }
        } else {
            Vec2::ZERO
        };

        Ok(TextureWithOrigin { texture, origin })
    }

    /// Load a button with all its states
    async fn load_button(root_node: &WzNodeArc, button_path: &str, x: f32, y: f32) -> Result<Button, String> {
        let mut button = Button::new(x, y);

        // Load button states
        for state in ["normal", "pressed", "disabled", "mouseOver"] {
            let path = format!("{}/{}/0", button_path, state);
            if let Ok(texture) = Self::load_texture(root_node, &path).await {
                button.width = texture.texture.width();
                button.height = texture.texture.height();

                match state {
                    "normal" => button.normal = Some(texture),
                    "mouseOver" => button.mouse_over = Some(texture),
                    "pressed" => button.pressed = Some(texture),
                    "disabled" => button.disabled = Some(texture),
                    _ => {}
                }
            }
        }

        Ok(button)
    }

    /// Load gauge animation frames
    async fn load_gauge_animation(root_node: &WzNodeArc, gauge_path: &str) -> Result<Vec<TextureWithOrigin>, String> {
        let mut frames = Vec::new();

        for i in 0..100 {  // Try up to 100 frames
            let path = format!("{}/{}", gauge_path, i);
            match Self::load_texture(root_node, &path).await {
                Ok(texture) => frames.push(texture),
                Err(_) => break,  // No more frames
            }
        }

        if frames.is_empty() {
            Err(format!("No frames found for gauge at '{}'", gauge_path))
        } else {
            Ok(frames)
        }
    }

    /// Update status bar (animations, input, buttons)
    pub fn update(&mut self, dt: f32, character: &CharacterData) {
        if !self.loaded {
            return;
        }

        // Update position based on screen size
        self.position = Vec2::new(0.0, screen_height());

        // Update gauge animation
        self.gauge_timer += dt;
        if self.gauge_timer >= 0.1 {  // 10 FPS
            self.gauge_timer = 0.0;
            let max_frames = self.gauge_hp.len().max(self.gauge_mp.len()).max(self.gauge_exp.len());
            if max_frames > 0 {
                self.gauge_frame = (self.gauge_frame + 1) % max_frames;
            }
        }

        // Update caret blink
        self.caret_timer += dt;
        if self.caret_timer >= 0.5 {
            self.caret_timer = 0.0;
            self.caret_visible = !self.caret_visible;
        }

        // Update buttons
        self.chat_open_button.update();
        self.chat_close_button.update();
        self.scroll_up.update();
        self.scroll_down.update();
        self.bt_chat.update();
        self.bt_claim.update();
        self.bt_character.update();
        self.bt_stat.update();
        self.bt_quest.update();

        // Handle button clicks
        if self.chat_open_button.is_clicked() && !self.is_chat_open {
            self.is_chat_open = true;
        }
        if self.chat_close_button.is_clicked() && self.is_chat_open {
            self.is_chat_open = false;
        }

        if self.scroll_up.is_clicked() {
            self.scroll_up();
        }
        if self.scroll_down.is_clicked() {
            self.scroll_down();
        }

        // Handle chat input
        self.handle_chat_input(character);
    }

    /// Handle chat input
    fn handle_chat_input(&mut self, character: &CharacterData) {
        while let Some(character_typed) = get_char_pressed() {
            if character_typed == '\r' || character_typed == '\n' {
                self.send_message(character);
            } else if character_typed == '\x08' {  // Backspace
                self.chat_state.input_buffer.pop();
            } else if !character_typed.is_control() {
                self.chat_state.input_buffer.push(character_typed);
            }
        }
    }

    /// Send a chat message
    fn send_message(&mut self, character: &CharacterData) {
        if !self.chat_state.input_buffer.is_empty() {
            let message = ChatMessage {
                target: self.current_chat_target.clone(),
                sender: character.name.clone(),
                text: self.chat_state.input_buffer.clone(),
                timestamp: get_time(),
            };
            self.chat_state.messages.push(message);
            self.chat_state.input_buffer.clear();

            // Auto-scroll to bottom
            let max_scroll = self.chat_state.messages.len()
                .saturating_sub(self.chat_state.max_visible_lines);
            self.chat_state.scroll_offset = max_scroll;
        }
    }

    /// Scroll chat up
    fn scroll_up(&mut self) {
        if self.chat_state.scroll_offset > 0 {
            self.chat_state.scroll_offset -= 1;
        }
    }

    /// Scroll chat down
    fn scroll_down(&mut self) {
        let max_scroll = self.chat_state.messages.len()
            .saturating_sub(self.chat_state.max_visible_lines);
        if self.chat_state.scroll_offset < max_scroll {
            self.chat_state.scroll_offset += 1;
        }
    }

    /// Draw the status bar
    pub fn draw(&self, character: &CharacterData) {
        if !self.loaded {
            // Debug: show loading status
            draw_text("Status Bar Loading...", 10.0, screen_height() - 10.0, 16.0, RED);
            return;
        }

        let base_x = self.position.x;
        let base_y = self.position.y;

        // Debug: show that we're drawing
        // draw_text(&format!("Status Bar: ({}, {})", base_x, base_y), 10.0, screen_height() - 10.0, 12.0, GREEN);

        // Draw background layers
        if let Some(bg) = &self.background {
            draw_texture(&bg.texture, base_x - bg.origin.x, base_y - bg.origin.y, WHITE);
        }

        if let Some(lv_back) = &self.lv_backtrnd {
            draw_texture(&lv_back.texture, base_x - lv_back.origin.x, base_y - lv_back.origin.y, WHITE);
        }

        if let Some(gauge_bg) = &self.gauge_backgrd {
            draw_texture(&gauge_bg.texture, base_x - gauge_bg.origin.x, base_y - gauge_bg.origin.y, WHITE);
        }

        // Draw gauges (HP, MP, EXP)
        self.draw_gauge("hp", character.hp as f32 / character.hp as f32, base_x, base_y);  // 100% for testing
        self.draw_gauge("mp", character.mp as f32 / character.mp as f32, base_x, base_y);  // 100% for testing
        self.draw_gauge("exp", 1.0, base_x, base_y);  // 100% for testing

        if let Some(gauge_cov) = &self.gauge_cover {
            draw_texture(&gauge_cov.texture, base_x - gauge_cov.origin.x, base_y - gauge_cov.origin.y, WHITE);
        }

        if let Some(lv_cov) = &self.lv_cover {
            draw_texture(&lv_cov.texture, base_x - lv_cov.origin.x, base_y - lv_cov.origin.y, WHITE);
        }

        // Draw chat elements
        if self.is_chat_open {
            if let Some(chat_space) = &self.chat_space {
                draw_texture(&chat_space.texture, base_x - chat_space.origin.x, base_y - chat_space.origin.y, WHITE);
            }

            if let Some(chat_space2) = &self.chat_space2 {
                draw_texture(&chat_space2.texture, base_x - chat_space2.origin.x, base_y - chat_space2.origin.y, WHITE);

                // Draw chat input with caret
                self.draw_chat_input(base_x, base_y);
            }

            if let Some(chat_cover) = &self.chat_cover {
                draw_texture(&chat_cover.texture, base_x - chat_cover.origin.x, base_y - chat_cover.origin.y, WHITE);
            }

            // Draw chat messages
            self.draw_chat_messages(base_x, base_y);
        }

        // Draw chat target icon
        if let Some(target_tex) = self.chat_targets.get(&self.current_chat_target) {
            draw_texture(&target_tex.texture, base_x - target_tex.origin.x + 10.0, base_y - target_tex.origin.y - 100.0, WHITE);
        }

        // Draw buttons
        self.chat_open_button.draw();
        self.chat_close_button.draw();
        self.scroll_up.draw();
        self.scroll_down.draw();
        self.bt_chat.draw();
        self.bt_claim.draw();
        self.bt_character.draw();
        self.bt_stat.draw();
        self.bt_quest.draw();

        // Draw notice
        if let Some(notice) = &self.notice {
            draw_texture(&notice.texture, base_x - notice.origin.x + 300.0, base_y - notice.origin.y - 50.0, WHITE);
        }

        // Draw level number
        self.draw_level(character.level, base_x + 50.0, base_y - 50.0);
    }

    /// Draw a gauge (HP/MP/EXP)
    fn draw_gauge(&self, gauge_type: &str, percentage: f32, base_x: f32, base_y: f32) {
        let frames = match gauge_type {
            "hp" => &self.gauge_hp,
            "mp" => &self.gauge_mp,
            "exp" => &self.gauge_exp,
            _ => return,
        };

        if frames.is_empty() {
            return;
        }

        let frame_idx = self.gauge_frame % frames.len();
        let frame = &frames[frame_idx];

        // Calculate gauge width based on percentage
        let gauge_width = frame.texture.width() * percentage;

        // Draw with clipping to show fill percentage
        let x_offset = match gauge_type {
            "hp" => 100.0,
            "mp" => 100.0,
            "exp" => 100.0,
            _ => 0.0,
        };

        let y_offset = match gauge_type {
            "hp" => -90.0,
            "mp" => -70.0,
            "exp" => -50.0,
            _ => 0.0,
        };

        draw_texture_ex(
            &frame.texture,
            base_x - frame.origin.x + x_offset,
            base_y - frame.origin.y + y_offset,
            WHITE,
            DrawTextureParams {
                source: Some(Rect::new(0.0, 0.0, gauge_width, frame.texture.height())),
                ..Default::default()
            },
        );
    }

    /// Draw level number using sprite digits
    fn draw_level(&self, level: u32, x: f32, y: f32) {
        let level_str = level.to_string();
        let mut offset_x = x;

        for digit_char in level_str.chars() {
            if let Some(digit) = digit_char.to_digit(10) {
                if let Some(num_tex) = self.lv_numbers.get(digit as usize) {
                    draw_texture(
                        &num_tex.texture,
                        offset_x - num_tex.origin.x,
                        y - num_tex.origin.y,
                        WHITE,
                    );
                    offset_x += num_tex.texture.width() - 2.0;  // Small spacing
                }
            }
        }
    }

    /// Draw chat input field with caret
    fn draw_chat_input(&self, base_x: f32, base_y: f32) {
        let text_x = base_x + 50.0;
        let text_y = base_y - 20.0;
        let font_size = 12.0;

        // Draw input text
        draw_text(&self.chat_state.input_buffer, text_x, text_y, font_size, WHITE);

        // Draw blinking caret
        if self.caret_visible {
            let text_width = measure_text(&self.chat_state.input_buffer, None, font_size as u16, 1.0).width;
            draw_text("|", text_x + text_width, text_y, font_size, WHITE);
        }
    }

    /// Draw chat message history
    fn draw_chat_messages(&self, base_x: f32, base_y: f32) {
        let text_x = base_x + 10.0;
        let mut text_y = base_y - 120.0;
        let font_size = 11.0;
        let line_height = 15.0;

        let start_idx = self.chat_state.scroll_offset;
        let end_idx = (start_idx + self.chat_state.max_visible_lines).min(self.chat_state.messages.len());

        for i in start_idx..end_idx {
            if let Some(msg) = self.chat_state.messages.get(i) {
                let formatted = format!("[{}] {}: {}", msg.target, msg.sender, msg.text);
                draw_text(&formatted, text_x, text_y, font_size, WHITE);
                text_y += line_height;
            }
        }
    }

    /// Check if status bar is loaded
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }
}

impl Default for StatusBarUI {
    fn default() -> Self {
        Self::new()
    }
}

/// Temporary structure to hold loaded UI data
#[derive(Default)]
struct StatusBarData {
    background: Option<TextureWithOrigin>,
    lv_backtrnd: Option<TextureWithOrigin>,
    lv_cover: Option<TextureWithOrigin>,
    gauge_backgrd: Option<TextureWithOrigin>,
    gauge_cover: Option<TextureWithOrigin>,
    chat_space: Option<TextureWithOrigin>,
    chat_space2: Option<TextureWithOrigin>,
    chat_cover: Option<TextureWithOrigin>,
    chat_targets: HashMap<String, TextureWithOrigin>,
    lv_numbers: Vec<TextureWithOrigin>,
    gauge_hp: Vec<TextureWithOrigin>,
    gauge_mp: Vec<TextureWithOrigin>,
    gauge_exp: Vec<TextureWithOrigin>,
    notice: Option<TextureWithOrigin>,
    chat_open_button: Button,
    chat_close_button: Button,
    scroll_up: Button,
    scroll_down: Button,
    bt_chat: Button,
    bt_claim: Button,
    bt_character: Button,
    bt_stat: Button,
    bt_quest: Button,
}
