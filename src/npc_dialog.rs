use macroquad::prelude::*;
use crate::assets::AssetManager;
use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzReader, WzNodeCast};

const UIWINDOW2_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/UI/UIWindow2.img";
const UIWINDOW2_CACHE: &str = "/01/UI/UIWindow2.img";

/// Button with states
#[derive(Clone, Default)]
struct DialogButton {
    normal: Option<Texture2D>,
    mouse_over: Option<Texture2D>,
    pressed: Option<Texture2D>,
    width: f32,
    height: f32,
}

impl DialogButton {
    fn draw(&self, x: f32, y: f32, hovered: bool, pressed: bool) {
        let tex = if pressed && self.pressed.is_some() {
            &self.pressed
        } else if hovered && self.mouse_over.is_some() {
            &self.mouse_over
        } else {
            &self.normal
        };
        if let Some(t) = tex {
            draw_texture(t, x, y, WHITE);
        }
    }
}

/// Active NPC dialog
pub struct ActiveNpcDialog {
    pub text: String,
    pub npc_texture: Option<Texture2D>,
    pub visible: bool,
}

/// NPC Dialog system using UtilDlgEx pieces
pub struct NpcDialogSystem {
    loaded: bool,
    // UtilDlgEx pieces - t=top, c=middle, s=bottom
    tex_t: Option<Texture2D>,
    tex_c: Option<Texture2D>,
    tex_s: Option<Texture2D>,
    // Buttons
    btn_ok: DialogButton,
    btn_prev: DialogButton,
    btn_next: DialogButton,
    // Dialog state
    active_dialog: Option<ActiveNpcDialog>,
    font: Option<Font>,
    x: f32,
    y: f32,
    btn_hovered: bool,
    btn_pressed: bool,
    // Dragging state
    is_dragging: bool,
    drag_offset_x: f32,
    drag_offset_y: f32,
}

impl NpcDialogSystem {
    pub fn new() -> Self {
        Self {
            loaded: false,
            tex_t: None,
            tex_c: None,
            tex_s: None,
            btn_ok: DialogButton::default(),
            btn_prev: DialogButton::default(),
            btn_next: DialogButton::default(),
            active_dialog: None,
            font: None,
            x: 100.0,
            y: 100.0,
            btn_hovered: false,
            btn_pressed: false,
            is_dragging: false,
            drag_offset_x: 0.0,
            drag_offset_y: 0.0,
        }
    }

    pub async fn load_assets(&mut self) {
        info!("Loading NPC dialog assets...");
        
        if let Ok(bytes) = AssetManager::fetch_and_cache(UIWINDOW2_URL, UIWINDOW2_CACHE).await {
            if let Some(wz_iv) = guess_iv_from_wz_img(&bytes) {
                let byte_len = bytes.len();
                let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
                let cache_name_ref: wz_reader::WzNodeName = UIWINDOW2_CACHE.to_string().into();
                let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
                let root_node: WzNodeArc = WzNode::new(&UIWINDOW2_CACHE.to_string().into(), wz_image, None).into();

                if root_node.write().unwrap().parse(&root_node).is_ok() {
                    self.tex_t = Self::load_tex(&root_node, "UtilDlgEx/t").await;
                    self.tex_c = Self::load_tex(&root_node, "UtilDlgEx/c").await;
                    self.tex_s = Self::load_tex(&root_node, "UtilDlgEx/s").await;

                    self.btn_ok = Self::load_button(&root_node, "UtilDlgEx/BtOK").await;
                    self.btn_prev = Self::load_button(&root_node, "UtilDlgEx/BtPrev").await;
                    self.btn_next = Self::load_button(&root_node, "UtilDlgEx/BtNext").await;

                    self.loaded = true;
                    info!("NPC dialog loaded: t={}, c={}, s={}", 
                        self.tex_t.is_some(), self.tex_c.is_some(), self.tex_s.is_some());
                }
            }
        }

        if let Ok(font) = load_ttf_font("https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/MaplestoryLight.ttf").await {
            self.font = Some(font);
        }
    }

    async fn load_tex(root: &WzNodeArc, path: &str) -> Option<Texture2D> {
        let node = root.read().unwrap().at_path(path)?.clone();
        node.write().unwrap().parse(&node).ok()?;
        let node_read = node.read().unwrap();
        let png = node_read.try_as_png()?;
        let img = png.extract_png().ok()?;
        let rgba = img.to_rgba8();
        Some(Texture2D::from_rgba8(rgba.width() as u16, rgba.height() as u16, &rgba.into_raw()))
    }

