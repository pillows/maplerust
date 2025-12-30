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
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    hovered: bool,
    down: bool,
}

impl DialogButton {
    fn update(&mut self, base_x: f32, base_y: f32) {
        let (mx, my) = mouse_position();
        let bx = base_x + self.x;
        let by = base_y + self.y;
        self.hovered = mx >= bx && mx <= bx + self.width && my >= by && my <= by + self.height;
        self.down = self.hovered && is_mouse_button_down(MouseButton::Left);
    }

    fn is_clicked(&self, base_x: f32, base_y: f32) -> bool {
        let (mx, my) = mouse_position();
        let bx = base_x + self.x;
        let by = base_y + self.y;
        mx >= bx && mx <= bx + self.width && my >= by && my <= by + self.height 
            && is_mouse_button_released(MouseButton::Left)
    }

    fn draw(&self, base_x: f32, base_y: f32) {
        let tex = if self.down && self.pressed.is_some() {
            &self.pressed
        } else if self.hovered && self.mouse_over.is_some() {
            &self.mouse_over
        } else {
            &self.normal
        };
        if let Some(t) = tex {
            draw_texture(t, base_x + self.x, base_y + self.y, WHITE);
        }
    }
}

/// Active NPC dialog
pub struct ActiveNpcDialog {
    pub text: String,
    pub npc_x: f32,
    pub npc_y: f32,
    pub npc_texture: Option<Texture2D>,
    pub visible: bool,
}

/// NPC Dialog system using UtilDlgEx pieces
pub struct NpcDialogSystem {
    loaded: bool,
    // UtilDlgEx pieces
    tex_t: Option<Texture2D>,    // Top edge
    tex_c: Option<Texture2D>,    // Left/right edge (corner)
    tex_line: Option<Texture2D>, // Line background for text
    tex_is: Option<Texture2D>,   // Bottom edge
    // Buttons
    btn_ok: DialogButton,
    btn_prev: DialogButton,
    btn_next: DialogButton,
    // Dialog state
    active_dialog: Option<ActiveNpcDialog>,
    font: Option<Font>,
    // Position
    x: f32,
    y: f32,
}

impl NpcDialogSystem {
    pub fn new() -> Self {
        Self {
            loaded: false,
            tex_t: None,
            tex_c: None,
            tex_line: None,
            tex_is: None,
            btn_ok: DialogButton::default(),
            btn_prev: DialogButton::default(),
            btn_next: DialogButton::default(),
            active_dialog: None,
            font: None,
            x: 100.0,
            y: 100.0,
        }
    }

