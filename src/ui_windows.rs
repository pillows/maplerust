use macroquad::prelude::*;
use crate::assets::AssetManager;
use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzReader, WzNodeCast, WzObjectType};

const UIWINDOW2_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/UI/UIWindow2.img";
const UIWINDOW2_CACHE: &str = "/01/UI/UIWindow2.img";

#[derive(Clone)]
struct TextureWithOrigin {
    texture: Texture2D,
    origin_x: i32,
    origin_y: i32,
}

pub struct InventoryWindow {
    backgrnd: Option<TextureWithOrigin>,
    backgrnd2: Option<TextureWithOrigin>,
    backgrnd3: Option<TextureWithOrigin>,
    tabs_enabled: Vec<Option<TextureWithOrigin>>,
    tabs_disabled: Vec<Option<TextureWithOrigin>>,
    current_tab: usize,
    visible: bool,
    loaded: bool,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    dragging: bool,
    drag_offset: Vec2,
}

impl InventoryWindow {
    pub fn new() -> Self {
        Self {
            backgrnd: None, backgrnd2: None, backgrnd3: None,
            tabs_enabled: Vec::new(), tabs_disabled: Vec::new(),
            current_tab: 0, visible: false, loaded: false,
            x: 400.0, y: 100.0, width: 172.0, height: 335.0,
            dragging: false, drag_offset: Vec2::ZERO,
        }
    }

    pub async fn load_assets(&mut self) {
        info!("Loading Inventory window assets...");
        match Self::load_from_wz().await {
            Ok(data) => {
                self.backgrnd = data.backgrnd;
                self.backgrnd2 = data.backgrnd2;
                self.backgrnd3 = data.backgrnd3;
                self.tabs_enabled = data.tabs_enabled;
                self.tabs_disabled = data.tabs_disabled;
                if let Some(ref bg) = self.backgrnd {
                    self.width = bg.texture.width();
                    self.height = bg.texture.height();
                }
                self.loaded = true;
                info!("Inventory window assets loaded successfully");
            }
            Err(e) => {
                error!("Failed to load Inventory assets: {}", e);
                self.loaded = false;
            }
        }
    }

    async fn load_from_wz() -> Result<InventoryData, String> {
        let bytes = AssetManager::fetch_and_cache(UIWINDOW2_URL, UIWINDOW2_CACHE).await
            .map_err(|e| format!("Failed to fetch UIWindow2.img: {}", e))?;
        let wz_iv = guess_iv_from_wz_img(&bytes)
            .ok_or_else(|| "Unable to guess WZ version".to_string())?;
        let byte_len = bytes.len();
        let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
        let cache_name_ref: wz_reader::WzNodeName = UIWINDOW2_CACHE.to_string().into();
        let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
        let root_node: WzNodeArc = WzNode::new(&UIWINDOW2_CACHE.to_string().into(), wz_image, None).into();
        root_node.write().unwrap().parse(&root_node)
            .map_err(|e| format!("Failed to parse UIWindow2.img: {:?}", e))?;
        let mut data = InventoryData::default();
        data.backgrnd = load_texture_with_origin(&root_node, "Item/backgrnd").await.ok();
        data.backgrnd2 = load_texture_with_origin(&root_node, "Item/backgrnd2").await.ok();
        data.backgrnd3 = load_texture_with_origin(&root_node, "Item/backgrnd3").await.ok();
        for i in 0..5 {
            let enabled_path = format!("Item/Tab/enabled/{}", i);
            let disabled_path = format!("Item/Tab/disabled/{}", i);
            data.tabs_enabled.push(load_texture_with_origin(&root_node, &enabled_path).await.ok());
            data.tabs_disabled.push(load_texture_with_origin(&root_node, &disabled_path).await.ok());
        }
        Ok(data)
    }

    pub fn toggle(&mut self) { self.visible = !self.visible; }

