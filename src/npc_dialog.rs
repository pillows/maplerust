use macroquad::prelude::*;
use crate::assets::AssetManager;
use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzReader, WzNodeCast};

const UIWINDOW2_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/UI/UIWindow2.img";
const UIWINDOW2_CACHE: &str = "/01/UI/UIWindow2.img";

/// Texture with origin point
#[derive(Clone)]
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

/// Simple button for dialog navigation
#[derive(Default, Clone)]
struct DialogButton {
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

impl DialogButton {
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
        // Use button's own x/y position if set, otherwise use base_x/base_y
        let draw_x = if self.x != 0.0 || self.y != 0.0 {
            self.x - self.origin.x
        } else {
            base_x - self.origin.x
        };
        let draw_y = if self.x != 0.0 || self.y != 0.0 {
            self.y - self.origin.y
        } else {
            base_y - self.origin.y
        };
        
        let (mouse_x, mouse_y) = mouse_position();
        let in_bounds = mouse_x >= draw_x && mouse_x <= draw_x + self.width
            && mouse_y >= draw_y && mouse_y <= draw_y + self.height;
        in_bounds && is_mouse_button_released(MouseButton::Left)
    }

    fn draw(&self, base_x: f32, base_y: f32) {
        // Use button's own x/y position if set, otherwise use base_x/base_y
        let draw_x = if self.x != 0.0 || self.y != 0.0 {
            self.x - self.origin.x
        } else {
            base_x - self.origin.x
        };
        let draw_y = if self.x != 0.0 || self.y != 0.0 {
            self.y - self.origin.y
        } else {
            base_y - self.origin.y
        };
        
        let texture = match self.state {
            ButtonState::MouseOver if self.mouse_over.is_some() => &self.mouse_over,
            ButtonState::Pressed if self.pressed.is_some() => &self.pressed,
            _ => &self.normal,
        };

        if let Some(tex) = texture {
            draw_texture(tex, draw_x, draw_y, WHITE);
        }
    }
}

/// Active NPC dialog with multiple pages
pub struct ActiveNpcDialog {
    pub pages: Vec<String>,  // Dialog text pages
    pub current_page: usize,
    pub npc_x: f32,
    pub npc_y: f32,
    pub visible: bool,
}

/// NPC Dialog window system (similar to inventory/equipment windows)
pub struct NpcDialogSystem {
    loaded: bool,
    // Background layers (z-order: backgrnd, backgrnd2, backgrnd3)
    backgrnd: Option<TextureWithOrigin>,
    backgrnd2: Option<TextureWithOrigin>,
    backgrnd3: Option<TextureWithOrigin>,
    // Navigation buttons
    btn_prev: DialogButton,
    btn_next: DialogButton,
    btn_ok: DialogButton,
    // Window position
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    // Active dialog
    active_dialog: Option<ActiveNpcDialog>,
    font: Option<Font>,
    dragging: bool,
    drag_offset_x: f32,
    drag_offset_y: f32,
}

impl NpcDialogSystem {
    pub fn new() -> Self {
        Self {
            loaded: false,
            backgrnd: None,
            backgrnd2: None,
            backgrnd3: None,
            btn_prev: DialogButton::new(),
            btn_next: DialogButton::new(),
            btn_ok: DialogButton::new(),
            x: 200.0,
            y: 200.0,
            width: 400.0,
            height: 300.0,
            active_dialog: None,
            font: None,
            dragging: false,
            drag_offset_x: 0.0,
            drag_offset_y: 0.0,
        }
    }

