use macroquad::prelude::*;
use crate::assets::AssetManager;
use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzReader, WzNodeCast};

// Use StatusBar2.img for menu assets (Menu section)
const STATUSBAR_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/UI/StatusBar2.img";
const STATUSBAR_CACHE: &str = "/01/UI/StatusBar2.img";

/// Texture with origin point
struct TextureWithOrigin {
    texture: Texture2D,
    origin: Vec2,
}

/// Button state
#[derive(PartialEq, Clone, Copy, Default)]
enum ButtonState {
    #[default]
    Normal,
    MouseOver,
    Pressed,
}

/// Menu button
struct MenuButton {
    normal: Option<Texture2D>,
    mouse_over: Option<Texture2D>,
    pressed: Option<Texture2D>,
    disabled: Option<Texture2D>,
    origin: Vec2,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    state: ButtonState,
    enabled: bool,
}

impl MenuButton {
    fn new() -> Self {
        Self {
            normal: None,
            mouse_over: None,
            pressed: None,
            disabled: None,
            origin: Vec2::ZERO,
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            state: ButtonState::Normal,
            enabled: true,
        }
    }

    fn update(&mut self, base_x: f32, base_y: f32) {
        if !self.enabled {
            return;
        }

        let draw_x = base_x + self.x - self.origin.x;
        let draw_y = base_y + self.y - self.origin.y;
        
        let (mouse_x, mouse_y) = mouse_position();
        let in_bounds = mouse_x >= draw_x && mouse_x <= draw_x + self.width
            && mouse_y >= draw_y && mouse_y <= draw_y + self.height;

        if in_bounds {
            if is_mouse_button_down(MouseButton::Left) {
                self.state = ButtonState::Pressed;
            } else {
                self.state = ButtonState::MouseOver;
            }
        } else {
            self.state = ButtonState::Normal;
        }
    }

    fn is_clicked(&self, base_x: f32, base_y: f32) -> bool {
        if !self.enabled {
            return false;
        }

        let draw_x = base_x + self.x - self.origin.x;
        let draw_y = base_y + self.y - self.origin.y;
        
        let (mouse_x, mouse_y) = mouse_position();
        let in_bounds = mouse_x >= draw_x && mouse_x <= draw_x + self.width
            && mouse_y >= draw_y && mouse_y <= draw_y + self.height;
        in_bounds && is_mouse_button_released(MouseButton::Left)
    }

    fn draw(&self, base_x: f32, base_y: f32) {
        let texture = if !self.enabled {
            &self.disabled
        } else {
            match self.state {
                ButtonState::MouseOver if self.mouse_over.is_some() => &self.mouse_over,
                ButtonState::Pressed if self.pressed.is_some() => &self.pressed,
                _ => &self.normal,
            }
        };

        if let Some(tex) = texture {
            draw_texture(tex, base_x + self.x - self.origin.x, base_y + self.y - self.origin.y, WHITE);
        }
    }
}

impl Default for MenuButton {
    fn default() -> Self {
        Self::new()
    }
}

/// Menu action types
#[derive(Clone, Copy, PartialEq)]
pub enum MenuAction {
    None,
    Character,
    Stat,
    Quest,
    Inventory,
    Equip,
    Skill,
    KeyConfig,
    SystemOption,
    GameOption,
    Quit,
    Channel,
    Messenger,
}

/// GameMenu window - main game menu
pub struct GameMenu {
    visible: bool,
    loaded: bool,
    // Background
    background: Option<TextureWithOrigin>,
    // Menu buttons
    btn_character: MenuButton,
    btn_stat: MenuButton,
    btn_quest: MenuButton,
    btn_inventory: MenuButton,
    btn_equip: MenuButton,
    btn_skill: MenuButton,
    btn_key_config: MenuButton,
    btn_system_option: MenuButton,
    btn_game_option: MenuButton,
    btn_quit: MenuButton,
    // Window position
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    // Pending action
    pending_action: MenuAction,
}