    pub fn update(&mut self) {
        if !self.visible { return; }
        let (mouse_x, mouse_y) = mouse_position();
        let title_bar_height = 25.0;
        if is_mouse_button_pressed(MouseButton::Left) {
            if mouse_x >= self.x && mouse_x <= self.x + self.width
                && mouse_y >= self.y && mouse_y <= self.y + title_bar_height {
                self.dragging = true;
                self.drag_offset = Vec2::new(mouse_x - self.x, mouse_y - self.y);
            }
        }
        if self.dragging {
            if is_mouse_button_down(MouseButton::Left) {
                self.x = mouse_x - self.drag_offset.x;
                self.y = mouse_y - self.drag_offset.y;
                self.x = self.x.max(0.0).min(screen_width() - self.width);
                self.y = self.y.max(0.0).min(screen_height() - self.height);
            } else { self.dragging = false; }
        }
        if is_mouse_button_pressed(MouseButton::Left) {
            let close_x = self.x + self.width - 18.0;
            let close_y = self.y + 2.0;
            if mouse_x >= close_x && mouse_x <= close_x + 16.0
                && mouse_y >= close_y && mouse_y <= close_y + 16.0 {
                self.visible = false;
            }
        }
        for i in 0..5 {
            if let Some(tab) = self.tabs_enabled.get(i).and_then(|t| t.as_ref()) {
                let tab_x = self.x - tab.origin_x as f32;
                let tab_y = self.y - tab.origin_y as f32;
                let tab_width = tab.texture.width();
                let tab_height = tab.texture.height();
                if is_mouse_button_pressed(MouseButton::Left) {
                    if mouse_x >= tab_x && mouse_x <= tab_x + tab_width
                        && mouse_y >= tab_y && mouse_y <= tab_y + tab_height {
                        self.current_tab = i;
                    }
                }
            }
        }
    }

    pub fn draw(&self) {
        if !self.visible { return; }
        if self.loaded {
            if let Some(bg) = &self.backgrnd {
                draw_texture(&bg.texture, self.x - bg.origin_x as f32, self.y - bg.origin_y as f32, WHITE);
            }
            if let Some(bg2) = &self.backgrnd2 {
                draw_texture(&bg2.texture, self.x - bg2.origin_x as f32, self.y - bg2.origin_y as f32, WHITE);
            }
            if let Some(bg3) = &self.backgrnd3 {
                draw_texture(&bg3.texture, self.x - bg3.origin_x as f32, self.y - bg3.origin_y as f32, WHITE);
            }
            for i in 0..5 {
                let tab = if i == self.current_tab {
                    self.tabs_enabled.get(i).and_then(|t| t.as_ref())
                } else {
                    self.tabs_disabled.get(i).and_then(|t| t.as_ref())
                };
                if let Some(t) = tab {
                    draw_texture(&t.texture, self.x - t.origin_x as f32, self.y - t.origin_y as f32, WHITE);
                }
            }
        } else {
            self.draw_fallback();
        }
        let close_x = self.x + self.width - 18.0;
        let close_y = self.y + 2.0;
        draw_rectangle(close_x, close_y, 16.0, 16.0, Color::from_rgba(100, 50, 50, 150));
        draw_text("X", close_x + 4.0, close_y + 12.0, 14.0, WHITE);
    }

    fn draw_fallback(&self) {
        draw_rectangle(self.x, self.y, self.width, self.height, Color::from_rgba(50, 50, 60, 240));
        draw_rectangle(self.x, self.y, self.width, 20.0, Color::from_rgba(70, 70, 80, 255));
        draw_rectangle_lines(self.x, self.y, self.width, self.height, 1.0, Color::from_rgba(100, 100, 120, 255));
        draw_text("Inventory", self.x + 10.0, self.y + 15.0, 14.0, WHITE);
        let grid_x = self.x + 10.0;
        let grid_y = self.y + 50.0;
        let slot_size = 36.0;
        let padding = 2.0;
        for row in 0..6 {
            for col in 0..4 {
                let slot_x = grid_x + (col as f32 * (slot_size + padding));
                let slot_y = grid_y + (row as f32 * (slot_size + padding));
                draw_rectangle(slot_x, slot_y, slot_size, slot_size, Color::from_rgba(30, 30, 40, 200));
                draw_rectangle_lines(slot_x, slot_y, slot_size, slot_size, 1.0, Color::from_rgba(80, 80, 100, 200));
            }
        }
    }

    pub fn is_visible(&self) -> bool { self.visible }
}

