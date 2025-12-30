use macroquad::prelude::*;
use crate::assets::AssetManager;
use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzReader, WzNodeCast};

const CASHSHOP_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/UI/CashShop.img";
const CASHSHOP_CACHE: &str = "/01/UI/CashShop.img";

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

/// Simple button for CashShop
struct CashShopButton {
    normal: Option<Texture2D>,
    mouse_over: Option<Texture2D>,
    pressed: Option<Texture2D>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    state: ButtonState,
}

impl CashShopButton {
    fn new() -> Self {
        Self {
            normal: None,
            mouse_over: None,
            pressed: None,
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            state: ButtonState::Normal,
        }
    }

    fn update(&mut self) {
        let (mouse_x, mouse_y) = mouse_position();
        let in_bounds = mouse_x >= self.x && mouse_x <= self.x + self.width
            && mouse_y >= self.y && mouse_y <= self.y + self.height;

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

    fn is_clicked(&self) -> bool {
        let (mouse_x, mouse_y) = mouse_position();
        let in_bounds = mouse_x >= self.x && mouse_x <= self.x + self.width
            && mouse_y >= self.y && mouse_y <= self.y + self.height;
        in_bounds && is_mouse_button_released(MouseButton::Left)
    }

    fn draw(&self) {
        let texture = match self.state {
            ButtonState::MouseOver if self.mouse_over.is_some() => &self.mouse_over,
            ButtonState::Pressed if self.pressed.is_some() => &self.pressed,
            _ => &self.normal,
        };

        if let Some(tex) = texture {
            draw_texture(tex, self.x, self.y, WHITE);
        }
    }
}

/// CashShop UI - Full screen overlay
pub struct CashShop {
    visible: bool,
    loaded: bool,
    // Background
    background: Option<Texture2D>,
    // Exit button
    exit_button: CashShopButton,
    // Tab buttons
    tab_buttons: Vec<CashShopButton>,
    current_tab: usize,
}

impl CashShop {
    pub fn new() -> Self {
        Self {
            visible: false,
            loaded: false,
            background: None,
            exit_button: CashShopButton::new(),
            tab_buttons: Vec::new(),
            current_tab: 0,
        }
    }

    /// Load CashShop assets
    pub async fn load_assets(&mut self) {
        info!("Loading CashShop assets...");
        
        match Self::load_from_wz().await {
            Ok((bg, exit_btn)) => {
                self.background = bg;
                self.exit_button = exit_btn;
                self.loaded = true;
                info!("CashShop assets loaded successfully");
            }
            Err(e) => {
                error!("Failed to load CashShop assets: {}", e);
            }
        }
    }

    async fn load_from_wz() -> Result<(Option<Texture2D>, CashShopButton), String> {
        let bytes = AssetManager::fetch_and_cache(CASHSHOP_URL, CASHSHOP_CACHE).await
            .map_err(|e| format!("Failed to fetch CashShop.img: {}", e))?;

        let wz_iv = guess_iv_from_wz_img(&bytes)
            .ok_or_else(|| "Unable to guess version from CashShop.img".to_string())?;

        let byte_len = bytes.len();
        let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
        let cache_name_ref: wz_reader::WzNodeName = CASHSHOP_CACHE.to_string().into();
        let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
        let root_node: WzNodeArc = WzNode::new(&CASHSHOP_CACHE.into(), wz_image, None).into();

        root_node.write().unwrap().parse(&root_node)
            .map_err(|e| format!("Failed to parse CashShop.img: {:?}", e))?;

        // Load background
        let bg = Self::load_texture(&root_node, "backgrnd").await.ok();

        // Load exit button
        let mut exit_btn = CashShopButton::new();
        if let Ok(normal) = Self::load_texture_simple(&root_node, "BtExit/normal/0").await {
            exit_btn.width = normal.width();
            exit_btn.height = normal.height();
            exit_btn.normal = Some(normal);
        }
        if let Ok(hover) = Self::load_texture_simple(&root_node, "BtExit/mouseOver/0").await {
            exit_btn.mouse_over = Some(hover);
        }
        if let Ok(pressed) = Self::load_texture_simple(&root_node, "BtExit/pressed/0").await {
            exit_btn.pressed = Some(pressed);
        }

        Ok((bg.map(|t| t.texture), exit_btn))
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

    async fn load_texture_simple(root_node: &WzNodeArc, path: &str) -> Result<Texture2D, String> {
        let tex = Self::load_texture(root_node, path).await?;
        Ok(tex.texture)
    }

    /// Show the CashShop
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide the CashShop
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Check if CashShop is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Update CashShop state
    pub fn update(&mut self) -> bool {
        if !self.visible {
            return false;
        }

        // Position exit button in top-right corner
        self.exit_button.x = screen_width() - self.exit_button.width - 20.0;
        self.exit_button.y = 20.0;
        self.exit_button.update();

        // Check for exit button click or ESC key
        if self.exit_button.is_clicked() || is_key_pressed(KeyCode::Escape) {
            self.visible = false;
            return true; // Consumed input
        }

        true // CashShop is active, consume input
    }

    /// Draw the CashShop
    pub fn draw(&self) {
        if !self.visible {
            return;
        }

        // Draw semi-transparent overlay
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::from_rgba(0, 0, 0, 200));

        // Draw background centered
        if let Some(bg) = &self.background {
            let x = (screen_width() - bg.width()) / 2.0;
            let y = (screen_height() - bg.height()) / 2.0;
            draw_texture(bg, x, y, WHITE);
        }

        // Draw title
        let title = "Cash Shop";
        let title_width = measure_text(title, None, 32, 1.0).width;
        draw_text(title, (screen_width() - title_width) / 2.0, 60.0, 32.0, WHITE);

        // Draw exit button
        self.exit_button.draw();

        // Draw placeholder content
        let content_y = 100.0;
        draw_text("Welcome to the Cash Shop!", screen_width() / 2.0 - 100.0, content_y, 20.0, WHITE);
        draw_text("Press ESC or click Exit to return to the game", screen_width() / 2.0 - 150.0, content_y + 30.0, 16.0, LIGHTGRAY);

        // Draw categories placeholder
        let categories = ["Hot Items", "Equipment", "Use", "Setup", "Etc", "Pet"];
        let mut cat_x = 100.0;
        let cat_y = 150.0;
        for cat in categories {
            draw_rectangle(cat_x, cat_y, 100.0, 30.0, Color::from_rgba(60, 60, 80, 200));
            draw_rectangle_lines(cat_x, cat_y, 100.0, 30.0, 1.0, Color::from_rgba(100, 100, 120, 255));
            draw_text(cat, cat_x + 10.0, cat_y + 20.0, 14.0, WHITE);
            cat_x += 110.0;
        }

        // Draw item grid placeholder
        let grid_x = 100.0;
        let grid_y = 200.0;
        let slot_size = 60.0;
        let cols = 8;
        let rows = 4;

        for row in 0..rows {
            for col in 0..cols {
                let x = grid_x + col as f32 * (slot_size + 5.0);
                let y = grid_y + row as f32 * (slot_size + 5.0);
                draw_rectangle(x, y, slot_size, slot_size, Color::from_rgba(40, 40, 50, 200));
                draw_rectangle_lines(x, y, slot_size, slot_size, 1.0, Color::from_rgba(80, 80, 100, 200));
            }
        }
    }
}

impl Default for CashShop {
    fn default() -> Self {
        Self::new()
    }
}
