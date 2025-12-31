use macroquad::prelude::*;
use crate::assets::AssetManager;
use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzReader, WzNodeCast};

const UIWINDOW2_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/UI/UIWindow2.img";
const UIWINDOW2_CACHE: &str = "/01/UI/UIWindow2.img";

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

/// Simple button
struct KeyConfigButton {
    normal: Option<Texture2D>,
    mouse_over: Option<Texture2D>,
    pressed: Option<Texture2D>,
    origin: Vec2,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    state: ButtonState,
}

impl KeyConfigButton {
    fn new() -> Self {
        Self {
            normal: None,
            mouse_over: None,
            pressed: None,
            origin: Vec2::ZERO,
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            state: ButtonState::Normal,
        }
    }

    fn update(&mut self, base_x: f32, base_y: f32) {
        let draw_x = base_x - self.origin.x;
        let draw_y = base_y - self.origin.y;
        
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
        let draw_x = base_x - self.origin.x;
        let draw_y = base_y - self.origin.y;
        
        let (mouse_x, mouse_y) = mouse_position();
        let in_bounds = mouse_x >= draw_x && mouse_x <= draw_x + self.width
            && mouse_y >= draw_y && mouse_y <= draw_y + self.height;
        in_bounds && is_mouse_button_released(MouseButton::Left)
    }

    fn draw(&self, base_x: f32, base_y: f32) {
        let texture = match self.state {
            ButtonState::MouseOver if self.mouse_over.is_some() => &self.mouse_over,
            ButtonState::Pressed if self.pressed.is_some() => &self.pressed,
            _ => &self.normal,
        };

        if let Some(tex) = texture {
            draw_texture(tex, base_x - self.origin.x, base_y - self.origin.y, WHITE);
        }
    }
}

/// KeyConfig window - keyboard configuration
pub struct KeyConfig {
    visible: bool,
    loaded: bool,
    // Background layers (z-order: backgrnd, backgrnd2, backgrnd3)
    background: Option<TextureWithOrigin>,
    background2: Option<TextureWithOrigin>,
    background3: Option<TextureWithOrigin>,
    // Buttons
    cancel_button: KeyConfigButton,
    default_button: KeyConfigButton,
    ok_button: KeyConfigButton,
    // Icons for key bindings
    icons: std::collections::HashMap<i32, Texture2D>,
    // Window position
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    // Dragging
    dragging: bool,
    drag_offset: Vec2,
}

impl KeyConfig {
    pub fn new() -> Self {
        Self {
            visible: false,
            loaded: false,
            background: None,
            background2: None,
            background3: None,
            cancel_button: KeyConfigButton::new(),
            default_button: KeyConfigButton::new(),
            ok_button: KeyConfigButton::new(),
            icons: std::collections::HashMap::new(),
            x: 100.0,
            y: 100.0,
            width: 632.0,
            height: 270.0,
            dragging: false,
            drag_offset: Vec2::ZERO,
        }
    }

    /// Load KeyConfig assets
    pub async fn load_assets(&mut self) {
        info!("Loading KeyConfig assets...");
        
        match Self::load_from_wz().await {
            Ok((bg, bg2, bg3, cancel, default_btn, ok_btn, icons)) => {
                if let Some(ref b) = bg {
                    self.width = b.texture.width();
                    self.height = b.texture.height();
                }
                self.background = bg;
                self.background2 = bg2;
                self.background3 = bg3;
                self.cancel_button = cancel;
                self.default_button = default_btn;
                self.ok_button = ok_btn;
                self.icons = icons;
                self.loaded = true;
                // Center window
                self.x = (screen_width() - self.width) / 2.0;
                self.y = (screen_height() - self.height) / 2.0;
                info!("KeyConfig assets loaded successfully with {} icons", self.icons.len());
            }
            Err(e) => {
                error!("Failed to load KeyConfig assets: {}", e);
            }
        }
    }

    async fn load_from_wz() -> Result<(Option<TextureWithOrigin>, Option<TextureWithOrigin>, Option<TextureWithOrigin>, KeyConfigButton, KeyConfigButton, KeyConfigButton, std::collections::HashMap<i32, Texture2D>), String> {
        let bytes = AssetManager::fetch_and_cache(UIWINDOW2_URL, UIWINDOW2_CACHE).await
            .map_err(|e| format!("Failed to fetch UIWindow2.img: {}", e))?;

        let wz_iv = guess_iv_from_wz_img(&bytes)
            .ok_or_else(|| "Unable to guess version from UIWindow2.img".to_string())?;

        let byte_len = bytes.len();
        let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
        let cache_name_ref: wz_reader::WzNodeName = UIWINDOW2_CACHE.to_string().into();
        let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
        let root_node: WzNodeArc = WzNode::new(&UIWINDOW2_CACHE.into(), wz_image, None).into();

        root_node.write().unwrap().parse(&root_node)
            .map_err(|e| format!("Failed to parse UIWindow2.img: {:?}", e))?;

        // Load background layers in z-order (backgrnd, backgrnd2, backgrnd3)
        let bg = Self::load_texture(&root_node, "KeyConfig/backgrnd").await.ok();
        let bg2 = Self::load_texture(&root_node, "KeyConfig/backgrnd2").await.ok();
        let bg3 = Self::load_texture(&root_node, "KeyConfig/backgrnd3").await.ok();

        // Load buttons
        let cancel = Self::load_button(&root_node, "KeyConfig/BtCancel").await;
        let default_btn = Self::load_button(&root_node, "KeyConfig/BtDefault").await;
        let ok_btn = Self::load_button(&root_node, "KeyConfig/BtOK").await;

        // Load icons
        let mut icons = std::collections::HashMap::new();
        let icon_ids = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 
                        20, 21, 23, 24, 25, 26, 27, 28, 29, 50, 51, 52, 53, 54, 
                        100, 101, 102, 103, 104, 105, 106];
        for id in icon_ids {
            let path = format!("KeyConfig/icon/{}", id);
            if let Ok(tex) = Self::load_texture(&root_node, &path).await {
                icons.insert(id, tex.texture);
            }
        }

