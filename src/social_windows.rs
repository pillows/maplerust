use macroquad::prelude::*;
use crate::assets::AssetManager;
use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzReader, WzNodeCast};

const UIWINDOW2_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/UI/UIWindow2.img";
const UIWINDOW2_CACHE: &str = "/01/UI/UIWindow2.img";

struct Tex { texture: Texture2D, origin: Vec2 }

async fn load_tex(root: &WzNodeArc, path: &str) -> Option<Tex> {
    let node = root.read().unwrap().at_path_parsed(path).ok()?;
    node.write().unwrap().parse(&node).ok()?;
    let r = node.read().unwrap();
    let png = r.try_as_png()?;
    let data = png.extract_png().ok()?.to_rgba8();
    let tex = Texture2D::from_rgba8(data.width() as u16, data.height() as u16, &data.into_raw());
    let origin = r.children.get("origin").and_then(|o| {
        o.read().unwrap().try_as_vector2d().map(|v| Vec2::new(v.0 as f32, v.1 as f32))
    }).unwrap_or(Vec2::ZERO);
    Some(Tex { texture: tex, origin })
}

async fn load_wz() -> Option<WzNodeArc> {
    let bytes = AssetManager::fetch_and_cache(UIWINDOW2_URL, UIWINDOW2_CACHE).await.ok()?;
    let iv = guess_iv_from_wz_img(&bytes)?;
    let len = bytes.len();
    let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(iv));
    let name: wz_reader::WzNodeName = UIWINDOW2_CACHE.to_string().into();
    let img = WzImage::new(&name, 0, len, &reader);
    let root: WzNodeArc = WzNode::new(&UIWINDOW2_CACHE.into(), img, None).into();
    root.write().unwrap().parse(&root).ok()?;
    Some(root)
}

/// Channel selection window
pub struct ChannelWindow {
    visible: bool,
    bg: Option<Tex>,
    bg2: Option<Tex>,
    bg3: Option<Tex>,
    x: f32, y: f32, w: f32, h: f32,
    dragging: bool, drag_off: Vec2,
    selected_channel: u8,
    loaded: bool,
}

impl ChannelWindow {
    pub fn new() -> Self {
        Self { visible: false, bg: None, bg2: None, bg3: None, x: 200.0, y: 150.0, w: 400.0, h: 170.0,
               dragging: false, drag_off: Vec2::ZERO, selected_channel: 1, loaded: false }
    }
    pub async fn load(&mut self) {
        if let Some(root) = load_wz().await {
            // Load in z-order: backgrnd (z=-1), backgrnd2 (z=0), backgrnd3 (z=1)
            self.bg = load_tex(&root, "Channel/backgrnd").await;
            self.bg2 = load_tex(&root, "Channel/backgrnd2").await;
            self.bg3 = load_tex(&root, "Channel/backgrnd3").await;
            if let Some(ref b) = self.bg { 
                self.w = b.texture.width(); 
                self.h = b.texture.height(); 
                self.loaded = true;
                info!("Channel window loaded: {}x{}", self.w, self.h);
            }
        }
    }
    pub fn show(&mut self) { self.visible = true; self.x = (screen_width() - self.w) / 2.0; self.y = (screen_height() - self.h) / 2.0; }
    pub fn hide(&mut self) { self.visible = false; }
    pub fn toggle(&mut self) { if self.visible { self.hide() } else { self.show() } }
    pub fn is_visible(&self) -> bool { self.visible }
    pub fn update(&mut self) {
        if !self.visible { return; }
        let (mx, my) = mouse_position();
        if is_mouse_button_pressed(MouseButton::Left) && mx >= self.x && mx <= self.x + self.w && my >= self.y && my <= self.y + 25.0 {
            self.dragging = true; self.drag_off = Vec2::new(mx - self.x, my - self.y);
        }
        if self.dragging {
            if is_mouse_button_down(MouseButton::Left) {
                self.x = (mx - self.drag_off.x).max(0.0).min(screen_width() - self.w);
                self.y = (my - self.drag_off.y).max(0.0).min(screen_height() - self.h);
            } else { self.dragging = false; }
        }
        // Handle channel selection clicks
        if is_mouse_button_pressed(MouseButton::Left) {
            for i in 1..=20u8 {
                let col = ((i - 1) % 5) as f32;
                let row = ((i - 1) / 5) as f32;
                let bx = self.x + 20.0 + col * 70.0;
                let by = self.y + 50.0 + row * 25.0;
                if mx >= bx && mx <= bx + 60.0 && my >= by - 12.0 && my <= by + 8.0 {
                    self.selected_channel = i;
                    info!("Selected channel {}", i);
                    break;
                }
            }
        }
    }
    pub fn draw(&self) {
        if !self.visible { return; }
        // Draw backgrounds in z-order
        if let Some(ref b) = self.bg { draw_texture(&b.texture, self.x - b.origin.x, self.y - b.origin.y, WHITE); }
        else { draw_rectangle(self.x, self.y, self.w, self.h, Color::from_rgba(40, 40, 50, 240)); }
        if let Some(ref b) = self.bg2 { draw_texture(&b.texture, self.x - b.origin.x, self.y - b.origin.y, WHITE); }
        if let Some(ref b) = self.bg3 { draw_texture(&b.texture, self.x - b.origin.x, self.y - b.origin.y, WHITE); }
        
        draw_text("Channel Selection", self.x + 10.0, self.y + 25.0, 16.0, WHITE);
        for i in 1..=20u8 {
            let col = ((i - 1) % 5) as f32;
            let row = ((i - 1) / 5) as f32;
            let bx = self.x + 20.0 + col * 70.0;
            let by = self.y + 50.0 + row * 25.0;
            let color = if i == self.selected_channel { YELLOW } else { WHITE };
            // Draw channel button background
            let bg_color = if i == self.selected_channel { 
                Color::from_rgba(80, 80, 120, 200) 
            } else { 
                Color::from_rgba(50, 50, 70, 150) 
            };
            draw_rectangle(bx - 2.0, by - 12.0, 64.0, 20.0, bg_color);
            draw_text(&format!("Ch {}", i), bx, by, 14.0, color);
        }
    }
}
impl Default for ChannelWindow { fn default() -> Self { Self::new() } }