#[derive(Default)]
struct InventoryData {
    backgrnd: Option<TextureWithOrigin>,
    backgrnd2: Option<TextureWithOrigin>,
    backgrnd3: Option<TextureWithOrigin>,
    tabs_enabled: Vec<Option<TextureWithOrigin>>,
    tabs_disabled: Vec<Option<TextureWithOrigin>>,
}

pub struct EquipWindow {
    backgrnd: Option<TextureWithOrigin>,
    backgrnd2: Option<TextureWithOrigin>,
    backgrnd3: Option<TextureWithOrigin>,
    visible: bool,
    loaded: bool,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    dragging: bool,
    drag_offset: Vec2,
}

impl EquipWindow {
    pub fn new() -> Self {
        Self {
            backgrnd: None, backgrnd2: None, backgrnd3: None,
            visible: false, loaded: false,
            x: 200.0, y: 100.0, width: 172.0, height: 290.0,
            dragging: false, drag_offset: Vec2::ZERO,
        }
    }

    pub async fn load_assets(&mut self) {
        info!("Loading Equip window assets...");
        match Self::load_from_wz().await {
            Ok(data) => {
                self.backgrnd = data.backgrnd;
                self.backgrnd2 = data.backgrnd2;
                self.backgrnd3 = data.backgrnd3;
                if let Some(ref bg) = self.backgrnd {
                    self.width = bg.texture.width();
                    self.height = bg.texture.height();
                }
                self.loaded = true;
                info!("Equip window assets loaded successfully");
            }
            Err(e) => {
                error!("Failed to load Equip assets: {}", e);
                self.loaded = false;
            }
        }
    }

    async fn load_from_wz() -> Result<EquipData, String> {
        let bytes = AssetManager::fetch_and_cache(UIWINDOW2_URL, UIWINDOW2_CACHE).await
            .map_err(|e| format!("Failed to fetch UIWindow2.img: {}", e))?;
        let wz_iv = guess_iv_from_wz_img(&bytes)
            .ok_or_else(|| "Unable to guess WZ version".to_string())?;
        let byte_len = bytes.len();
        let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
        let cache_name_ref: wz_reader::WzNodeName = UIWINDOW2_CACHE.to_string().into();
        let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
        let root_node: WzNodeArc = WzNode::new(&UIWINDOW2_CACHE.to_string().into(), wz_image, None).into();
        root_node.write().unwrap().parse(&root_node)
            .map_err(|e| format!("Failed to parse UIWindow2.img: {:?}", e))?;
        let mut data = EquipData::default();
        data.backgrnd = load_texture_with_origin(&root_node, "Equip/character/backgrnd").await.ok();
        data.backgrnd2 = load_texture_with_origin(&root_node, "Equip/character/backgrnd2").await.ok();
        data.backgrnd3 = load_texture_with_origin(&root_node, "Equip/character/backgrnd3").await.ok();
        Ok(data)
    }

    pub fn toggle(&mut self) { self.visible = !self.visible; }

    pub fn update(&mut self) {
        if !self.visible { return; }
        let (mouse_x, mouse_y) = mouse_position();
        let title_bar_height = 25.0;
        if is_mouse_button_pressed(MouseButton::Left) {
            if mouse_x >= self.x && mouse_x <= self.x + self.width
                && mouse_y >= self.y && mouse_y <= self.y + title_bar_height {
                self.dragging = true;
                self.drag_offset = Vec2::new(mouse_x - self.x, mouse_y - self.y);
            }
        }
        if self.dragging {
            if is_mouse_button_down(MouseButton::Left) {
                self.x = mouse_x - self.drag_offset.x;
                self.y = mouse_y - self.drag_offset.y;
                self.x = self.x.max(0.0).min(screen_width() - self.width);
                self.y = self.y.max(0.0).min(screen_height() - self.height);
            } else { self.dragging = false; }
        }
        if is_mouse_button_pressed(MouseButton::Left) {
            let close_x = self.x + self.width - 18.0;
            let close_y = self.y + 2.0;
            if mouse_x >= close_x && mouse_x <= close_x + 16.0
                && mouse_y >= close_y && mouse_y <= close_y + 16.0 {
                self.visible = false;
            }
        }
    }