        Ok((bg, bg2, bg3, cancel, default_btn, ok_btn, icons))
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

    async fn load_button(root_node: &WzNodeArc, base_path: &str) -> KeyConfigButton {
        let mut btn = KeyConfigButton::new();
        
        // Load normal state
        let normal_path = format!("{}/normal/0", base_path);
        if let Ok(tex) = Self::load_texture(root_node, &normal_path).await {
            btn.width = tex.texture.width();
            btn.height = tex.texture.height();
            btn.origin = tex.origin;
            btn.normal = Some(tex.texture);
        }
        
        // Load mouseOver state
        let hover_path = format!("{}/mouseOver/0", base_path);
        if let Ok(tex) = Self::load_texture(root_node, &hover_path).await {
            btn.mouse_over = Some(tex.texture);
        }
        
        // Load pressed state
        let pressed_path = format!("{}/pressed/0", base_path);
        if let Ok(tex) = Self::load_texture(root_node, &pressed_path).await {
            btn.pressed = Some(tex.texture);
        }
        
        btn
    }

    /// Show the KeyConfig window
    pub fn show(&mut self) {
        self.visible = true;
        // Center window when shown
        self.x = (screen_width() - self.width) / 2.0;
        self.y = (screen_height() - self.height) / 2.0;
    }

    /// Hide the KeyConfig window
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        if self.visible {
            self.hide();
        } else {
            self.show();
        }
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Update KeyConfig state
    pub fn update(&mut self) {
        if !self.visible {
            return;
        }

        let (mouse_x, mouse_y) = mouse_position();

        // Handle dragging
        if is_mouse_button_pressed(MouseButton::Left) {
            // Check if clicking on title bar area (top 25 pixels)
            if mouse_x >= self.x && mouse_x <= self.x + self.width
                && mouse_y >= self.y && mouse_y <= self.y + 25.0 {
                self.dragging = true;
                self.drag_offset = Vec2::new(mouse_x - self.x, mouse_y - self.y);
            }
        }

        if self.dragging {
            if is_mouse_button_down(MouseButton::Left) {
                self.x = mouse_x - self.drag_offset.x;
                self.y = mouse_y - self.drag_offset.y;
                // Keep on screen
                self.x = self.x.max(0.0).min(screen_width() - self.width);
                self.y = self.y.max(0.0).min(screen_height() - self.height);
            } else {
                self.dragging = false;
            }
        }

        // Update buttons
        self.cancel_button.update(self.x, self.y);
        self.default_button.update(self.x, self.y);
        self.ok_button.update(self.x, self.y);

        // Handle button clicks
        if self.cancel_button.is_clicked(self.x, self.y) || is_key_pressed(KeyCode::Escape) {
            self.visible = false;
        }
        if self.ok_button.is_clicked(self.x, self.y) {
            // Save settings and close
            self.visible = false;
        }
        if self.default_button.is_clicked(self.x, self.y) {
            // Reset to defaults
            info!("Reset key bindings to default");
        }
    }

    /// Draw the KeyConfig window
    pub fn draw(&self) {
        if !self.visible {
            return;
        }

        // Draw background layers in z-order (backgrnd z=-5, backgrnd2 z=-4, backgrnd3 z=-3)
        // backgrnd (z=-5) should be drawn first (behind)
        if let Some(bg) = &self.background {
            draw_texture(&bg.texture, self.x - bg.origin.x, self.y - bg.origin.y, WHITE);
        } else {
            // Fallback background
            draw_rectangle(self.x, self.y, self.width, self.height, Color::from_rgba(30, 30, 40, 240));
            draw_rectangle_lines(self.x, self.y, self.width, self.height, 1.0, Color::from_rgba(100, 100, 120, 255));
        }

        // backgrnd2 (z=-4) should be drawn second (middle)
        if let Some(bg2) = &self.background2 {
            draw_texture(&bg2.texture, self.x - bg2.origin.x, self.y - bg2.origin.y, WHITE);
        }

        // backgrnd3 (z=-3) should be drawn last (in front)
        if let Some(bg3) = &self.background3 {
            draw_texture(&bg3.texture, self.x - bg3.origin.x, self.y - bg3.origin.y, WHITE);
        }

        // Draw buttons (keyboard layout removed - only WZ assets should be visible)
        self.cancel_button.draw(self.x, self.y);
        self.default_button.draw(self.x, self.y);
        self.ok_button.draw(self.x, self.y);
    }

}

impl Default for KeyConfig {
    fn default() -> Self {
        Self::new()
    }
}