    /// Load NPC dialog assets from UIWindow2.img
    /// Using a general window background structure similar to Item/Equip
    pub async fn load_assets(&mut self) {
        info!("Loading NPC dialog window assets...");
        
        match Self::load_from_wz().await {
            Ok(data) => {
                self.backgrnd = data.backgrnd;
                self.backgrnd2 = data.backgrnd2;
                self.backgrnd3 = data.backgrnd3;
                self.btn_prev = data.btn_prev;
                self.btn_next = data.btn_next;
                self.btn_ok = data.btn_ok;
                
                // Set window size from background if available
                if let Some(ref bg) = self.backgrnd {
                    self.width = bg.texture.width();
                    self.height = bg.texture.height();
                }
                
                self.loaded = true;
                info!("NPC dialog window assets loaded successfully");
            }
            Err(e) => {
                error!("Failed to load NPC dialog window assets: {}", e);
            }
        }

        // Load font
        match load_ttf_font("https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/MaplestoryLight.ttf").await {
            Ok(font) => {
                self.font = Some(font);
            }
            Err(e) => {
                info!("Failed to load font for NPC dialogs: {:?}", e);
            }
        }
    }

    async fn load_from_wz() -> Result<NpcDialogData, String> {
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

        let mut data = NpcDialogData::default();

        // Try to load from Item window structure (similar window style)
        // Use Item/backgrnd as a template since NPC dialogs might not have dedicated assets
        data.backgrnd = Self::load_texture(&root_node, "Item/backgrnd").await.ok();
        data.backgrnd2 = Self::load_texture(&root_node, "Item/backgrnd2").await.ok();
        data.backgrnd3 = Self::load_texture(&root_node, "Item/backgrnd3").await.ok();

        // Load navigation buttons (try to find or create placeholder buttons)
        // For now, we'll create simple buttons
        data.btn_prev = Self::load_button(&root_node, "Item/BtSort").await.unwrap_or_else(|_| DialogButton::new());
        data.btn_next = Self::load_button(&root_node, "Item/BtFull").await.unwrap_or_else(|_| DialogButton::new());
        data.btn_ok = Self::load_button(&root_node, "KeyConfig/BtOK").await.unwrap_or_else(|_| DialogButton::new());

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

    async fn load_button(root_node: &WzNodeArc, base_path: &str) -> Result<DialogButton, String> {
        let mut btn = DialogButton::new();
        
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
        
        Ok(btn)
    }

    /// Show an NPC dialog
    pub fn show_dialog(&mut self, text: &str, npc_x: f32, npc_y: f32) {
        // Split text into pages (simple splitting by sentences for now)
        let pages: Vec<String> = text.split('.').filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().to_string() + ".")
            .collect();
        
        let pages = if pages.is_empty() {
            vec![text.to_string()]
        } else {
            pages
        };

        self.active_dialog = Some(ActiveNpcDialog {
            pages,
            current_page: 0,
            npc_x,
            npc_y,
            visible: true,
        });

        // Center window when shown
        self.x = (screen_width() - self.width) / 2.0;
        self.y = (screen_height() - self.height) / 2.0;
    }

    /// Close the current dialog
    pub fn close_dialog(&mut self) {
        self.active_dialog = None;
    }

    /// Check if dialog is visible
    pub fn is_visible(&self) -> bool {
        self.active_dialog.is_some()
    }