    pub fn draw(&self) {
        if !self.visible { return; }
        if self.loaded {
            if let Some(bg) = &self.backgrnd {
                draw_texture(&bg.texture, self.x - bg.origin_x as f32, self.y - bg.origin_y as f32, WHITE);
            }
            if let Some(bg2) = &self.backgrnd2 {
                draw_texture(&bg2.texture, self.x - bg2.origin_x as f32, self.y - bg2.origin_y as f32, WHITE);
            }
            if let Some(bg3) = &self.backgrnd3 {
                draw_texture(&bg3.texture, self.x - bg3.origin_x as f32, self.y - bg3.origin_y as f32, WHITE);
            }
        } else {
            self.draw_fallback();
        }
        let close_x = self.x + self.width - 18.0;
        let close_y = self.y + 2.0;
        draw_rectangle(close_x, close_y, 16.0, 16.0, Color::from_rgba(100, 50, 50, 150));
        draw_text("X", close_x + 4.0, close_y + 12.0, 14.0, WHITE);
    }

    fn draw_fallback(&self) {
        draw_rectangle(self.x, self.y, self.width, self.height, Color::from_rgba(50, 50, 60, 240));
        draw_rectangle(self.x, self.y, self.width, 20.0, Color::from_rgba(70, 70, 80, 255));
        draw_rectangle_lines(self.x, self.y, self.width, self.height, 1.0, Color::from_rgba(100, 100, 120, 255));
        draw_text("Equipment", self.x + 10.0, self.y + 15.0, 14.0, WHITE);
        let center_x = self.x + self.width / 2.0;
        let start_y = self.y + 35.0;
        let slot_size = 36.0;
        let slots = [("Hat", 0.0, 0.0), ("Face", -40.0, 40.0), ("Eye", 40.0, 40.0),
                     ("Top", 0.0, 80.0), ("Glove", -40.0, 120.0), ("Weapon", 40.0, 120.0)];
        for (name, offset_x, offset_y) in slots {
            let slot_x = center_x + offset_x - slot_size / 2.0;
            let slot_y = start_y + offset_y;
            draw_rectangle(slot_x, slot_y, slot_size, slot_size, Color::from_rgba(30, 30, 40, 200));
            draw_rectangle_lines(slot_x, slot_y, slot_size, slot_size, 1.0, Color::from_rgba(80, 80, 100, 200));
            let text_width = measure_text(name, None, 10, 1.0).width;
            draw_text(name, slot_x + (slot_size - text_width) / 2.0, slot_y + slot_size + 12.0, 10.0, LIGHTGRAY);
        }
    }

    pub fn is_visible(&self) -> bool { self.visible }
}

#[derive(Default)]
struct EquipData {
    backgrnd: Option<TextureWithOrigin>,
    backgrnd2: Option<TextureWithOrigin>,
    backgrnd3: Option<TextureWithOrigin>,
}

async fn load_texture_with_origin(root_node: &WzNodeArc, path: &str) -> Result<TextureWithOrigin, String> {
    let node = root_node.read().unwrap().at_path_parsed(path)
        .map_err(|e| format!("Path '{}' not found: {:?}", path, e))?;
    node.write().unwrap().parse(&node)
        .map_err(|e| format!("Failed to parse node at '{}': {:?}", path, e))?;
    let node_read = node.read().unwrap();
    let png = node_read.try_as_png()
        .ok_or_else(|| format!("Node at '{}' is not a PNG", path))?;
    let (origin_x, origin_y) = if let Ok(origin_node) = node_read.at_path_parsed("origin") {
        let origin_read = origin_node.read().unwrap();
        match &origin_read.object_type {
            WzObjectType::Value(wz_reader::property::WzValue::Vector(vec)) => (vec.0, vec.1),
            _ => (0, 0),
        }
    } else { (0, 0) };
    let png_data = png.extract_png()
        .map_err(|e| format!("Failed to extract PNG at '{}': {:?}", path, e))?;
    let rgba_img = png_data.to_rgba8();
    let width = rgba_img.width() as u16;
    let height = rgba_img.height() as u16;
    let bytes = rgba_img.into_raw();
    let texture = Texture2D::from_rgba8(width, height, &bytes);
    texture.set_filter(FilterMode::Nearest);
    Ok(TextureWithOrigin { texture, origin_x, origin_y })
}

impl Default for InventoryWindow { fn default() -> Self { Self::new() } }
impl Default for EquipWindow { fn default() -> Self { Self::new() } }
