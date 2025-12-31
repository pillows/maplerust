use macroquad::prelude::*;
use crate::assets::AssetManager;
use crate::character::CharacterData;
use std::collections::HashMap;
use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzReader, WzNodeCast};

// StatusBar2.img is the correct file (StatusBar3.img doesn't exist)
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

    /// Get the screen position of the button (top-left corner)
    fn get_screen_pos(&self) -> (f32, f32) {
        if let Some(tex) = &self.normal {
            (self.x - tex.origin.x, self.y - tex.origin.y)
        } else {
            (self.x, self.y)
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
    cursor_position: usize,  // Cursor position in input buffer
    scroll_offset: usize,
    max_visible_lines: usize,
    last_sent_message: Option<String>,  // Track last sent message for balloon display
}

impl ChatState {
    fn new() -> Self {
        Self {
            messages: Vec::new(),
            input_buffer: String::new(),
            cursor_position: 0,
            scroll_offset: 0,
            max_visible_lines: 5,
            last_sent_message: None,
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
    chat_enter: Option<TextureWithOrigin>,  // Chat input background when focused
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
    bt_cashshop: Button,
    bt_channel: Button,
    bt_equip: Button,
    bt_inven: Button,
    bt_keysetting: Button,
    bt_menu: Button,
    bt_system: Button,
    bt_skill: Button,
    bt_mts: Button,
    notice: Option<TextureWithOrigin>,

    // Level number sprites (0-9)
    lv_numbers: Vec<TextureWithOrigin>,

    // Gauge number sprites (0-9 and symbols for HP/MP/EXP display)
    gauge_numbers: HashMap<String, TextureWithOrigin>,

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

    // Debug: Gauge positioning mode
    gauge_edit_mode: bool,
    selected_gauge: Option<String>,  // "hp", "mp", or "exp"
    gauge_offsets: std::collections::HashMap<String, (f32, f32, f32)>,  // gauge_type -> (x, y, width)
    dragging_gauge: bool,
    drag_start: Vec2,

    // Chat focus state
    chat_focused: bool,
}

impl StatusBarUI {
    pub fn new() -> Self {
        let gauge_offsets = HashMap::new();
        // Gauge positions will be set based on gaugeBackgrd origin

        Self {
            background: None,
            lv_backtrnd: None,
            lv_cover: None,
            gauge_backgrd: None,
            gauge_cover: None,
            chat_space: None,
            chat_space2: None,
            chat_cover: None,
            chat_enter: None,
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
            bt_cashshop: Button::new(0.0, 0.0),
            bt_channel: Button::new(0.0, 0.0),
            bt_equip: Button::new(0.0, 0.0),
            bt_inven: Button::new(0.0, 0.0),
            bt_keysetting: Button::new(0.0, 0.0),
            bt_menu: Button::new(0.0, 0.0),
            bt_system: Button::new(0.0, 0.0),
            bt_skill: Button::new(0.0, 0.0),
            bt_mts: Button::new(0.0, 0.0),
            notice: None,
            lv_numbers: Vec::new(),
            gauge_numbers: HashMap::new(),
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
            gauge_edit_mode: false,
            selected_gauge: None,
            gauge_offsets,
            dragging_gauge: false,
            drag_start: Vec2::ZERO,
            chat_focused: false,
        }
    }

    /// Load all status bar assets from WZ
    pub async fn load_assets(&mut self) {
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
                self.gauge_numbers = ui_data.gauge_numbers;
                self.gauge_hp = ui_data.gauge_hp;
                self.gauge_mp = ui_data.gauge_mp;
                self.gauge_exp = ui_data.gauge_exp;
                self.notice = ui_data.notice;
                self.chat_enter = ui_data.chat_enter;

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
                self.bt_cashshop = ui_data.bt_cashshop;
                self.bt_channel = ui_data.bt_channel;
                self.bt_equip = ui_data.bt_equip;
                self.bt_inven = ui_data.bt_inven;
                self.bt_keysetting = ui_data.bt_keysetting;
                self.bt_menu = ui_data.bt_menu;
                self.bt_system = ui_data.bt_system;
                self.bt_skill = ui_data.bt_skill;
                self.bt_mts = ui_data.bt_mts;

                self.loaded = true;
            }
            Err(e) => {
                error!("StatusBar UI failed to load: {}", e);
                self.loaded = false;
            }
        }
    }

    /// Load status bar from WZ file - returns a temporary structure with all loaded data
    async fn load_statusbar_from_wz() -> Result<StatusBarData, String> {
        // Fetch the WZ file
        let bytes = AssetManager::fetch_and_cache(STATUSBAR_URL, STATUSBAR_CACHE).await
            .map_err(|e| format!("Failed to fetch StatusBar2.img: {}", e))?;

        // Parse WZ file
        let wz_iv = guess_iv_from_wz_img(&bytes)
            .ok_or_else(|| "Unable to guess WZ version from StatusBar2.img".to_string())?;

        let byte_len = bytes.len();
        let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
        let cache_name_ref: wz_reader::WzNodeName = STATUSBAR_CACHE.to_string().into();
        let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
        let root_node: WzNodeArc = WzNode::new(&STATUSBAR_CACHE.to_string().into(), wz_image, None).into();

        root_node.write().unwrap().parse(&root_node)
            .map_err(|e| format!("Failed to parse StatusBar2.img: {:?}", e))?;

        // Load all UI elements
        let mut data = StatusBarData::default();

        // Load backgrounds (optional textures) - all under mainBar/
        data.background = Self::load_texture(&root_node, "mainBar/backgrnd").await.ok();
        data.lv_backtrnd = Self::load_texture(&root_node, "mainBar/lvBacktrnd").await.ok();
        data.lv_cover = Self::load_texture(&root_node, "mainBar/lvCover").await.ok();
        data.gauge_backgrd = Self::load_texture(&root_node, "mainBar/gaugeBackgrd").await.ok();
        data.gauge_cover = Self::load_texture(&root_node, "mainBar/gaugeCover").await.ok();

        // Debug: Print background info
        if let Some(bg) = &data.background {
            info!("Background - width: {}, height: {}, origin: {:?}",
                bg.texture.width(), bg.texture.height(), bg.origin);
        }
        if let Some(lv_back) = &data.lv_backtrnd {
            info!("Level background - width: {}, height: {}, origin: {:?}",
                lv_back.texture.width(), lv_back.texture.height(), lv_back.origin);
        }
        if let Some(gauge_bg) = &data.gauge_backgrd {
            info!("Gauge background - width: {}, height: {}, origin: {:?}",
                gauge_bg.texture.width(), gauge_bg.texture.height(), gauge_bg.origin);
        }
        data.chat_space = Self::load_texture(&root_node, "mainBar/chatSpace").await.ok();
        data.chat_space2 = Self::load_texture(&root_node, "mainBar/chatSpace2").await.ok();
        data.chat_cover = Self::load_texture(&root_node, "mainBar/chatCover").await.ok();
        data.chat_enter = Self::load_texture(&root_node, "mainBar/chatEnter").await.ok();
        data.notice = Self::load_texture(&root_node, "mainBar/notice").await.ok();

        // Load chat targets (optional) - under mainBar/chatTarget/
        // Most are direct PNGs, except "base" which has button states
        for target in ["expedition", "association", "guild", "party", "friend", "all"] {
            let path = format!("mainBar/chatTarget/{}", target);
            if let Ok(texture) = Self::load_texture(&root_node, &path).await {
                data.chat_targets.insert(target.to_string(), texture);
            }
        }
        // "base" has button states, load normal/0
        if let Ok(texture) = Self::load_texture(&root_node, "mainBar/chatTarget/base/normal/0").await {
            data.chat_targets.insert("base".to_string(), texture);
        }

        // Load level numbers (0-9) - under mainBar/
        for num in 0..=9 {
            let path = format!("mainBar/lvNumber/{}", num);
            if let Ok(texture) = Self::load_texture(&root_node, &path).await {
                data.lv_numbers.push(texture);
            }
        }

        // Load gauge numbers (0-9 and symbols) - under mainBar/gauge/number/
        for num in 0..=9 {
            let path = format!("mainBar/gauge/number/{}", num);
            if let Ok(texture) = Self::load_texture(&root_node, &path).await {
                data.gauge_numbers.insert(num.to_string(), texture);
            }
        }
        // Load special symbols for gauge display
        for symbol in [".", "%", "[", "]", "\\"] {
            let path = format!("mainBar/gauge/number/{}", symbol);
            if let Ok(texture) = Self::load_texture(&root_node, &path).await {
                data.gauge_numbers.insert(symbol.to_string(), texture);
            }
        }

        // Debug: Explore gauge structure to find positioning info
        if let Ok(gauge_node) = root_node.read().unwrap().at_path_parsed("mainBar/gauge") {
            let gauge_read = gauge_node.read().unwrap();
            info!("mainBar/gauge children: {:?}", gauge_read.children.keys().collect::<Vec<_>>());

            // Check each gauge type for positioning data
            for gauge_type in ["hp", "mp", "exp"].iter() {
                if let Some(gauge_type_node) = gauge_read.children.get(*gauge_type) {
                    {
                        let gauge_type_read = gauge_type_node.read().unwrap();
                        info!("{} gauge children: {:?}", gauge_type, gauge_type_read.children.keys().collect::<Vec<_>>());

                        // Check for positioning data
                        for key in ["x", "y", "origin"].iter() {
                            if let Some(node) = gauge_type_read.children.get(*key) {
                                let node_read = node.read().unwrap();
                                if let Some(vec) = node_read.try_as_vector2d() {
                                    info!("{} gauge {} position: ({}, {})", gauge_type, key, vec.0, vec.1);
                                } else if let Some(val) = node_read.try_as_int() {
                                    info!("{} gauge {} value: {}", gauge_type, key, val);
                                }
                            }
                        }
                    }

                    // Check frame 0 for positioning
                    let frame_node_opt = {
                        let gauge_type_read = gauge_type_node.read().unwrap();
                        gauge_type_read.children.get("0").cloned()
                    };

                    if let Some(frame_node) = frame_node_opt {
                        frame_node.write().unwrap().parse(&frame_node).ok();
                        let frame_read = frame_node.read().unwrap();

                        if let Some(origin_node) = frame_read.children.get("origin") {
                            let origin_read = origin_node.read().unwrap();
                            if let Some(vec) = origin_read.try_as_vector2d() {
                                info!("{} gauge frame 0 origin: ({}, {})", gauge_type, vec.0, vec.1);
                            }
                        }
                    }
                }
            }
        }

        // Load gauges - under mainBar/gauge/
        data.gauge_hp = Self::load_gauge_animation(&root_node, "mainBar/gauge/hp").await?;
        data.gauge_mp = Self::load_gauge_animation(&root_node, "mainBar/gauge/mp").await?;
        data.gauge_exp = Self::load_gauge_animation(&root_node, "mainBar/gauge/exp").await?;

        // Debug: Print gauge info
        if let Some(hp_frame) = data.gauge_hp.first() {
            info!("HP gauge frame 0 - width: {}, height: {}, origin: {:?}",
                hp_frame.texture.width(), hp_frame.texture.height(), hp_frame.origin);
        }
        if let Some(mp_frame) = data.gauge_mp.first() {
            info!("MP gauge frame 0 - width: {}, height: {}, origin: {:?}",
                mp_frame.texture.width(), mp_frame.texture.height(), mp_frame.origin);
        }
        if let Some(exp_frame) = data.gauge_exp.first() {
            info!("EXP gauge frame 0 - width: {}, height: {}, origin: {:?}",
                exp_frame.texture.width(), exp_frame.texture.height(), exp_frame.origin);
        }

        // Load buttons - all under mainBar/
        // Try to load each button, logging errors but not failing the entire UI
        info!("Loading buttons...");

        data.chat_open_button = Self::load_button(&root_node, "mainBar/chatOpen", 0.0, 0.0).await
            .map_err(|e| { info!("chatOpen failed: {}", e); e })?;
        data.chat_close_button = Self::load_button(&root_node, "mainBar/chatClose", 0.0, 0.0).await
            .map_err(|e| { info!("chatClose failed: {}", e); e })?;
        data.scroll_up = Self::load_button(&root_node, "mainBar/scrollUp", 0.0, 0.0).await
            .map_err(|e| { info!("scrollUp failed: {}", e); e })?;
        data.scroll_down = Self::load_button(&root_node, "mainBar/scrollDown", 0.0, 0.0).await
            .map_err(|e| { info!("scrollDown failed: {}", e); e })?;

        info!("Loading main buttons...");
        data.bt_chat = Self::load_button(&root_node, "mainBar/BtChat", 0.0, 0.0).await
            .map_err(|e| { info!("BtChat failed: {}", e); e })?;
        data.bt_claim = Self::load_button(&root_node, "mainBar/BtClaim", 0.0, 0.0).await
            .map_err(|e| { info!("BtClaim failed: {}", e); e })?;
        data.bt_character = Self::load_button(&root_node, "mainBar/BtCharacter", 0.0, 0.0).await
            .map_err(|e| { info!("BtCharacter failed: {}", e); e })?;
        data.bt_stat = Self::load_button(&root_node, "mainBar/BtStat", 0.0, 0.0).await
            .map_err(|e| { info!("BtStat failed: {}", e); e })?;
        data.bt_quest = Self::load_button(&root_node, "mainBar/BtQuest", 0.0, 0.0).await
            .map_err(|e| { info!("BtQuest failed: {}", e); e })?;

        info!("Loading new buttons...");
        data.bt_cashshop = Self::load_button(&root_node, "mainBar/BtCashShop", 0.0, 0.0).await
            .map_err(|e| { info!("BtCashShop failed: {}", e); e })?;
        data.bt_channel = Self::load_button(&root_node, "mainBar/BtChannel", 0.0, 0.0).await
            .map_err(|e| { info!("BtChannel failed: {}", e); e })?;
        data.bt_equip = Self::load_button(&root_node, "mainBar/BtEquip", 0.0, 0.0).await
            .map_err(|e| { info!("BtEquip failed: {}", e); e })?;
        data.bt_inven = Self::load_button(&root_node, "mainBar/BtInven", 0.0, 0.0).await
            .map_err(|e| { info!("BtInven failed: {}", e); e })?;
        data.bt_keysetting = Self::load_button(&root_node, "mainBar/BtKeysetting", 0.0, 0.0).await
            .map_err(|e| { info!("BtKeysetting failed: {}", e); e })?;
        data.bt_menu = Self::load_button(&root_node, "mainBar/BtMenu", 0.0, 0.0).await
            .map_err(|e| { info!("BtMenu failed: {}", e); e })?;
        data.bt_system = Self::load_button(&root_node, "mainBar/BtSystem", 0.0, 0.0).await
            .map_err(|e| { info!("BtSystem failed: {}", e); e })?;
        data.bt_skill = Self::load_button(&root_node, "mainBar/BtSkill", 0.0, 0.0).await
            .map_err(|e| { info!("BtSkill failed: {}", e); e })?;
        data.bt_mts = Self::load_button(&root_node, "mainBar/BtMTS", 0.0, 0.0).await
            .map_err(|e| { info!("BtMTS failed: {}", e); e })?;

        info!("All buttons loaded successfully!");
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

    /// Load a button with all its states and try to get position from WZ
    async fn load_button(root_node: &WzNodeArc, button_path: &str, x: f32, y: f32) -> Result<Button, String> {
        let mut button = Button::new(x, y);

        // Try to get position from WZ data
        if let Ok(button_node) = root_node.read().unwrap().at_path_parsed(button_path) {
            let button_read = button_node.read().unwrap();

            // Check for x coordinate
            if let Some(x_node) = button_read.children.get("x") {
                if let Some(x_val) = x_node.read().unwrap().try_as_int() {
                    info!("{} has x position: {}", button_path, x_val);
                }
            }

            // Check for y coordinate
            if let Some(y_node) = button_read.children.get("y") {
                if let Some(y_val) = y_node.read().unwrap().try_as_int() {
                    info!("{} has y position: {}", button_path, y_val);
                }
            }
        }

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

        // Try loading frames
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

        // Handle chat focus with Enter key
        if is_key_pressed(KeyCode::Enter) {
            if !self.chat_focused {
                // Focus chat when unfocused
                self.chat_focused = true;
            } else if self.chat_state.input_buffer.is_empty() {
                // Unfocus chat when focused and no text
                self.chat_focused = false;
            }
            // If focused and has text, handle_chat_input will send the message
        }

        // Handle Escape to unfocus chat
        if is_key_pressed(KeyCode::Escape) && self.chat_focused {
            self.chat_focused = false;
            self.chat_state.input_buffer.clear();
            self.chat_state.cursor_position = 0;
        }

        // Handle chat target selection with keys 1-6 (only when chat is NOT focused)
        // When unfocused: pressing a number sets chat target AND focuses chat (shows chatEnter)
        // When focused: numbers are typed into the chat input
        if !self.chat_focused {
            let mut target_selected = false;
            if is_key_pressed(KeyCode::Key1) {
                self.current_chat_target = "expedition".to_string();
                target_selected = true;
            }
            if is_key_pressed(KeyCode::Key2) {
                self.current_chat_target = "association".to_string();
                target_selected = true;
            }
            if is_key_pressed(KeyCode::Key3) {
                self.current_chat_target = "guild".to_string();
                target_selected = true;
            }
            if is_key_pressed(KeyCode::Key4) {
                self.current_chat_target = "party".to_string();
                target_selected = true;
            }
            if is_key_pressed(KeyCode::Key5) {
                self.current_chat_target = "friend".to_string();
                target_selected = true;
            }
            if is_key_pressed(KeyCode::Key6) {
                self.current_chat_target = "all".to_string();
                target_selected = true;
            }
            
            // If a chat target was selected, also focus the chat to show chatEnter
            if target_selected {
                self.chat_focused = true;
            }
        }

        // Check for mouse click outside chat area to unfocus
        if self.chat_focused && is_mouse_button_pressed(MouseButton::Left) {
            let (mouse_x, mouse_y) = mouse_position();
            let screen_w = screen_width();
            let screen_h = screen_height();
            let base_x = screen_w / 2.0;
            let base_y = screen_h;

            // Define chat area bounds based on chatEnter origin (467, 58)
            // chatEnter is positioned at base_x - 467, base_y - 58
            let chat_left = base_x - 512.0;  // Left edge of chat area
            let chat_right = base_x - 467.0 + 500.0;  // Right edge
            let chat_top = base_y - 60.0;
            let chat_bottom = base_y - 40.0;

            // If clicked outside chat area, unfocus
            if mouse_x < chat_left || mouse_x > chat_right || mouse_y < chat_top || mouse_y > chat_bottom {
                self.chat_focused = false;
            }
        }

        // Handle gauge edit mode controls
        if self.gauge_edit_mode {
            self.handle_gauge_editing();
        }

        // Don't animate gauges - they were flashing due to animation
        // Keep gauge_frame at 0 for static display
        self.gauge_frame = 0;

        // Update caret blink
        self.caret_timer += dt;
        if self.caret_timer >= 0.5 {
            self.caret_timer = 0.0;
            self.caret_visible = !self.caret_visible;
        }

        // Update button positions relative to screen (for click detection)
        let screen_w = screen_width();
        let screen_h = screen_height();
        let base_x = screen_w / 2.0;
        let base_y = screen_h;

        // Update button positions - buttons are positioned using base_x/base_y
        self.bt_claim.x = base_x;
        self.bt_claim.y = base_y;

        self.bt_system.x = base_x;
        self.bt_system.y = base_y;

        self.bt_cashshop.x = base_x;
        self.bt_cashshop.y = base_y;

        self.bt_channel.x = base_x;
        self.bt_channel.y = base_y;

        self.bt_chat.x = base_x;
        self.bt_chat.y = base_y;

        self.bt_keysetting.x = base_x;
        self.bt_keysetting.y = base_y;

        self.bt_menu.x = base_x;
        self.bt_menu.y = base_y;

        self.bt_quest.x = base_x;
        self.bt_quest.y = base_y;

        self.bt_stat.x = base_x;
        self.bt_stat.y = base_y;

        self.bt_inven.x = base_x;
        self.bt_inven.y = base_y;

        self.bt_equip.x = base_x;
        self.bt_equip.y = base_y;

        self.bt_character.x = base_x;
        self.bt_character.y = base_y;

        self.bt_skill.x = base_x;
        self.bt_skill.y = base_y;

        self.bt_mts.x = base_x;
        self.bt_mts.y = base_y;

        self.chat_open_button.x = base_x;
        self.chat_open_button.y = base_y;

        self.chat_close_button.x = base_x;
        self.chat_close_button.y = base_y;

        self.scroll_up.x = base_x;
        self.scroll_up.y = base_y;

        self.scroll_down.x = base_x;
        self.scroll_down.y = base_y;

        // Now update all buttons
        self.chat_open_button.update();
        self.chat_close_button.update();
        self.scroll_up.update();
        self.scroll_down.update();
        self.bt_chat.update();
        self.bt_claim.update();
        self.bt_character.update();
        self.bt_stat.update();
        self.bt_quest.update();
        self.bt_cashshop.update();
        self.bt_channel.update();
        self.bt_equip.update();
        self.bt_inven.update();
        self.bt_keysetting.update();
        self.bt_menu.update();
        self.bt_system.update();
        self.bt_skill.update();
        self.bt_mts.update();

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

        // Handle chat input (only when chat is focused)
        if self.chat_focused {
            self.handle_chat_input(character);
        }
    }

    /// Handle gauge editing mode - keyboard and mouse controls
    fn handle_gauge_editing(&mut self) {
        use macroquad::prelude::KeyCode;

        // Keyboard: 1=HP, 2=MP, 3=EXP
        if is_key_pressed(KeyCode::Key1) {
            self.selected_gauge = Some("hp".to_string());
            info!("Selected HP gauge");
        }
        if is_key_pressed(KeyCode::Key2) {
            self.selected_gauge = Some("mp".to_string());
            info!("Selected MP gauge");
        }
        if is_key_pressed(KeyCode::Key3) {
            self.selected_gauge = Some("exp".to_string());
            info!("Selected EXP gauge");
        }

        // Arrow keys: fine adjustment (1px at a time)
        if let Some(ref gauge_name) = self.selected_gauge {
            if let Some((x, y, width)) = self.gauge_offsets.get_mut(gauge_name) {
                let mut changed = false;

                if is_key_down(KeyCode::Left) {
                    *x -= 1.0;
                    changed = true;
                }
                if is_key_down(KeyCode::Right) {
                    *x += 1.0;
                    changed = true;
                }
                if is_key_down(KeyCode::Up) {
                    *y -= 1.0;
                    changed = true;
                }
                if is_key_down(KeyCode::Down) {
                    *y += 1.0;
                    changed = true;
                }

                // Shift + Left/Right: adjust width
                if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
                    if is_key_down(KeyCode::Left) {
                        *width -= 1.0;
                        changed = true;
                    }
                    if is_key_down(KeyCode::Right) {
                        *width += 1.0;
                        changed = true;
                    }
                }

                if changed {
                    info!("{} gauge: x={}, y={}, width={}", gauge_name, x, y, width);
                }
            }
        }

        // P key: print all current positions
        if is_key_pressed(KeyCode::P) {
            info!("=== GAUGE POSITIONS ===");
            if let Some((x, y, w)) = self.gauge_offsets.get("hp") {
                info!("HP:  x={}, y={}, width={}", x, y, w);
            }
            if let Some((x, y, w)) = self.gauge_offsets.get("mp") {
                info!("MP:  x={}, y={}, width={}", x, y, w);
            }
            if let Some((x, y, w)) = self.gauge_offsets.get("exp") {
                info!("EXP: x={}, y={}, width={}", x, y, w);
            }
            info!("======================");
        }
    }

    /// Handle chat input
    fn handle_chat_input(&mut self, character: &CharacterData) {
        // Handle arrow keys for cursor navigation
        if is_key_pressed(KeyCode::Left) {
            if self.chat_state.cursor_position > 0 {
                self.chat_state.cursor_position -= 1;
            }
        }
        if is_key_pressed(KeyCode::Right) {
            if self.chat_state.cursor_position < self.chat_state.input_buffer.len() {
                self.chat_state.cursor_position += 1;
            }
        }
        
        // Handle Home/End keys
        if is_key_pressed(KeyCode::Home) {
            self.chat_state.cursor_position = 0;
        }
        if is_key_pressed(KeyCode::End) {
            self.chat_state.cursor_position = self.chat_state.input_buffer.len();
        }
        
        // Handle backspace key
        if is_key_pressed(KeyCode::Backspace) {
            if self.chat_state.cursor_position > 0 {
                self.chat_state.input_buffer.remove(self.chat_state.cursor_position - 1);
                self.chat_state.cursor_position -= 1;
            }
        }
        
        // Handle Delete key
        if is_key_pressed(KeyCode::Delete) {
            if self.chat_state.cursor_position < self.chat_state.input_buffer.len() {
                self.chat_state.input_buffer.remove(self.chat_state.cursor_position);
            }
        }
        
        // Handle character input
        // Process all characters that were pressed this frame
        let mut chars_processed = 0;
        while let Some(character_typed) = get_char_pressed() {
            chars_processed += 1;
            // Limit to prevent infinite loops
            if chars_processed > 100 {
                break;
            }
            
            if character_typed == '\r' || character_typed == '\n' {
                self.send_message(character);
            } else if character_typed == '\x08' {  // Backspace character
                if self.chat_state.cursor_position > 0 {
                    self.chat_state.input_buffer.remove(self.chat_state.cursor_position - 1);
                    self.chat_state.cursor_position -= 1;
                }
            } else if !character_typed.is_control() {
                // Insert character at cursor position
                self.chat_state.input_buffer.insert(self.chat_state.cursor_position, character_typed);
                self.chat_state.cursor_position += 1;
            }
        }
    }

    /// Send a chat message
    fn send_message(&mut self, character: &CharacterData) {
        if !self.chat_state.input_buffer.is_empty() {
            let message_text = self.chat_state.input_buffer.clone();
            let message = ChatMessage {
                target: self.current_chat_target.clone(),
                sender: character.name.clone(),
                text: message_text.clone(),
                timestamp: get_time(),
            };
            self.chat_state.messages.push(message);
            
            // Store the sent message for balloon display
            self.chat_state.last_sent_message = Some(message_text);
            
            self.chat_state.input_buffer.clear();

            // Auto-scroll to bottom
            let max_scroll = self.chat_state.messages.len()
                .saturating_sub(self.chat_state.max_visible_lines);
            self.chat_state.scroll_offset = max_scroll;
            
            // Unfocus chat after sending
            self.chat_focused = false;
            // Reset cursor position
            self.chat_state.cursor_position = 0;
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
            // Show a minimal UI if status bar failed to load
            let panel_height = 120.0;
            let panel_width = 300.0;
            let panel_x = 0.0;
            let panel_y = screen_height() - panel_height;

            // Background
            draw_rectangle(
                panel_x,
                panel_y,
                panel_width,
                panel_height,
                Color::from_rgba(0, 0, 0, 200),
            );

            // Character stats (simple text fallback)
            let mut y_offset = panel_y + 20.0;
            draw_text(&format!("{} (Lv.{})", character.name, character.level),
                     panel_x + 10.0, y_offset, 18.0, WHITE);
            y_offset += 25.0;
            draw_text(&format!("HP: {}/{}", character.hp, character.hp),
                     panel_x + 10.0, y_offset, 16.0, GREEN);
            y_offset += 25.0;
            draw_text(&format!("MP: {}/{}", character.mp, character.mp),
                     panel_x + 10.0, y_offset, 16.0, BLUE);
            y_offset += 25.0;
            draw_text("(Status bar UI failed to load)",
                     panel_x + 10.0, y_offset, 12.0, Color::from_rgba(255, 100, 100, 255));
            return;
        }

        // Position status bar at bottom-center of screen
        // StatusBar background is typically 1366px wide in MapleStory
        let screen_w = screen_width();
        let screen_h = screen_height();

        // Use origin points from the background texture to position correctly
        let base_x = screen_w / 2.0;  // Center horizontally
        let base_y = screen_h;  // Bottom of screen

        // Debug: show that we're drawing
        // draw_text(&format!("Status Bar: ({}, {})", base_x, base_y), 10.0, screen_height() - 10.0, 12.0, GREEN);

        // Draw background layers
        if let Some(bg) = &self.background {
            draw_texture(&bg.texture, base_x - bg.origin.x, base_y - bg.origin.y, WHITE);
        }

        if let Some(lv_back) = &self.lv_backtrnd {
            draw_texture(&lv_back.texture, base_x - lv_back.origin.x, base_y - lv_back.origin.y, WHITE);
        }

        // Get gauge background position to position gauges relative to it
        let gauge_bg_pos = if let Some(gauge_bg) = &self.gauge_backgrd {
            let gauge_bg_x = base_x - gauge_bg.origin.x;
            let gauge_bg_y = base_y - gauge_bg.origin.y;
            draw_texture(&gauge_bg.texture, gauge_bg_x, gauge_bg_y, WHITE);
            Some((gauge_bg_x, gauge_bg_y))
        } else {
            None
        };

        // Draw gauges (HP, MP, EXP) - pass base_x/base_y for positioning
        if gauge_bg_pos.is_some() {
            // HP and MP at 100%, EXP at 50%
            self.draw_gauge("hp", 1.0, base_x, base_y);
            self.draw_gauge("mp", 1.0, base_x, base_y);
            self.draw_gauge("exp", 0.5, base_x, base_y);  // 50% EXP

            // Draw HP/MP/EXP numbers
            self.draw_gauge_numbers("hp", character.hp, character.hp, base_x, base_y);
            self.draw_gauge_numbers("mp", character.mp, character.mp, base_x, base_y);
            // For EXP, show as percentage (0-100)
            let exp_percent = 50; // Placeholder
            self.draw_gauge_numbers("exp", exp_percent, 100, base_x, base_y);
        }

        if let Some(gauge_cov) = &self.gauge_cover {
            draw_texture(&gauge_cov.texture, base_x - gauge_cov.origin.x, base_y - gauge_cov.origin.y, WHITE);
        }

        if let Some(lv_cov) = &self.lv_cover {
            draw_texture(&lv_cov.texture, base_x - lv_cov.origin.x, base_y - lv_cov.origin.y, WHITE);
        }

        // Draw chat elements
        if self.is_chat_open {
            let chat_space_pos = if let Some(chat_space) = &self.chat_space {
                let chat_x = base_x - chat_space.origin.x;
                let chat_y = base_y - chat_space.origin.y;
                draw_texture(&chat_space.texture, chat_x, chat_y, WHITE);
                Some((chat_x, chat_y))
            } else {
                None
            };

            if let Some(chat_space2) = &self.chat_space2 {
                let chat_x = base_x - chat_space2.origin.x;
                let chat_y = base_y - chat_space2.origin.y;
                draw_texture(&chat_space2.texture, chat_x, chat_y, WHITE);
            }

            // Draw chat target icon
            if let Some(target_tex) = self.chat_targets.get(&self.current_chat_target) {
                draw_texture(&target_tex.texture, base_x - target_tex.origin.x, base_y - target_tex.origin.y, WHITE);
            }

            // Draw chatEnter FIRST (background for input area) when chat is focused
            if self.chat_focused {
                if let Some(chat_enter) = &self.chat_enter {
                    let enter_x = base_x - chat_enter.origin.x;
                    let enter_y = base_y - chat_enter.origin.y;
                    draw_texture(&chat_enter.texture, enter_x, enter_y, WHITE);
                }
            }

            // Draw chat input text AFTER chatEnter so text is visible on top
            let input_x = if let Some(chat_space) = &self.chat_space {
                base_x - chat_space.origin.x + 5.0
            } else {
                base_x - 512.0 + 5.0
            };
            let input_y = base_y - 46.0;
            self.draw_chat_input(input_x, input_y, self.chat_focused);

            if let Some(chat_cover) = &self.chat_cover {
                draw_texture(&chat_cover.texture, base_x - chat_cover.origin.x, base_y - chat_cover.origin.y, WHITE);
            }

            // Draw chat messages - position relative to chat_space
            if let Some((chat_x, chat_y)) = chat_space_pos {
                self.draw_chat_messages(chat_x, chat_y);
            }
        }

        // Draw buttons using their origin values from WZ data
        self.bt_claim.draw();
        self.bt_system.draw();
        self.bt_cashshop.draw();
        self.bt_channel.draw();
        self.bt_chat.draw();
        self.bt_keysetting.draw();
        self.bt_menu.draw();
        self.bt_quest.draw();
        self.bt_stat.draw();
        self.bt_inven.draw();
        self.bt_equip.draw();
        self.bt_character.draw();
        self.bt_skill.draw();
        self.bt_mts.draw();
        self.chat_open_button.draw();
        self.chat_close_button.draw();
        self.scroll_up.draw();
        self.scroll_down.draw();

        // Draw notice
        if let Some(notice) = &self.notice {
            draw_texture(&notice.texture, base_x - notice.origin.x, base_y - notice.origin.y, WHITE);
        }

        // Draw level number (use origin from lv_backtrnd as reference if available)
        if let Some(lv_back) = &self.lv_backtrnd {
            // Level background origin is (510, 33), size 222x32
            // Position level numbers inside the level background box
            let lv_x = base_x - lv_back.origin.x + 35.0;
            let lv_y = base_y - lv_back.origin.y + 10.0;
            self.draw_level(character.level, lv_x, lv_y);
        }

        // Draw gauge edit mode UI
        if self.gauge_edit_mode {
            self.draw_gauge_edit_ui();
        }
    }

    /// Draw gauge editing mode UI overlay
    fn draw_gauge_edit_ui(&self) {
        let y_offset = 10.0;
        draw_text("GAUGE EDIT MODE", 10.0, y_offset, 20.0, YELLOW);
        draw_text("Keys: 1=HP 2=MP 3=EXP | Arrows=Move | Shift+Arrows=Width | P=Print", 10.0, y_offset + 20.0, 16.0, WHITE);

        if let Some(ref selected) = self.selected_gauge {
            let msg = format!("Selected: {} gauge", selected.to_uppercase());
            draw_text(&msg, 10.0, y_offset + 40.0, 18.0, GREEN);

            if let Some((x, y, w)) = self.gauge_offsets.get(selected) {
                let coords = format!("Position: x={:.0}, y={:.0}, width={:.0}", x, y, w);
                draw_text(&coords, 10.0, y_offset + 60.0, 16.0, LIGHTGRAY);
            }
        }
    }

    /// Draw a gauge (HP/MP/EXP) - base_x and base_y are the screen center/bottom
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

        // Use frame 0 only - no animation to prevent flashing
        let frame = &frames[0];

        // Gauge positions relative to gaugeBackgrd origin (286, 33)
        // gaugeBackgrd is at base_x - 286, base_y - 33
        // HP gauge: x=29, width=137
        // MP gauge: x=198, width=137  
        // EXP gauge: x=28, y=18, width=255
        // The gauge texture width represents 100%, so we clip it based on percentage
        let (offset_x, offset_y, max_width) = match gauge_type {
            "hp" => (29.0, 2.0, 137.0),
            "mp" => (198.0, 2.0, 137.0),
            "exp" => (28.0, 18.0, 255.0),
            _ => return,
        };

        // Get gaugeBackgrd position
        let gauge_bg_x = base_x - 286.0;
        let gauge_bg_y = base_y - 33.0;

        // Calculate the visible width based on percentage
        // percentage 0.5 = 50% = half the gauge should be visible
        let gauge_width = max_width * percentage;
        let draw_x = gauge_bg_x + offset_x;
        let draw_y = gauge_bg_y + offset_y;

        // Use source_rect to clip the texture instead of stretching
        // This shows the correct portion of the gauge texture
        let tex_width = frame.texture.width();
        let tex_height = frame.texture.height();
        let source_width = tex_width * percentage;

        draw_texture_ex(
            &frame.texture,
            draw_x,
            draw_y,
            WHITE,
            DrawTextureParams {
                source: Some(Rect::new(0.0, 0.0, source_width, tex_height)),
                dest_size: Some(Vec2::new(gauge_width, tex_height)),
                ..Default::default()
            },
        );
    }

    /// Draw gauge numbers (HP, MP, EXP) - displays "current/max" format
    fn draw_gauge_numbers(&self, gauge_type: &str, current: u32, max: u32, base_x: f32, base_y: f32) {
        if self.gauge_numbers.is_empty() {
            return;
        }

        // Get gaugeBackgrd position (origin 286, 33)
        let gauge_bg_x = base_x - 286.0;
        let gauge_bg_y = base_y - 33.0;

        // Position numbers centered on each gauge bar
        // HP bar: x=29 to x=166 (width 137), center at ~97
        // MP bar: x=198 to x=335 (width 137), center at ~266
        // EXP bar: x=28 to x=283 (width 255), center at ~155, y offset for exp row
        let (num_x, num_y) = match gauge_type {
            "hp" => (gauge_bg_x + 97.0, gauge_bg_y + 4.0),
            "mp" => (gauge_bg_x + 266.0, gauge_bg_y + 4.0),
            "exp" => (gauge_bg_x + 155.0, gauge_bg_y + 20.0),
            _ => return,
        };

        // Format the text: "[current\max]" or "[current%]" for exp
        let text = if gauge_type == "exp" {
            format!("[{}%]", current)
        } else {
            format!("[{}\\{}]", current, max)
        };

        // Calculate total width to center the text
        let mut total_width = 0.0;
        for ch in text.chars() {
            let key = ch.to_string();
            if let Some(num_tex) = self.gauge_numbers.get(&key) {
                total_width += num_tex.texture.width() - 1.0;
            }
        }

        let mut x_offset = num_x - total_width / 2.0;
        let spacing = -1.0;

        // Draw each character
        for ch in text.chars() {
            let key = ch.to_string();
            if let Some(num_tex) = self.gauge_numbers.get(&key) {
                draw_texture(
                    &num_tex.texture,
                    x_offset - num_tex.origin.x,
                    num_y - num_tex.origin.y,
                    WHITE,
                );
                x_offset += num_tex.texture.width() + spacing;
            }
        }
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
    fn draw_chat_input(&self, text_x: f32, text_y: f32, has_chat_enter: bool) {
        let font_size = 12.0;

        // Draw a background for the input area to ensure visibility
        if self.chat_focused {
            draw_rectangle(text_x - 2.0, text_y - 12.0, 400.0, 16.0, Color::from_rgba(255, 255, 255, 200));
        }

        // Draw chat target prefix (e.g., "[party]", "[all]")
        let target_prefix = format!("[{}] ", self.current_chat_target);
        let prefix_color = Color::from_rgba(100, 100, 50, 255);  // Dark yellow/olive
        draw_text(&target_prefix, text_x, text_y, font_size, prefix_color);
        let prefix_width = measure_text(&target_prefix, None, font_size as u16, 1.0).width;

        // Draw input text - BLACK for visibility
        let text_color = BLACK;
        
        // Draw text before cursor
        let text_before_cursor = &self.chat_state.input_buffer[..self.chat_state.cursor_position.min(self.chat_state.input_buffer.len())];
        let text_after_cursor = &self.chat_state.input_buffer[self.chat_state.cursor_position.min(self.chat_state.input_buffer.len())..];
        
        let before_width = measure_text(text_before_cursor, None, font_size as u16, 1.0).width;
        draw_text(text_before_cursor, text_x + prefix_width, text_y, font_size, text_color);
        draw_text(text_after_cursor, text_x + prefix_width + before_width, text_y, font_size, text_color);

        // Draw blinking caret at cursor position (only when focused)
        if self.chat_focused && self.caret_visible {
            draw_text("|", text_x + prefix_width + before_width, text_y, font_size, text_color);
        }
        
        // Debug: show chat state
        if self.chat_focused {
            draw_text("(Chat Active - ESC to cancel)", text_x, text_y + 14.0, 10.0, RED);
        }
    }

    /// Draw chat message history - chat_x/chat_y is the top-left of chat_space
    fn draw_chat_messages(&self, chat_x: f32, chat_y: f32) {
        // Position messages inside the chat area with padding
        let text_x = chat_x + 10.0;
        let mut text_y = chat_y + 15.0;  // Start from top with padding
        let font_size = 11.0;
        let line_height = 15.0;

        let start_idx = self.chat_state.scroll_offset;
        let end_idx = (start_idx + self.chat_state.max_visible_lines).min(self.chat_state.messages.len());

        for i in start_idx..end_idx {
            if let Some(msg) = self.chat_state.messages.get(i) {
                // Draw message with dark gray text for visibility
                let text_color = Color::from_rgba(50, 50, 50, 255);  // Dark gray
                draw_text(&msg.text, text_x, text_y, font_size, text_color);
                text_y += line_height;
            }
        }
    }

    /// Check if status bar is loaded
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Check if chat is focused (for disabling character movement)
    pub fn is_chat_focused(&self) -> bool {
        self.chat_focused
    }

    /// Check if cash shop button was clicked
    pub fn bt_cashshop_clicked(&self) -> bool {
        self.bt_cashshop.is_clicked()
    }

    /// Check if key setting button was clicked
    pub fn bt_keysetting_clicked(&self) -> bool {
        self.bt_keysetting.is_clicked()
    }

    /// Check if menu button was clicked
    pub fn bt_menu_clicked(&self) -> bool {
        self.bt_menu.is_clicked()
    }

    /// Check if channel button was clicked
    pub fn bt_channel_clicked(&self) -> bool {
        self.bt_channel.is_clicked()
    }

    /// Get menu button screen position for positioning the menu above it
    pub fn get_menu_button_pos(&self) -> (f32, f32) {
        self.bt_menu.get_screen_pos()
    }

    /// Get and clear the last sent chat message (for balloon display)
    pub fn take_last_sent_message(&mut self) -> Option<String> {
        self.chat_state.last_sent_message.take()
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
    chat_enter: Option<TextureWithOrigin>,
    chat_targets: HashMap<String, TextureWithOrigin>,
    lv_numbers: Vec<TextureWithOrigin>,
    gauge_numbers: HashMap<String, TextureWithOrigin>,
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
    bt_cashshop: Button,
    bt_channel: Button,
    bt_equip: Button,
    bt_inven: Button,
    bt_keysetting: Button,
    bt_menu: Button,
    bt_system: Button,
    bt_skill: Button,
    bt_mts: Button,
}