/// Megaphone window
pub struct MegaphoneWindow {
    visible: bool,
    bg: Option<Tex>,
    bg_super: Option<Tex>,
    x: f32, y: f32, w: f32, h: f32,
    dragging: bool, drag_off: Vec2,
    message: String,
    loaded: bool,
}

impl MegaphoneWindow {
    pub fn new() -> Self {
        Self { visible: false, bg: None, bg_super: None, x: 200.0, y: 200.0, w: 400.0, h: 100.0,
               dragging: false, drag_off: Vec2::ZERO, message: String::new(), loaded: false }
    }
    pub async fn load(&mut self) {
        if let Some(root) = load_wz().await {
            // Try Megaphone/backgrnd first
            self.bg = load_tex(&root, "Megaphone/backgrnd").await;
            self.bg_super = load_tex(&root, "Megaphone/backgrnd_super").await;
            if let Some(ref b) = self.bg { 
                self.w = b.texture.width(); 
                self.h = b.texture.height(); 
                self.loaded = true;
                info!("Megaphone window loaded: {}x{}", self.w, self.h);
            }
        }
    }
    pub fn show(&mut self) { self.visible = true; self.x = (screen_width() - self.w) / 2.0; self.y = (screen_height() - self.h) / 2.0; }
    pub fn hide(&mut self) { self.visible = false; }
    pub fn toggle(&mut self) { if self.visible { self.hide() } else { self.show() } }
    pub fn is_visible(&self) -> bool { self.visible }
    pub fn update(&mut self) {
        if !self.visible { return; }
        let (mx, my) = mouse_position();
        if is_mouse_button_pressed(MouseButton::Left) && mx >= self.x && mx <= self.x + self.w && my >= self.y && my <= self.y + 25.0 {
            self.dragging = true; self.drag_off = Vec2::new(mx - self.x, my - self.y);
        }
        if self.dragging {
            if is_mouse_button_down(MouseButton::Left) {
                self.x = (mx - self.drag_off.x).max(0.0).min(screen_width() - self.w);
                self.y = (my - self.drag_off.y).max(0.0).min(screen_height() - self.h);
            } else { self.dragging = false; }
        }
    }
    pub fn draw(&self) {
        if !self.visible { return; }
        if let Some(ref b) = self.bg { 
            draw_texture(&b.texture, self.x - b.origin.x, self.y - b.origin.y, WHITE); 
        } else { 
            // Fallback UI
            draw_rectangle(self.x, self.y, self.w, self.h, Color::from_rgba(40, 40, 50, 240)); 
            draw_rectangle_lines(self.x, self.y, self.w, self.h, 2.0, Color::from_rgba(100, 100, 140, 255));
        }
        // Draw title and input hint
        draw_text("Megaphone", self.x + 10.0, self.y + 20.0, 16.0, WHITE);
        draw_text("Press T to toggle | Type your message:", self.x + 10.0, self.y + 45.0, 12.0, GRAY);
        // Draw input area
        draw_rectangle(self.x + 10.0, self.y + 55.0, self.w - 20.0, 25.0, Color::from_rgba(255, 255, 255, 200));
        draw_text(&self.message, self.x + 15.0, self.y + 72.0, 12.0, BLACK);
    }
}
impl Default for MegaphoneWindow { fn default() -> Self { Self::new() } }

/// Memo window
pub struct MemoWindow {
    visible: bool,
    bg: Option<Tex>,
    bg2: Option<Tex>,
    x: f32, y: f32, w: f32, h: f32,
    dragging: bool, drag_off: Vec2,
}