impl GameMenu {
    pub fn new() -> Self {
        Self {
            visible: false,
            loaded: false,
            background: None,
            btn_character: MenuButton::new(),
            btn_stat: MenuButton::new(),
            btn_quest: MenuButton::new(),
            btn_inventory: MenuButton::new(),
            btn_equip: MenuButton::new(),
            btn_skill: MenuButton::new(),
            btn_key_config: MenuButton::new(),
            btn_system_option: MenuButton::new(),
            btn_game_option: MenuButton::new(),
            btn_quit: MenuButton::new(),
            x: 100.0,
            y: 100.0,
            width: 150.0,
            height: 300.0,
            pending_action: MenuAction::None,
        }
    }

    /// Load GameMenu assets
    pub async fn load_assets(&mut self) {
        info!("Loading GameMenu assets...");
        
        match Self::load_from_wz().await {
            Ok(data) => {
                self.background = data.background;
                self.btn_character = data.btn_character;
                self.btn_stat = data.btn_stat;
                self.btn_quest = data.btn_quest;
                self.btn_inventory = data.btn_inventory;
                self.btn_equip = data.btn_equip;
                self.btn_skill = data.btn_skill;
                self.btn_key_config = data.btn_key_config;
                self.btn_system_option = data.btn_system_option;
                self.btn_game_option = data.btn_game_option;
                self.btn_quit = data.btn_quit;
                
                if let Some(ref bg) = self.background {
                    self.width = bg.texture.width();
                    self.height = bg.texture.height();
                }
                
                self.loaded = true;
                info!("GameMenu assets loaded successfully");
            }
            Err(e) => {
                error!("Failed to load GameMenu assets: {}", e);
            }
        }
    }

    async fn load_from_wz() -> Result<GameMenuData, String> {
        let bytes = AssetManager::fetch_and_cache(STATUSBAR_URL, STATUSBAR_CACHE).await
            .map_err(|e| format!("Failed to fetch StatusBar2.img: {}", e))?;

        let wz_iv = guess_iv_from_wz_img(&bytes)
            .ok_or_else(|| "Unable to guess version from StatusBar2.img".to_string())?;

        let byte_len = bytes.len();
        let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
        let cache_name_ref: wz_reader::WzNodeName = STATUSBAR_CACHE.to_string().into();
        let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
        let root_node: WzNodeArc = WzNode::new(&STATUSBAR_CACHE.into(), wz_image, None).into();

        root_node.write().unwrap().parse(&root_node)
            .map_err(|e| format!("Failed to parse StatusBar2.img: {:?}", e))?;

        let mut data = GameMenuData::default();

        // Load menu background from mainBar/Menu/backgrnd (has frames 0, 1, 2)
        data.background = Self::load_texture(&root_node, "mainBar/Menu/backgrnd/0").await.ok();

        // Load menu buttons from mainBar/Menu section
        // Based on StatusBar2_structure.txt: BtMSN, BtEquip, BtSkill, BtRank, BtStat, BtCommunity, BtItem, BtQuest
        let button_configs = [
            ("BtStat", &mut data.btn_stat, 0.0),
            ("BtEquip", &mut data.btn_equip, 22.0),
            ("BtItem", &mut data.btn_inventory, 44.0),
            ("BtSkill", &mut data.btn_skill, 66.0),
            ("BtQuest", &mut data.btn_quest, 88.0),
            ("BtCommunity", &mut data.btn_character, 110.0),
        ];

        for (btn_name, btn, y_offset) in button_configs {
            let base_path = format!("mainBar/Menu/{}", btn_name);
            if let Ok(normal) = Self::load_texture(&root_node, &format!("{}/normal/0", base_path)).await {
                btn.width = normal.texture.width();
                btn.height = normal.texture.height();
                btn.origin = normal.origin;
                btn.normal = Some(normal.texture);
            }
            if let Ok(hover) = Self::load_texture(&root_node, &format!("{}/mouseOver/0", base_path)).await {
                btn.mouse_over = Some(hover.texture);
            }
            if let Ok(pressed) = Self::load_texture(&root_node, &format!("{}/pressed/0", base_path)).await {
                btn.pressed = Some(pressed.texture);
            }
            if let Ok(disabled) = Self::load_texture(&root_node, &format!("{}/disabled/0", base_path)).await {
                btn.disabled = Some(disabled.texture);
            }
            btn.y = y_offset;
            btn.x = 0.0;
        }

        // Set up remaining buttons with placeholder positions
        data.btn_key_config.y = 132.0;
        data.btn_key_config.width = 80.0;
        data.btn_key_config.height = 22.0;
        
        data.btn_system_option.y = 154.0;
        data.btn_system_option.width = 80.0;
        data.btn_system_option.height = 22.0;
        
        data.btn_game_option.y = 176.0;
        data.btn_game_option.width = 80.0;
        data.btn_game_option.height = 22.0;
        
        data.btn_quit.y = 198.0;
        data.btn_quit.width = 80.0;
        data.btn_quit.height = 22.0;

        Ok(data)
    }

