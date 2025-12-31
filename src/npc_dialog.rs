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
    // UtilDlgEx pieces - t=top, c=corner, line=text line bg, is=bottom, it=inner top, ic=inner corner
    tex_t: Option<Texture2D>,
    tex_c: Option<Texture2D>,
    tex_line: Option<Texture2D>,
    tex_is: Option<Texture2D>,
    tex_it: Option<Texture2D>,
    tex_ic: Option<Texture2D>,
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
}

impl NpcDialogSystem {
    pub fn new() -> Self {
        Self {
            loaded: false,
            tex_t: None,
            tex_c: None,
            tex_line: None,
            tex_is: None,
            tex_it: None,
            tex_ic: None,
            btn_ok: DialogButton::default(),
            btn_prev: DialogButton::default(),
            btn_next: DialogButton::default(),
            active_dialog: None,
            font: None,
            x: 100.0,
            y: 100.0,
            btn_hovered: false,
            btn_pressed: false,
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
                    self.tex_line = Self::load_tex(&root_node, "UtilDlgEx/line").await;
                    self.tex_is = Self::load_tex(&root_node, "UtilDlgEx/is").await;
                    self.tex_it = Self::load_tex(&root_node, "UtilDlgEx/it").await;
                    self.tex_ic = Self::load_tex(&root_node, "UtilDlgEx/ic").await;

                    self.btn_ok = Self::load_button(&root_node, "UtilDlgEx/BtOK").await;
                    self.btn_prev = Self::load_button(&root_node, "UtilDlgEx/BtPrev").await;
                    self.btn_next = Self::load_button(&root_node, "UtilDlgEx/BtNext").await;

                    self.loaded = true;
                    info!("NPC dialog loaded: t={}, c={}, line={}, is={}, it={}, ic={}", 
                        self.tex_t.is_some(), self.tex_c.is_some(), 
                        self.tex_line.is_some(), self.tex_is.is_some(),
                        self.tex_it.is_some(), self.tex_ic.is_some());
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
        let t_h = self.tex_t.as_ref().map(|t| t.height()).unwrap_or(12.0);
        let line_h = self.tex_line.as_ref().map(|t| t.height()).unwrap_or(14.0);
        let it_h = self.tex_it.as_ref().map(|t| t.height()).unwrap_or(0.0);
        
        // Calculate dialog height based on text lines
        let lines = if let Some(ref dialog) = self.active_dialog {
            self.wrap_text(&dialog.text, width - 80.0, 12.0)
        } else {
            vec![]
        };
        let num_lines = lines.len().max(3);
        let dialog_height = t_h + it_h + (num_lines as f32 * line_h);

        // Check OK button hover/click
        let (mx, my) = mouse_position();
        let btn_x = self.x + width - 60.0;
        let btn_y = self.y + dialog_height - 5.0;
        self.btn_hovered = mx >= btn_x && mx <= btn_x + self.btn_ok.width 
            && my >= btn_y && my <= btn_y + self.btn_ok.height;
        self.btn_pressed = self.btn_hovered && is_mouse_button_down(MouseButton::Left);

        if (self.btn_hovered && is_mouse_button_released(MouseButton::Left)) || 
           is_key_pressed(KeyCode::Enter) {
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
        let line_h = self.tex_line.as_ref().map(|t| t.height()).unwrap_or(14.0);
        let is_h = self.tex_is.as_ref().map(|t| t.height()).unwrap_or(12.0);
        let c_w = self.tex_c.as_ref().map(|t| t.width()).unwrap_or(6.0);

        let lines = self.wrap_text(&dialog.text, width - 80.0, 12.0);
        let num_lines = lines.len().max(3);

        // === Draw TOP row (outer border) ===
        // Left corner
        if let Some(c) = &self.tex_c {
            draw_texture(c, self.x, self.y, WHITE);
        }
        // Top edge (tiled)
        if let Some(t) = &self.tex_t {
            let mut x = self.x + c_w;
            while x < self.x + width - c_w {
                let w = (self.x + width - c_w - x).min(t.width());
                draw_texture_ex(t, x, self.y, WHITE, DrawTextureParams {
                    source: Some(Rect::new(0.0, 0.0, w, t.height())),
                    ..Default::default()
                });
                x += t.width();
            }
        }
        // Right corner (flipped)
        if let Some(c) = &self.tex_c {
            draw_texture_ex(c, self.x + width - c_w, self.y, WHITE, DrawTextureParams {
                flip_x: true, ..Default::default()
            });
        }

        // === Draw INNER TOP row (below outer top) ===
        let mut y = self.y + t_h;
        let it_h = self.tex_it.as_ref().map(|t| t.height()).unwrap_or(0.0);
        if it_h > 0.0 {
            // Left inner corner
            if let Some(ic) = &self.tex_ic {
                draw_texture(ic, self.x, y, WHITE);
            } else if let Some(c) = &self.tex_c {
                draw_texture(c, self.x, y, WHITE);
            }
            // Inner top (tiled)
            if let Some(it) = &self.tex_it {
                let mut x = self.x + c_w;
                while x < self.x + width - c_w {
                    let w = (self.x + width - c_w - x).min(it.width());
                    draw_texture_ex(it, x, y, WHITE, DrawTextureParams {
                        source: Some(Rect::new(0.0, 0.0, w, it.height())),
                        ..Default::default()
                    });
                    x += it.width();
                }
            }
            // Right inner corner (flipped)
            if let Some(ic) = &self.tex_ic {
                draw_texture_ex(ic, self.x + width - c_w, y, WHITE, DrawTextureParams {
                    flip_x: true, ..Default::default()
                });
            } else if let Some(c) = &self.tex_c {
                draw_texture_ex(c, self.x + width - c_w, y, WHITE, DrawTextureParams {
                    flip_x: true, ..Default::default()
                });
            }
            y += it_h;
        }

        // === Draw MIDDLE rows (one per line of text) ===
        for _ in 0..num_lines {
            // Left edge
            if let Some(c) = &self.tex_c {
                draw_texture(c, self.x, y, WHITE);
            }
            // Line background (white, tiled)
            if let Some(line) = &self.tex_line {
                let mut x = self.x + c_w;
                while x < self.x + width - c_w {
                    let w = (self.x + width - c_w - x).min(line.width());
                    draw_texture_ex(line, x, y, WHITE, DrawTextureParams {
                        source: Some(Rect::new(0.0, 0.0, w, line.height())),
                        ..Default::default()
                    });
                    x += line.width();
                }
            }
            // Right edge (flipped)
            if let Some(c) = &self.tex_c {
                draw_texture_ex(c, self.x + width - c_w, y, WHITE, DrawTextureParams {
                    flip_x: true, ..Default::default()
                });
            }
            y += line_h;
        }

        // === Draw BOTTOM row ===
        // Left corner (flipped vertically)
        if let Some(c) = &self.tex_c {
            draw_texture_ex(c, self.x, y, WHITE, DrawTextureParams {
                flip_y: true, ..Default::default()
            });
        }
        // Bottom edge (tiled)
        if let Some(is) = &self.tex_is {
            let mut x = self.x + c_w;
            while x < self.x + width - c_w {
                let w = (self.x + width - c_w - x).min(is.width());
                draw_texture_ex(is, x, y, WHITE, DrawTextureParams {
                    source: Some(Rect::new(0.0, 0.0, w, is.height())),
                    ..Default::default()
                });
                x += is.width();
            }
        }
        // Right corner (flipped both)
        if let Some(c) = &self.tex_c {
            draw_texture_ex(c, self.x + width - c_w, y, WHITE, DrawTextureParams {
                flip_x: true, flip_y: true, ..Default::default()
            });
        }

        // Draw NPC image if available
        let text_start_x = if let Some(npc_tex) = &dialog.npc_texture {
            draw_texture(npc_tex, self.x + 10.0, self.y + t_h + it_h + 5.0, WHITE);
            self.x + 70.0
        } else {
            self.x + 15.0
        };

        // Draw text
        let mut text_y = self.y + t_h + it_h + 14.0;
        for line in &lines {
            if let Some(font) = &self.font {
                draw_text_ex(line, text_start_x, text_y, TextParams {
                    font: Some(font), font_size: 12, color: BLACK, ..Default::default()
                });
            } else {
                draw_text(line, text_start_x, text_y, 12.0, BLACK);
            }
            text_y += line_h;
        }

        // Draw OK button at bottom right
        let btn_y = y - 5.0;
        self.btn_ok.draw(self.x + width - 60.0, btn_y, self.btn_hovered, self.btn_pressed);
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