    async fn load_button(root: &WzNodeArc, base_path: &str) -> DialogButton {
        let mut btn = DialogButton::default();
        if let Some(tex) = Self::load_tex(root, &format!("{}/normal/0", base_path)).await {
            btn.width = tex.width();
            btn.height = tex.height();
            btn.normal = Some(tex);
        }
        btn.mouse_over = Self::load_tex(root, &format!("{}/mouseOver/0", base_path)).await;
        btn.pressed = Self::load_tex(root, &format!("{}/pressed/0", base_path)).await;
        btn
    }

    pub fn show_dialog(&mut self, text: &str, _npc_x: f32, _npc_y: f32) {
        self.active_dialog = Some(ActiveNpcDialog {
            text: text.to_string(),
            npc_texture: None,
            visible: true,
        });
        self.x = (screen_width() - 350.0) / 2.0;
        self.y = (screen_height() - 150.0) / 2.0;
    }

    pub fn show_dialog_with_npc(&mut self, text: &str, _npc_x: f32, _npc_y: f32, npc_texture: Option<Texture2D>) {
        self.active_dialog = Some(ActiveNpcDialog {
            text: text.to_string(),
            npc_texture,
            visible: true,
        });
        self.x = (screen_width() - 350.0) / 2.0;
        self.y = (screen_height() - 150.0) / 2.0;
    }

    pub fn close_dialog(&mut self) {
        self.active_dialog = None;
    }

    pub fn is_visible(&self) -> bool {
        self.active_dialog.as_ref().map(|d| d.visible).unwrap_or(false)
    }

    pub fn update(&mut self) {
        if !self.is_visible() { return; }

        let width = 350.0;
        let fill_count = 6;
        let t_h = self.tex_t.as_ref().map(|t| t.height()).unwrap_or(10.0);
        let c_h = self.tex_c.as_ref().map(|t| t.height()).unwrap_or(14.0);
        let s_h = self.tex_s.as_ref().map(|t| t.height()).unwrap_or(10.0);
        
        // Total height of dialog window
        let total_height = t_h + (fill_count as f32 * c_h) + s_h;
        
        // Button position - inside the window, in the bottom section
        // From C++: y_cord = height_ + 48, but we want it inside the bottom texture
        let btn_x = self.x + width - 60.0; // Right side
        let btn_y = self.y + t_h + (fill_count as f32 * c_h) + (s_h / 2.0) - 10.0; // Center of bottom section
        let btn_w = self.btn_ok.width.max(40.0);
        let btn_h = self.btn_ok.height.max(20.0);

        let (mx, my) = mouse_position();
        
        // Check button hover/click
        self.btn_hovered = mx >= btn_x && mx <= btn_x + btn_w 
            && my >= btn_y && my <= btn_y + btn_h;
        self.btn_pressed = self.btn_hovered && is_mouse_button_down(MouseButton::Left);

        // Close on button click, Enter, or Escape
        if (self.btn_hovered && is_mouse_button_released(MouseButton::Left)) || 
           is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Escape) {
            self.close_dialog();
            return;
        }
        
        // Handle dragging only on top area (not on button)
        let is_in_drag_area = mx >= self.x && mx <= self.x + width 
            && my >= self.y && my <= self.y + t_h;
        
        if is_mouse_button_pressed(MouseButton::Left) && is_in_drag_area {
            self.is_dragging = true;
            self.drag_offset_x = mx - self.x;
            self.drag_offset_y = my - self.y;
        }
        
