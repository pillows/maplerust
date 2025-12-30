use macroquad::prelude::*;
use crate::assets::AssetManager;
use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzReader, WzNodeCast};

const UIWINDOW2_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/UI/UIWindow2.img";
const UIWINDOW2_CACHE: &str = "/01/UI/UIWindow2.img";

#[derive(Clone)]
struct TextureWithOrigin {
    texture: Texture2D,
    origin: Vec2,
}

#[derive(Clone)]
pub struct InventoryWindow {
    pub visible: bool,
    loaded: bool,
    // Background layers (z-order: backgrnd, backgrnd2, backgrnd3)
    backgrnd: Option<TextureWithOrigin>,
    backgrnd2: Option<TextureWithOrigin>,
    backgrnd3: Option<TextureWithOrigin>,
    // Window position
    x: f32,
    y: f32,
    dragging: bool,
    drag_offset_x: f32,
    drag_offset_y: f32,
}

impl InventoryWindow {
    pub fn new() -> Self {
        Self {
            visible: false,
            loaded: false,
            backgrnd: None,
            backgrnd2: None,
            backgrnd3: None,
            x: 100.0,
            y: 100.0,
            dragging: false,
            drag_offset_x: 0.0,
            drag_offset_y: 0.0,
        }
    }

    pub async fn load_assets(&mut self) {
        match Self::load_from_wz().await {
            Ok(data) => {
                self.backgrnd = data.backgrnd;
                self.backgrnd2 = data.backgrnd2;
                self.backgrnd3 = data.backgrnd3;
                self.loaded = true;
                info!("Inventory window assets loaded successfully");
            }
            Err(e) => {
                error!("Failed to load Inventory window assets: {}", e);
                self.loaded = false;
            }
        }
    }

    async fn load_from_wz() -> Result<InventoryWindowData, String> {
        let bytes = AssetManager::fetch_and_cache(UIWINDOW2_URL, UIWINDOW2_CACHE).await
            .map_err(|e| format!("Failed to fetch UIWindow2.img: {}", e))?;

        let wz_iv = guess_iv_from_wz_img(&bytes)
            .ok_or_else(|| "Unable to guess WZ version from UIWindow2.img".to_string())?;

        let byte_len = bytes.len();
        let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
        let cache_name_ref: wz_reader::WzNodeName = UIWINDOW2_CACHE.to_string().into();
        let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
        let root_node: WzNodeArc = WzNode::new(&UIWINDOW2_CACHE.to_string().into(), wz_image, None).into();

        root_node.write().unwrap().parse(&root_node)
            .map_err(|e| format!("Failed to parse UIWindow2.img: {:?}", e))?;

        let mut data = InventoryWindowData::default();

        // Load background layers in z-order
        data.backgrnd = Self::load_texture(&root_node, "Item/backgrnd").await.ok();
        data.backgrnd2 = Self::load_texture(&root_node, "Item/backgrnd2").await.ok();
        data.backgrnd3 = Self::load_texture(&root_node, "Item/backgrnd3").await.ok();

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

    pub fn update(&mut self) {
        if !self.visible || !self.loaded {
            return;
        }

        let (mouse_x, mouse_y) = mouse_position();

        // Handle dragging
        if is_mouse_button_pressed(MouseButton::Left) {
            // Check if clicking on title bar area (top 30 pixels)
            if mouse_y >= self.y && mouse_y <= self.y + 30.0 &&
               mouse_x >= self.x && mouse_x <= self.x + 500.0 {
                self.dragging = true;
                self.drag_offset_x = mouse_x - self.x;
                self.drag_offset_y = mouse_y - self.y;
            }
        }

        if is_mouse_button_down(MouseButton::Left) && self.dragging {
            self.x = mouse_x - self.drag_offset_x;
            self.y = mouse_y - self.drag_offset_y;
        } else {
            self.dragging = false;
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            // Center window when shown
            self.x = (screen_width() - 500.0) / 2.0;
            self.y = (screen_height() - 400.0) / 2.0;
        }
    }

    pub fn draw(&self) {
        if !self.visible || !self.loaded {
            return;
        }

        // Draw background layers in z-order (backgrnd, backgrnd2, backgrnd3)
        if let Some(bg) = &self.backgrnd {
            draw_texture(&bg.texture, self.x - bg.origin.x, self.y - bg.origin.y, WHITE);
        }
        if let Some(bg2) = &self.backgrnd2 {
            draw_texture(&bg2.texture, self.x - bg2.origin.x, self.y - bg2.origin.y, WHITE);
        }
        if let Some(bg3) = &self.backgrnd3 {
            draw_texture(&bg3.texture, self.x - bg3.origin.x, self.y - bg3.origin.y, WHITE);
        }
    }
}

#[derive(Clone)]
pub struct EquipWindow {
    pub visible: bool,
    loaded: bool,
    // Background layers (z-order: backgrnd, backgrnd2, backgrnd3)
    backgrnd: Option<TextureWithOrigin>,
    backgrnd2: Option<TextureWithOrigin>,
    backgrnd3: Option<TextureWithOrigin>,
    // Window position
    x: f32,
    y: f32,
    dragging: bool,
    drag_offset_x: f32,
    drag_offset_y: f32,
}

impl EquipWindow {
    pub fn new() -> Self {
        Self {
            visible: false,
            loaded: false,
            backgrnd: None,
            backgrnd2: None,
            backgrnd3: None,
            x: 550.0,
            y: 100.0,
            dragging: false,
            drag_offset_x: 0.0,
            drag_offset_y: 0.0,
        }
    }