    async fn load_texture(root_node: &WzNodeArc, path: &str) -> Result<TextureWithOrigin, String> {
        let node = root_node.read().unwrap()
            .at_path_parsed(path)
            .map_err(|e| format!("Path '{}' not found: {:?}", path, e))?;

        node.write().unwrap().parse(&node)
            .map_err(|e| format!("Failed to parse node at '{}': {:?}", path, e))?;

        let node_read = node.read().unwrap();
        let png = node_read.try_as_png()
            .ok_or_else(|| format!("Node at '{}' is not a PNG", path))?;

        let png_data = png.extract_png()
            .map_err(|e| format!("Failed to extract PNG at '{}': {:?}", path, e))?;

        let rgba_img = png_data.to_rgba8();
        let width = rgba_img.width() as u16;
        let height = rgba_img.height() as u16;
        let bytes = rgba_img.into_raw();
        let texture = Texture2D::from_rgba8(width, height, &bytes);

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

    /// Show the menu at a position
    pub fn show(&mut self, x: f32, y: f32) {
        self.x = x;
        self.y = y;
        self.visible = true;
        self.pending_action = MenuAction::None;
    }

    /// Hide the menu
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Toggle visibility - show above the menu button
    pub fn toggle(&mut self) {
        if self.visible {
            self.hide();
        } else {
            // Show above the menu button at bottom-right of status bar
            // Position menu so its bottom aligns just above the status bar (~70px from bottom)
            let menu_x = screen_width() / 2.0 + 180.0 - self.width;
            let menu_y = screen_height() - 70.0 - self.height;
            self.show(menu_x, menu_y);
        }
    }

    /// Toggle visibility at specific position (above button)
    pub fn toggle_at(&mut self, button_x: f32, button_y: f32) {
        if self.visible {
            self.hide();
        } else {
            // Show above the button - position menu well above the status bar
            let menu_x = button_x - self.width / 2.0;
            let menu_y = button_y - self.height - 80.0; // 80px gap above button
            // Clamp to screen bounds
            let clamped_x = menu_x.max(0.0).min(screen_width() - self.width);
            let clamped_y = menu_y.max(0.0);
            self.show(clamped_x, clamped_y);
        }
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Get and clear pending action
    pub fn take_action(&mut self) -> MenuAction {
        let action = self.pending_action;
        self.pending_action = MenuAction::None;
        action
    }

    /// Update menu state
    pub fn update(&mut self) {
        if !self.visible {
            return;
        }

        // Update buttons
        self.btn_character.update(self.x, self.y);
        self.btn_stat.update(self.x, self.y);
        self.btn_quest.update(self.x, self.y);
        self.btn_inventory.update(self.x, self.y);
        self.btn_equip.update(self.x, self.y);
        self.btn_skill.update(self.x, self.y);
        self.btn_key_config.update(self.x, self.y);
        self.btn_system_option.update(self.x, self.y);
        self.btn_game_option.update(self.x, self.y);
        self.btn_quit.update(self.x, self.y);

        // Check button clicks
        if self.btn_character.is_clicked(self.x, self.y) {
            self.pending_action = MenuAction::Character;
            self.visible = false;
        }
        if self.btn_stat.is_clicked(self.x, self.y) {
            self.pending_action = MenuAction::Stat;
            self.visible = false;
        }
        if self.btn_quest.is_clicked(self.x, self.y) {
            self.pending_action = MenuAction::Quest;
            self.visible = false;
        }
        if self.btn_inventory.is_clicked(self.x, self.y) {
            self.pending_action = MenuAction::Inventory;
            self.visible = false;
        }
        if self.btn_equip.is_clicked(self.x, self.y) {
            self.pending_action = MenuAction::Equip;
            self.visible = false;
        }
        if self.btn_skill.is_clicked(self.x, self.y) {
            self.pending_action = MenuAction::Skill;
            self.visible = false;
        }
        if self.btn_key_config.is_clicked(self.x, self.y) {
            self.pending_action = MenuAction::KeyConfig;
            self.visible = false;
        }
        if self.btn_system_option.is_clicked(self.x, self.y) {
            self.pending_action = MenuAction::SystemOption;
            self.visible = false;
        }
        if self.btn_game_option.is_clicked(self.x, self.y) {
            self.pending_action = MenuAction::GameOption;
            self.visible = false;
        }
        if self.btn_quit.is_clicked(self.x, self.y) {
            self.pending_action = MenuAction::Quit;
            self.visible = false;
        }

        // Close on ESC or click outside
        if is_key_pressed(KeyCode::Escape) {
            self.visible = false;
        }

        // Check click outside menu
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            if mx < self.x || mx > self.x + self.width || my < self.y || my > self.y + self.height {
                self.visible = false;
            }
        }
    }

    /// Draw the menu
    pub fn draw(&self) {
        if !self.visible {
            return;
        }

        // Draw background from loaded asset
        if let Some(bg) = &self.background {
            draw_texture(&bg.texture, self.x - bg.origin.x, self.y - bg.origin.y, WHITE);
        } else {
            // Fallback background only if no asset loaded
            draw_rectangle(self.x, self.y, self.width, self.height, Color::from_rgba(30, 30, 40, 240));
            draw_rectangle_lines(self.x, self.y, self.width, self.height, 1.0, Color::from_rgba(100, 100, 120, 255));
        }

        // Draw buttons using loaded textures
        self.btn_stat.draw(self.x, self.y);
        self.btn_equip.draw(self.x, self.y);
        self.btn_inventory.draw(self.x, self.y);
        self.btn_skill.draw(self.x, self.y);
        self.btn_quest.draw(self.x, self.y);
        self.btn_character.draw(self.x, self.y);

        // Draw fallback labels for buttons without textures
        let fallback_items = [
            ("Key Config", &self.btn_key_config),
            ("System Option", &self.btn_system_option),
            ("Game Option", &self.btn_game_option),
            ("Quit Game", &self.btn_quit),
        ];

        for (label, btn) in fallback_items {
            if btn.normal.is_none() {
                let draw_x = self.x + btn.x;
                let draw_y = self.y + btn.y;

                let bg_color = match btn.state {
                    ButtonState::MouseOver => Color::from_rgba(60, 60, 80, 200),
                    ButtonState::Pressed => Color::from_rgba(80, 80, 100, 200),
                    _ => Color::from_rgba(40, 40, 50, 200),
                };

                draw_rectangle(draw_x, draw_y, btn.width, btn.height, bg_color);
                draw_rectangle_lines(draw_x, draw_y, btn.width, btn.height, 1.0, Color::from_rgba(80, 80, 100, 255));
                draw_text(label, draw_x + 5.0, draw_y + 16.0, 14.0, WHITE);
            } else {
                btn.draw(self.x, self.y);
            }
        }
    }
}

impl Default for GameMenu {
    fn default() -> Self {
        Self::new()
    }
}

/// Temporary data structure for loading
#[derive(Default)]
struct GameMenuData {
    background: Option<TextureWithOrigin>,
    btn_character: MenuButton,
    btn_stat: MenuButton,
    btn_quest: MenuButton,
    btn_inventory: MenuButton,
    btn_equip: MenuButton,
    btn_skill: MenuButton,
    btn_key_config: MenuButton,
    btn_system_option: MenuButton,
    btn_game_option: MenuButton,
    btn_quit: MenuButton,
}