impl MemoWindow {
    pub fn new() -> Self {
        Self { visible: false, bg: None, bg2: None, x: 200.0, y: 150.0, w: 300.0, h: 200.0,
               dragging: false, drag_off: Vec2::ZERO }
    }
    pub async fn load(&mut self) {
        if let Some(root) = load_wz().await {
            self.bg = load_tex(&root, "Memo/Get/backgrnd").await;
            self.bg2 = load_tex(&root, "Memo/Get/backgrnd2").await;
            if let Some(ref b) = self.bg { self.w = b.texture.width(); self.h = b.texture.height(); }
        }
    }
    pub fn show(&mut self) { self.visible = true; self.x = (screen_width() - self.w) / 2.0; self.y = (screen_height() - self.h) / 2.0; }
    pub fn hide(&mut self) { self.visible = false; }
    pub fn toggle(&mut self) { if self.visible { self.hide() } else { self.show() } }
    pub fn is_visible(&self) -> bool { self.visible }
    pub fn update(&mut self) {
        if !self.visible { return; }
        let (mx, my) = mouse_position();
        if is_mouse_button_pressed(MouseButton::Left) && mx >= self.x && mx <= self.x + self.w && my >= self.y && my <= self.y + 25.0 {
            self.dragging = true; self.drag_off = Vec2::new(mx - self.x, my - self.y);
        }
        if self.dragging {
            if is_mouse_button_down(MouseButton::Left) {
                self.x = (mx - self.drag_off.x).max(0.0).min(screen_width() - self.w);
                self.y = (my - self.drag_off.y).max(0.0).min(screen_height() - self.h);
            } else { self.dragging = false; }
        }
        if is_key_pressed(KeyCode::Escape) { self.visible = false; }
    }
    pub fn draw(&self) {
        if !self.visible { return; }
        if let Some(ref b) = self.bg { draw_texture(&b.texture, self.x - b.origin.x, self.y - b.origin.y, WHITE); }
        else { draw_rectangle(self.x, self.y, self.w, self.h, Color::from_rgba(40, 40, 50, 240)); }
        if let Some(ref b) = self.bg2 { draw_texture(&b.texture, self.x - b.origin.x, self.y - b.origin.y, WHITE); }
        draw_text("Memo", self.x + 10.0, self.y + 40.0, 16.0, WHITE);
        draw_text("No new memos", self.x + 10.0, self.y + 70.0, 14.0, GRAY);
    }
}
impl Default for MemoWindow { fn default() -> Self { Self::new() } }

/// Messenger window
pub struct MessengerWindow {
    visible: bool,
    bg: Option<Tex>,
    bg2: Option<Tex>,
    bg3: Option<Tex>,
    x: f32, y: f32, w: f32, h: f32,
    dragging: bool, drag_off: Vec2,
}

impl MessengerWindow {
    pub fn new() -> Self {
        Self { visible: false, bg: None, bg2: None, bg3: None, x: 200.0, y: 150.0, w: 300.0, h: 230.0,
               dragging: false, drag_off: Vec2::ZERO }
    }
    pub async fn load(&mut self) {
        if let Some(root) = load_wz().await {
            self.bg = load_tex(&root, "Messenger/Min/backgrnd").await;
            self.bg2 = load_tex(&root, "Messenger/Min/backgrnd2").await;
            self.bg3 = load_tex(&root, "Messenger/Min/backgrnd3").await;
            if let Some(ref b) = self.bg { self.w = b.texture.width(); self.h = b.texture.height(); }
        }
    }
    pub fn show(&mut self) { self.visible = true; self.x = (screen_width() - self.w) / 2.0; self.y = (screen_height() - self.h) / 2.0; }
    pub fn hide(&mut self) { self.visible = false; }
    pub fn toggle(&mut self) { if self.visible { self.hide() } else { self.show() } }
    pub fn is_visible(&self) -> bool { self.visible }
    pub fn update(&mut self) {
        if !self.visible { return; }
        let (mx, my) = mouse_position();
        if is_mouse_button_pressed(MouseButton::Left) && mx >= self.x && mx <= self.x + self.w && my >= self.y && my <= self.y + 25.0 {
            self.dragging = true; self.drag_off = Vec2::new(mx - self.x, my - self.y);
        }
        if self.dragging {
            if is_mouse_button_down(MouseButton::Left) {
                self.x = (mx - self.drag_off.x).max(0.0).min(screen_width() - self.w);
                self.y = (my - self.drag_off.y).max(0.0).min(screen_height() - self.h);
            } else { self.dragging = false; }
        }
        if is_key_pressed(KeyCode::Escape) { self.visible = false; }
    }
    pub fn draw(&self) {
        if !self.visible { return; }
        if let Some(ref b) = self.bg { draw_texture(&b.texture, self.x - b.origin.x, self.y - b.origin.y, WHITE); }
        else { draw_rectangle(self.x, self.y, self.w, self.h, Color::from_rgba(40, 40, 50, 240)); }
        if let Some(ref b) = self.bg2 { draw_texture(&b.texture, self.x - b.origin.x, self.y - b.origin.y, WHITE); }
        if let Some(ref b) = self.bg3 { draw_texture(&b.texture, self.x - b.origin.x, self.y - b.origin.y, WHITE); }
        draw_text("Messenger", self.x + 10.0, self.y + 40.0, 16.0, WHITE);
        draw_text("No friends online", self.x + 10.0, self.y + 70.0, 14.0, GRAY);
    }
}
impl Default for MessengerWindow { fn default() -> Self { Self::new() } }