    pub async fn load_assets(&mut self) {
        info!("Loading NPC dialog assets from UtilDlgEx...");
        
        if let Ok(bytes) = AssetManager::fetch_and_cache(UIWINDOW2_URL, UIWINDOW2_CACHE).await {
            if let Some(wz_iv) = guess_iv_from_wz_img(&bytes) {
                let byte_len = bytes.len();
                let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
                let cache_name_ref: wz_reader::WzNodeName = UIWINDOW2_CACHE.to_string().into();
                let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
                let root_node: WzNodeArc = WzNode::new(&UIWINDOW2_CACHE.to_string().into(), wz_image, None).into();

                if root_node.write().unwrap().parse(&root_node).is_ok() {
                    // Load UtilDlgEx pieces
                    self.tex_t = Self::load_tex(&root_node, "UtilDlgEx/t").await;
                    self.tex_c = Self::load_tex(&root_node, "UtilDlgEx/c").await;
                    self.tex_line = Self::load_tex(&root_node, "UtilDlgEx/line").await;
                    self.tex_is = Self::load_tex(&root_node, "UtilDlgEx/is").await;

                    // Load buttons
                    self.btn_ok = Self::load_button(&root_node, "UtilDlgEx/BtOK").await;
                    self.btn_prev = Self::load_button(&root_node, "UtilDlgEx/BtPrev").await;
                    self.btn_next = Self::load_button(&root_node, "UtilDlgEx/BtNext").await;

                    self.loaded = true;
                    info!("NPC dialog assets loaded");
                }
            }
        }

        // Load font
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

    pub fn show_dialog(&mut self, text: &str, npc_x: f32, npc_y: f32) {
        self.active_dialog = Some(ActiveNpcDialog {
            text: text.to_string(),
            npc_x,
            npc_y,
            npc_texture: None,
            visible: true,
        });
        // Center dialog on screen
        self.x = (screen_width() - 350.0) / 2.0;
        self.y = (screen_height() - 200.0) / 2.0;
    }

    pub fn show_dialog_with_npc(&mut self, text: &str, npc_x: f32, npc_y: f32, npc_texture: Option<Texture2D>) {
        self.active_dialog = Some(ActiveNpcDialog {
            text: text.to_string(),
            npc_x,
            npc_y,
            npc_texture,
            visible: true,
        });
        self.x = (screen_width() - 350.0) / 2.0;
        self.y = (screen_height() - 200.0) / 2.0;
    }

    pub fn close_dialog(&mut self) {
        self.active_dialog = None;
    }

    pub fn is_visible(&self) -> bool {
        self.active_dialog.as_ref().map(|d| d.visible).unwrap_or(false)
    }

    pub fn update(&mut self) {
        if !self.is_visible() { return; }

        // Update buttons
        self.btn_ok.update(self.x, self.y);
        self.btn_prev.update(self.x, self.y);
        self.btn_next.update(self.x, self.y);

        // Handle close
        if self.btn_ok.is_clicked(self.x, self.y) || 
           is_key_pressed(KeyCode::Enter) || 
           is_key_pressed(KeyCode::Escape) {
            self.close_dialog();
        }
    }

    pub fn draw(&self, _camera_x: f32, _camera_y: f32) {
        let dialog = match &self.active_dialog {
            Some(d) if d.visible => d,
            _ => return,
        };

        let width = 350.0;
        let t_h = self.tex_t.as_ref().map(|t| t.height()).unwrap_or(12.0);
        let line_h = self.tex_line.as_ref().map(|t| t.height()).unwrap_or(16.0);
        let is_h = self.tex_is.as_ref().map(|t| t.height()).unwrap_or(12.0);
        let c_w = self.tex_c.as_ref().map(|t| t.width()).unwrap_or(8.0);

        // Word wrap text
        let lines = self.wrap_text(&dialog.text, width - 80.0, 12.0);
        let num_lines = lines.len().max(3);

        // Draw top edge
        if let Some(t) = &self.tex_t {
            let mut x = self.x + c_w;
            while x < self.x + width - c_w {
                draw_texture(t, x, self.y, WHITE);
                x += t.width();
            }
        }

        // Draw left/right edges and fill
        let mut y = self.y + t_h;
        for _ in 0..num_lines {
            // Left edge
            if let Some(c) = &self.tex_c {
                draw_texture(c, self.x, y, WHITE);
            }
            // Line background (white fill for text)
            if let Some(line) = &self.tex_line {
                let mut x = self.x + c_w;
                while x < self.x + width - c_w {
                    draw_texture(line, x, y, WHITE);
                    x += line.width();
                }
            }
            // Right edge (flipped)
            if let Some(c) = &self.tex_c {
                draw_texture_ex(c, self.x + width - c_w, y, WHITE, DrawTextureParams {
                    flip_x: true,
                    ..Default::default()
                });
            }
            y += line_h;
        }

        // Draw bottom edge
        if let Some(is) = &self.tex_is {
            let mut x = self.x + c_w;
            while x < self.x + width - c_w {
                draw_texture(is, x, y, WHITE);
                x += is.width();
            }
        }

        // Draw NPC image if available (left side)
        let text_start_x = if let Some(npc_tex) = &dialog.npc_texture {
            let npc_draw_x = self.x + 10.0;
            let npc_draw_y = self.y + t_h + 5.0;
            draw_texture(npc_tex, npc_draw_x, npc_draw_y, WHITE);
            self.x + 70.0 // Text starts after NPC image
        } else {
            self.x + 15.0
        };

        // Draw text
        let font_size = 12.0;
        let mut text_y = self.y + t_h + 14.0;
        for line in &lines {
            if let Some(font) = &self.font {
                draw_text_ex(line, text_start_x, text_y, TextParams {
                    font: Some(font),
                    font_size: font_size as u16,
                    color: BLACK,
                    ..Default::default()
                });
            } else {
                draw_text(line, text_start_x, text_y, font_size, BLACK);
            }
            text_y += line_h;
        }

        // Draw OK button at bottom
        let btn_x = self.x + width - self.btn_ok.width - 10.0;
        let btn_y = y + 5.0;
        if let Some(tex) = if self.btn_ok.down && self.btn_ok.pressed.is_some() {
            &self.btn_ok.pressed
        } else if self.btn_ok.hovered && self.btn_ok.mouse_over.is_some() {
            &self.btn_ok.mouse_over
        } else {
            &self.btn_ok.normal
        } {
            draw_texture(tex, btn_x, btn_y, WHITE);
        }
    }

    fn wrap_text(&self, text: &str, max_width: f32, font_size: f32) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current = String::new();

        for word in text.split_whitespace() {
            let test = if current.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current, word)
            };
            let w = measure_text(&test, self.font.as_ref(), font_size as u16, 1.0).width;
            if w > max_width && !current.is_empty() {
                lines.push(current);
                current = word.to_string();
            } else {
                current = test;
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
        if lines.is_empty() {
            lines.push(text.to_string());
        }
        lines
    }

    pub fn is_loaded(&self) -> bool { self.loaded }
}

impl Default for NpcDialogSystem {
    fn default() -> Self { Self::new() }
}