    /// Update dialog (handle input and buttons)
    pub fn update(&mut self) {
        let dialog_visible = self.active_dialog.as_ref().map(|d| d.visible).unwrap_or(false);
        if !dialog_visible {
            return;
        }

        // Handle dragging
        let (mouse_x, mouse_y) = mouse_position();
        if is_mouse_button_pressed(MouseButton::Left) {
            // Check if clicking on title bar area (top 30 pixels)
            if mouse_y >= self.y && mouse_y <= self.y + 30.0 &&
               mouse_x >= self.x && mouse_x <= self.x + self.width {
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

        // Position buttons at bottom of window
        let btn_y = self.y + self.height - 40.0;
        let btn_prev_x = self.x + 20.0;
        let btn_next_x = self.x + 120.0;
        let btn_ok_x = self.x + self.width - 100.0;
        
        // Update button positions and states
        self.btn_prev.x = btn_prev_x;
        self.btn_prev.y = btn_y;
        self.btn_prev.update(self.x, self.y);
        
        self.btn_next.x = btn_next_x;
        self.btn_next.y = btn_y;
        self.btn_next.update(self.x, self.y);
        
        self.btn_ok.x = btn_ok_x;
        self.btn_ok.y = btn_y;
        self.btn_ok.update(self.x, self.y);

        // Now handle dialog-specific updates
        // Check button clicks first (before borrowing dialog)
        let prev_clicked = self.btn_prev.is_clicked(self.x, self.y);
        let next_clicked = self.btn_next.is_clicked(self.x, self.y);
        let ok_clicked = self.btn_ok.is_clicked(self.x, self.y);
        let enter_pressed = is_key_pressed(KeyCode::Enter);
        let escape_pressed = is_key_pressed(KeyCode::Escape);
        let left_pressed = is_key_pressed(KeyCode::Left);
        let up_pressed = is_key_pressed(KeyCode::Up);
        let right_pressed = is_key_pressed(KeyCode::Right);
        let down_pressed = is_key_pressed(KeyCode::Down);
        
        if let Some(ref mut dialog) = self.active_dialog {
            // Handle button clicks
            if prev_clicked {
                if dialog.current_page > 0 {
                    dialog.current_page -= 1;
                }
            }
            if next_clicked {
                if dialog.current_page < dialog.pages.len() - 1 {
                    dialog.current_page += 1;
                }
            }
            if ok_clicked || enter_pressed || escape_pressed {
                self.close_dialog();
                return;
            }

            // Handle arrow key navigation
            if left_pressed || up_pressed {
                if dialog.current_page > 0 {
                    dialog.current_page -= 1;
                }
            }
            if right_pressed || down_pressed {
                if dialog.current_page < dialog.pages.len() - 1 {
                    dialog.current_page += 1;
                }
            }
        }
    }

    /// Draw the NPC dialog window
    pub fn draw(&self, _camera_x: f32, _camera_y: f32) {
        if let Some(dialog) = &self.active_dialog {
            if !dialog.visible || !self.loaded {
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

            // Draw dialog text
            if let Some(page_text) = dialog.pages.get(dialog.current_page) {
                let font_size = 12.0;
                let padding = 20.0;
                let text_x = self.x + padding;
                let mut text_y = self.y + 50.0;
                let max_width = self.width - padding * 2.0;

                // Word wrap text
                let lines = self.wrap_text(page_text, max_width, font_size);
                let line_height = 18.0;

                for line in &lines {
                    if let Some(font) = &self.font {
                        draw_text_ex(line, text_x, text_y, TextParams {
                            font: Some(font),
                            font_size: font_size as u16,
                            color: Color::from_rgba(0, 0, 0, 255),
                            ..Default::default()
                        });
                    } else {
                        draw_text(line, text_x, text_y, font_size, Color::from_rgba(0, 0, 0, 255));
                    }
                    text_y += line_height;
                }

                // Draw page indicator
                if dialog.pages.len() > 1 {
                    let page_text = format!("Page {} of {}", dialog.current_page + 1, dialog.pages.len());
                    let page_y = self.y + self.height - 60.0;
                    draw_text(&page_text, text_x, page_y, 10.0, Color::from_rgba(100, 100, 100, 255));
                }
            }

            // Draw navigation buttons
            // Buttons are already positioned in update(), just draw them
            // Only show prev/next if multiple pages
            if dialog.pages.len() > 1 {
                self.btn_prev.draw(self.x, self.y);
                self.btn_next.draw(self.x, self.y);
            }
            self.btn_ok.draw(self.x, self.y);
        }
    }

    fn wrap_text(&self, text: &str, max_width: f32, font_size: f32) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in text.split_whitespace() {
            let test_line = if current_line.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current_line, word)
            };

            let width = measure_text(&test_line, self.font.as_ref(), font_size as u16, 1.0).width;

            if width > max_width && !current_line.is_empty() {
                lines.push(current_line);
                current_line = word.to_string();
            } else {
                current_line = test_line;
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        if lines.is_empty() {
            lines.push(text.to_string());
        }

        lines
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }
}

impl Default for NpcDialogSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
struct NpcDialogData {
    backgrnd: Option<TextureWithOrigin>,
    backgrnd2: Option<TextureWithOrigin>,
    backgrnd3: Option<TextureWithOrigin>,
    btn_prev: DialogButton,
    btn_next: DialogButton,
    btn_ok: DialogButton,
}