    pub async fn load_assets(&mut self) {
        match Self::load_from_wz().await {
            Ok(data) => {
                self.backgrnd = data.backgrnd;
                self.backgrnd2 = data.backgrnd2;
                self.backgrnd3 = data.backgrnd3;
                self.loaded = true;
                info!("Equip window assets loaded successfully");
            }
            Err(e) => {
                error!("Failed to load Equip window assets: {}", e);
                self.loaded = false;
            }
        }
    }

    async fn load_from_wz() -> Result<EquipWindowData, String> {
        let bytes = AssetManager::fetch_and_cache(UIWINDOW2_URL, UIWINDOW2_CACHE).await
            .map_err(|e| format!("Failed to fetch UIWindow2.img: {}", e))?;

        let wz_iv = guess_iv_from_wz_img(&bytes)
            .ok_or_else(|| "Unable to guess WZ version from UIWindow2.img".to_string())?;

        let byte_len = bytes.len();
        let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
        let cache_name_ref: wz_reader::WzNodeName = UIWINDOW2_CACHE.to_string().into();
        let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
        let root_node: WzNodeArc = WzNode::new(&UIWINDOW2_CACHE.to_string().into(), wz_image, None).into();

        root_node.write().unwrap().parse(&root_node)
            .map_err(|e| format!("Failed to parse UIWindow2.img: {:?}", e))?;

        let mut data = EquipWindowData::default();

        // Load background layers in z-order
        data.backgrnd = Self::load_texture(&root_node, "Equip/character/backgrnd").await.ok();
        data.backgrnd2 = Self::load_texture(&root_node, "Equip/character/backgrnd2").await.ok();
        data.backgrnd3 = Self::load_texture(&root_node, "Equip/character/backgrnd3").await.ok();

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

    pub fn update(&mut self) {
        if !self.visible || !self.loaded {
            return;
        }

        let (mouse_x, mouse_y) = mouse_position();

        // Handle dragging
        if is_mouse_button_pressed(MouseButton::Left) {
            // Check if clicking on title bar area (top 30 pixels)
            if mouse_y >= self.y && mouse_y <= self.y + 30.0 &&
               mouse_x >= self.x && mouse_x <= self.x + 300.0 {
                self.dragging = true;
                self.drag_offset_x = mouse_x - self.x;
                self.drag_offset_y = mouse_y - self.y;
            }
        }

        if is_mouse_button_down(MouseButton::Left) && self.dragging {
            self.x = mouse_x - self.drag_offset_x;
            self.y = mouse_y - self.drag_offset_y;
        } else {
            self.dragging = false;
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            // Center window when shown
            self.x = (screen_width() - 300.0) / 2.0;
            self.y = (screen_height() - 400.0) / 2.0;
        }
    }

    pub fn draw(&self) {
        if !self.visible || !self.loaded {
            return;
        }

        // Draw background layers in z-order (backgrnd, backgrnd2, backgrnd3)
        if let Some(bg) = &self.backgrnd {
            draw_texture(&bg.texture, self.x - bg.origin.x, self.y - bg.origin.y, WHITE);
        }
        if let Some(bg2) = &self.backgrnd2 {
            draw_texture(&bg2.texture, self.x - bg2.origin.x, self.y - bg2.origin.y, WHITE);
        }
        if let Some(bg3) = &self.backgrnd3 {
            draw_texture(&bg3.texture, self.x - bg3.origin.x, self.y - bg3.origin.y, WHITE);
        }
    }
}

#[derive(Clone)]
pub struct UserInfoWindow {
    pub visible: bool,
}

impl UserInfoWindow {
    pub fn new() -> Self {
        Self { visible: false }
    }

    pub async fn load_assets(&mut self) {
        // TODO: Load user info window assets
    }

    pub fn update(&mut self) {
        // TODO: Update user info window state
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn draw(&self, _name: &str, _level: u32) {
        if !self.visible {
            return;
        }
        // TODO: Draw user info window
    }
}

#[derive(Default)]
struct InventoryWindowData {
    backgrnd: Option<TextureWithOrigin>,
    backgrnd2: Option<TextureWithOrigin>,
    backgrnd3: Option<TextureWithOrigin>,
}

#[derive(Default)]
struct EquipWindowData {
    backgrnd: Option<TextureWithOrigin>,
    backgrnd2: Option<TextureWithOrigin>,
    backgrnd3: Option<TextureWithOrigin>,
}
