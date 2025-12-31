use macroquad::prelude::*;
use crate::assets::AssetManager;
use std::sync::Arc;
use std::collections::HashMap;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzReader, WzNodeCast};

const UIWINDOW2_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/UI/UIWindow2.img";
const UIWINDOW2_CACHE: &str = "/01/UI/UIWindow2.img";

#[derive(Clone)]
struct TextureWithOrigin {
    texture: Texture2D,
    origin: Vec2,
}

/// Item data for inventory display
#[derive(Clone)]
struct ItemIcon {
    texture: Texture2D,
    item_id: String,
}

#[derive(Clone)]
pub struct InventoryWindow {
    pub visible: bool,
    loaded: bool,
    backgrnd: Option<TextureWithOrigin>,
    backgrnd2: Option<TextureWithOrigin>,
    backgrnd3: Option<TextureWithOrigin>,
    // Tabs: 0=Equip, 1=Use, 2=Etc, 3=Setup, 4=Cash
    tabs_enabled: [Option<TextureWithOrigin>; 5],
    tabs_disabled: [Option<TextureWithOrigin>; 5],
    selected_tab: usize,
    x: f32,
    y: f32,
    dragging: bool,
    drag_offset_x: f32,
    drag_offset_y: f32,
    // Item icons
    items: Vec<ItemIcon>,
    items_loaded: bool,
}

impl InventoryWindow {
    pub fn new() -> Self {
        Self {
            visible: false,
            loaded: false,
            backgrnd: None,
            backgrnd2: None,
            backgrnd3: None,
            tabs_enabled: [None, None, None, None, None],
            tabs_disabled: [None, None, None, None, None],
            selected_tab: 0,
            x: 100.0,
            y: 100.0,
            dragging: false,
            drag_offset_x: 0.0,
            drag_offset_y: 0.0,
            items: Vec::new(),
            items_loaded: false,
        }
    }

    pub async fn load_assets(&mut self) {
        match Self::load_from_wz().await {
            Ok(data) => {
                self.backgrnd = data.backgrnd;
                self.backgrnd2 = data.backgrnd2;
                self.backgrnd3 = data.backgrnd3;
                self.tabs_enabled = data.tabs_enabled;
                self.tabs_disabled = data.tabs_disabled;
                self.loaded = true;
                info!("Inventory window assets loaded successfully");
            }
            Err(e) => {
                error!("Failed to load Inventory window assets: {}", e);
                self.loaded = false;
            }
        }
        
        // Load some item icons
        self.load_items().await;
    }

    async fn load_items(&mut self) {
        // Try to load items from 0501.img
        let url = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/Item/Cash/0501.img";
        let cache = "/01/Item/Cash/0501.img";
        
        if let Ok(bytes) = AssetManager::fetch_and_cache(url, cache).await {
            if let Some(wz_iv) = guess_iv_from_wz_img(&bytes) {
                let byte_len = bytes.len();
                let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
                let cache_name_ref: wz_reader::WzNodeName = cache.to_string().into();
                let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
                let root_node: WzNodeArc = WzNode::new(&cache.to_string().into(), wz_image, None).into();

                if root_node.write().unwrap().parse(&root_node).is_ok() {
                    // Get first few item IDs
                    let item_ids: Vec<String> = {
                        let root_read = root_node.read().unwrap();
                        root_read.children.keys()
                            .filter(|k| k.as_str().starts_with("0501"))
                            .take(8)
                            .map(|k| k.to_string())
                            .collect()
                    };

                    for item_id in item_ids {
                        let icon_path = format!("{}/info/icon", item_id);
                        if let Some(tex) = Self::load_item_icon(&root_node, &icon_path).await {
                            self.items.push(ItemIcon { texture: tex, item_id });
                        }
                    }
                    self.items_loaded = true;
                    info!("Loaded {} item icons", self.items.len());
                }
            }
        }
    }