        if self.is_dragging {
            if is_mouse_button_down(MouseButton::Left) {
                self.x = mx - self.drag_offset_x;
                self.y = my - self.drag_offset_y;
            } else {
                self.is_dragging = false;
            }
        }
    }

    #[inline(never)]
    pub fn draw(&self, _camera_x: f32, _camera_y: f32) {
        let dialog = match &self.active_dialog {
            Some(d) if d.visible => d,
            _ => return,
        };

        let width = 350.0;
        let fill_count = 6;  // Base fill count
        let left_padding = 20.0;
        
        let t_h = self.tex_t.as_ref().map(|t| t.height()).unwrap_or(10.0);
        let t_w = self.tex_t.as_ref().map(|t| t.width()).unwrap_or(width);
        let c_h = self.tex_c.as_ref().map(|t| t.height()).unwrap_or(14.0);
        let c_w = self.tex_c.as_ref().map(|t| t.width()).unwrap_or(width);
        let s_h = self.tex_s.as_ref().map(|t| t.height()).unwrap_or(10.0);
        let s_w = self.tex_s.as_ref().map(|t| t.width()).unwrap_or(width);
        
        let has_textures = self.tex_t.is_some() && self.tex_c.is_some() && self.tex_s.is_some();
        
        if has_textures {
            // Draw TOP
            if let Some(t) = &self.tex_t {
                draw_texture(t, self.x, self.y, WHITE);
            }
            
            // Draw FILL (middle) - repeat fill_count times
            let mut y = self.y + t_h;
            for _ in 0..fill_count {
                if let Some(c) = &self.tex_c {
                    draw_texture(c, self.x, y, WHITE);
                }
                y += c_h;
            }
            
            // Draw BOTTOM
            if let Some(s) = &self.tex_s {
                draw_texture(s, self.x, y, WHITE);
            }
            
            let total_height = t_h + (fill_count as f32 * c_h) + s_h;
            
            // Draw NPC speaker on left side, centered vertically in middle section
            if let Some(npc_tex) = &dialog.npc_texture {
                let npc_width = npc_tex.width();
                let npc_height = npc_tex.height();
                let middle_height = fill_count as f32 * c_h;
                // Center NPC vertically in the middle section
                let npc_x = self.x + left_padding + (22.0 - left_padding); // 22 from C++ example
                let npc_y = self.y + t_h + (middle_height - npc_height) / 2.0;
                draw_texture(npc_tex, npc_x, npc_y, WHITE);
            }
            
            // Draw text at x + 166, y + 48 (from TypeScript/C++ examples)
            let text_x = self.x + 166.0;
            let text_y = self.y + 48.0;
            let lines = self.wrap_text(&dialog.text, 180.0, 12.0);
            let mut ty = text_y;
            for line in &lines {
                if let Some(font) = &self.font {
                    draw_text_ex(line, text_x, ty, TextParams {
                        font: Some(font), font_size: 12, color: BLACK, ..Default::default()
                    });
                } else {
                    draw_text(line, text_x, ty, 12.0, BLACK);
                }
                ty += 14.0;
            }
            
            // Draw OK button inside the window, in the bottom section
            let btn_x = self.x + width - 60.0; // Right side
            let btn_y = self.y + t_h + (fill_count as f32 * c_h) + (s_h / 2.0) - 10.0; // Center of bottom section
            self.btn_ok.draw(btn_x, btn_y, self.btn_hovered, self.btn_pressed);
        } else {
            // Fallback rendering
            let height = 150.0;
            draw_rectangle(self.x, self.y, width, height, Color::from_rgba(245, 235, 210, 255));
            draw_rectangle_lines(self.x, self.y, width, height, 2.0, Color::from_rgba(139, 90, 43, 255));
            
            // NPC avatar
            let text_x = if let Some(npc_tex) = &dialog.npc_texture {
                draw_texture(npc_tex, self.x + 20.0, self.y + 20.0, WHITE);
                self.x + 100.0
            } else {
                self.x + 20.0
            };
            
            // Text
            let lines = self.wrap_text(&dialog.text, 200.0, 12.0);
            let mut ty = self.y + 30.0;
            for line in &lines {
                draw_text(line, text_x, ty, 12.0, BLACK);
                ty += 16.0;
            }
            
            // OK button
            let btn_x = self.x + 9.0;
            let btn_y = self.y + height - 30.0;
            let btn_color = if self.btn_pressed {
                Color::from_rgba(180, 140, 80, 255)
            } else if self.btn_hovered {
                Color::from_rgba(220, 180, 120, 255)
            } else {
                Color::from_rgba(200, 160, 100, 255)
            };
            draw_rectangle(btn_x, btn_y, 50.0, 20.0, btn_color);
            draw_rectangle_lines(btn_x, btn_y, 50.0, 20.0, 1.0, Color::from_rgba(139, 90, 43, 255));
            draw_text("OK", btn_x + 17.0, btn_y + 14.0, 14.0, BLACK);
        }
    }

    fn wrap_text(&self, text: &str, max_width: f32, font_size: f32) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current = String::new();
        for word in text.split_whitespace() {
            let test = if current.is_empty() { word.to_string() } else { format!("{} {}", current, word) };
            let w = measure_text(&test, self.font.as_ref(), font_size as u16, 1.0).width;
            if w > max_width && !current.is_empty() {
                lines.push(current);
                current = word.to_string();
            } else {
                current = test;
            }
        }
        if !current.is_empty() { lines.push(current); }
        if lines.is_empty() { lines.push(text.to_string()); }
        lines
    }

    pub fn is_loaded(&self) -> bool { self.loaded }
}

impl Default for NpcDialogSystem {
    fn default() -> Self { Self::new() }
}