    async fn load_item_icon(root: &WzNodeArc, path: &str) -> Option<Texture2D> {
        let node = root.read().unwrap().at_path(path)?.clone();
        node.write().unwrap().parse(&node).ok()?;
        let node_read = node.read().unwrap();
        let png = node_read.try_as_png()?;
        let img = png.extract_png().ok()?;
        let rgba = img.to_rgba8();
        Some(Texture2D::from_rgba8(rgba.width() as u16, rgba.height() as u16, &rgba.into_raw()))
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

        // Load tabs (0-4): enabled and disabled states
        for i in 0..5 {
            data.tabs_enabled[i] = Self::load_texture(&root_node, &format!("Item/Tab/enabled/{}", i)).await.ok();
            data.tabs_disabled[i] = Self::load_texture(&root_node, &format!("Item/Tab/disabled/{}", i)).await.ok();
        }

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

        // Handle tab clicks - simple fixed positions
        if is_mouse_button_pressed(MouseButton::Left) {
            // Tabs are near the top of the window
            let tab_y_start = self.y + 5.0;
            let tab_y_end = self.y + 30.0;
            
            if mouse_y >= tab_y_start && mouse_y <= tab_y_end {
                // 5 tabs, each about 30 pixels wide, starting at x + 7
                let tab_start_x = self.x + 7.0;
                let tab_width = 30.0;
                
                for i in 0..5 {
                    let tab_x = tab_start_x + (i as f32 * tab_width);
                    if mouse_x >= tab_x && mouse_x <= tab_x + tab_width {
                        self.selected_tab = i;
                        info!("Selected inventory tab {}", i);
                        break;
                    }
                }
            }
        }

        // Handle dragging - only on the right side of the title bar
        if is_mouse_button_pressed(MouseButton::Left) {
            if mouse_y >= self.y && mouse_y <= self.y + 20.0 &&
               mouse_x >= self.x + 160.0 {
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

        // Draw tabs - selected tab uses enabled texture, others use disabled
        for i in 0..5 {
            let tab = if i == self.selected_tab {
                &self.tabs_enabled[i]
            } else {
                &self.tabs_disabled[i]
            };
            if let Some(t) = tab {
                draw_texture(&t.texture, self.x - t.origin.x, self.y - t.origin.y, WHITE);
            }
        }

        // Draw item icons in a grid (4 columns, starting after some padding)
        let slot_size = 32.0;
        let slot_gap = 4.0;
        let start_x = self.x + 12.0;
        let start_y = self.y + 52.0;
        let cols = 4;

        for (i, item) in self.items.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let ix = start_x + col as f32 * (slot_size + slot_gap);
            let iy = start_y + row as f32 * (slot_size + slot_gap);
            draw_texture(&item.texture, ix, iy, WHITE);
        }

        // Draw currency at bottom of window
        let currency_y = self.y + 200.0;
        draw_text("Mesos:", self.x + 10.0, currency_y, 12.0, WHITE);
        draw_text("1,234,567", self.x + 60.0, currency_y, 12.0, YELLOW);
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
    x: f32,
    y: f32,
}

impl UserInfoWindow {
    pub fn new() -> Self {
        Self { visible: false, x: 200.0, y: 200.0 }
    }

    pub async fn load_assets(&mut self) {
        // TODO: Load user info window assets
    }

    pub fn update(&mut self) {
        if !self.visible { return; }
        
        // Close on ESC or click outside
        if is_key_pressed(KeyCode::Escape) {
            self.visible = false;
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
        // Center on screen
        self.x = (screen_width() - 200.0) / 2.0;
        self.y = (screen_height() - 150.0) / 2.0;
    }

    pub fn draw(&self, name: &str, level: u32) {
        if !self.visible { return; }
        
        let width = 200.0;
        let height = 150.0;
        
        // Draw background
        draw_rectangle(self.x, self.y, width, height, Color::from_rgba(40, 40, 60, 240));
        draw_rectangle_lines(self.x, self.y, width, height, 2.0, Color::from_rgba(100, 100, 140, 255));
        
        // Draw title
        draw_text("Character Info", self.x + 10.0, self.y + 25.0, 18.0, WHITE);
        draw_line(self.x + 5.0, self.y + 35.0, self.x + width - 5.0, self.y + 35.0, 1.0, GRAY);
        
        // Draw info
        draw_text(&format!("Name: {}", name), self.x + 15.0, self.y + 60.0, 14.0, WHITE);
        draw_text(&format!("Level: {}", level), self.x + 15.0, self.y + 80.0, 14.0, WHITE);
        draw_text("Job: Beginner", self.x + 15.0, self.y + 100.0, 14.0, WHITE);
        
        // Draw close hint
        draw_text("Press ESC to close", self.x + 10.0, self.y + height - 15.0, 10.0, GRAY);
    }
}

#[derive(Default)]
struct InventoryWindowData {
    backgrnd: Option<TextureWithOrigin>,
    backgrnd2: Option<TextureWithOrigin>,
    backgrnd3: Option<TextureWithOrigin>,
    tabs_enabled: [Option<TextureWithOrigin>; 5],
    tabs_disabled: [Option<TextureWithOrigin>; 5],
}

#[derive(Default)]
struct EquipWindowData {
    backgrnd: Option<TextureWithOrigin>,
    backgrnd2: Option<TextureWithOrigin>,
    backgrnd3: Option<TextureWithOrigin>,
}
