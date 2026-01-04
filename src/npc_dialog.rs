use macroquad::prelude::*;
use crate::assets::AssetManager;
use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzReader, WzNodeCast};

const UIWINDOW2_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/UI/UIWindow2.img";
const UIWINDOW2_CACHE: &str = "/01/UI/UIWindow2.img";

/// Dialog type determines UI layout and available buttons
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DialogType {
    Ok,              // OK button only
    Next,            // Next button only
    YesNo,           // Yes + No buttons
    AcceptDecline,   // Accept + Decline buttons (same as YesNo visually)
    Selection,       // List of selectable options + OK button
    Style,           // Style preview + OK/Cancel buttons
}

/// User's response to dialog
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DialogResponse {
    None,
    Ok,
    Yes,
    No,
    Next,
    Accept,
    Decline,
    Selection(usize),  // Index of selected option
    Style(i32),        // Selected style ID
}

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
    tex_bar: Option<Texture2D>,  // Name tag bar
    // Buttons
    btn_ok: DialogButton,
    btn_prev: DialogButton,
    btn_next: DialogButton,
    btn_close: DialogButton,
    btn_yes: DialogButton,
    btn_no: DialogButton,
    // Dialog state
    active_dialog: Option<ActiveNpcDialog>,
    dialog_type: DialogType,
    npc_name: String,
    last_response: DialogResponse,
    font: Option<Font>,
    x: f32,
    y: f32,
    vtile: usize,  // Vertical tile count (default 8)
    // Selection dialog
    selection_options: Vec<String>,
    selected_index: usize,
    selection_scroll_offset: usize,
    // Button hover tracking
    btn_hovered: i32,  // -1 = none, 0+ = button index
    btn_pressed: i32,
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
            tex_bar: None,
            btn_ok: DialogButton::default(),
            btn_prev: DialogButton::default(),
            btn_next: DialogButton::default(),
            btn_close: DialogButton::default(),
            btn_yes: DialogButton::default(),
            btn_no: DialogButton::default(),
            active_dialog: None,
            dialog_type: DialogType::Ok,
            npc_name: String::new(),
            last_response: DialogResponse::None,
            font: None,
            x: 100.0,
            y: 100.0,
            vtile: 8,  // Default from C++ reference
            selection_options: Vec::new(),
            selected_index: 0,
            selection_scroll_offset: 0,
            btn_hovered: -1,
            btn_pressed: -1,
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
                    self.tex_bar = Self::load_tex(&root_node, "UtilDlgEx/bar").await;

                    self.btn_ok = Self::load_button(&root_node, "UtilDlgEx/BtOK").await;
                    self.btn_prev = Self::load_button(&root_node, "UtilDlgEx/BtPrev").await;
                    self.btn_next = Self::load_button(&root_node, "UtilDlgEx/BtNext").await;
                    self.btn_close = Self::load_button(&root_node, "UtilDlgEx/BtClose").await;
                    self.btn_yes = Self::load_button(&root_node, "UtilDlgEx/BtYes").await;
                    self.btn_no = Self::load_button(&root_node, "UtilDlgEx/BtNo").await;

                    self.loaded = true;
                    info!("NPC dialog loaded: t={}, c={}, s={}, bar={}",
                        self.tex_t.is_some(), self.tex_c.is_some(), self.tex_s.is_some(), self.tex_bar.is_some());
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
        self.show_dialog_typed(text, "", None, DialogType::Ok);
    }

    pub fn show_dialog_with_npc(&mut self, text: &str, _npc_x: f32, _npc_y: f32, npc_texture: Option<Texture2D>) {
        self.show_dialog_typed(text, "", npc_texture, DialogType::Ok);
    }

    /// Show dialog with specific type and NPC name
    pub fn show_dialog_typed(&mut self, text: &str, npc_name: &str,
                             npc_tex: Option<Texture2D>, dtype: DialogType) {
        self.active_dialog = Some(ActiveNpcDialog {
            text: text.to_string(),
            npc_texture: npc_tex,
            visible: true
        });
        self.npc_name = npc_name.to_string();
        self.dialog_type = dtype;
        self.last_response = DialogResponse::None;
        self.calculate_vtile(text);
        self.center_on_screen();
    }

    /// Show selection dialog with options
    pub fn show_selection(&mut self, text: &str, npc_name: &str,
                          npc_tex: Option<Texture2D>, options: Vec<String>) {
        self.show_dialog_typed(text, npc_name, npc_tex, DialogType::Selection);
        self.selection_options = options;
        self.selected_index = 0;
        self.selection_scroll_offset = 0;
    }

    /// Get and clear last response
    pub fn take_response(&mut self) -> DialogResponse {
        let resp = self.last_response;
        self.last_response = DialogResponse::None;
        resp
    }

    pub fn close_dialog(&mut self) {
        self.active_dialog = None;
        self.last_response = DialogResponse::None;
    }

    pub fn is_visible(&self) -> bool {
        self.active_dialog.as_ref().map(|d| d.visible).unwrap_or(false)
    }

    /// Auto-calculate vtile based on text length
    fn calculate_vtile(&mut self, text: &str) {
        let lines = self.wrap_text(text, 320.0, 12.0);
        let needed = (lines.len() as f32 * 14.0 / 14.0).ceil() as usize + 2;
        self.vtile = needed.clamp(8, 16);
    }

    /// Center dialog on screen
    fn center_on_screen(&mut self) {
        let w = self.tex_t.as_ref().map(|t| t.width()).unwrap_or(400.0);
        let h_fill = self.tex_c.as_ref().map(|t| t.height()).unwrap_or(14.0);
        let h = self.vtile as f32 * h_fill;
        self.x = (screen_width() - w) / 2.0;
        self.y = (screen_height() - h) / 2.0;
    }

    pub fn update(&mut self) {
        if !self.is_visible() { return; }

        let width = 350.0;
        let t_h = self.tex_t.as_ref().map(|t| t.height()).unwrap_or(10.0);
        let c_h = self.tex_c.as_ref().map(|t| t.height()).unwrap_or(14.0);
        let s_h = self.tex_s.as_ref().map(|t| t.height()).unwrap_or(10.0);

        // Dynamic height based on vtile
        let content_height = self.vtile as f32 * c_h;
        let total_height = t_h + content_height + s_h;

        // Button base position (from C++ reference: height + 48)
        let btn_base_y = self.y + t_h + content_height + 48.0;

        let (mx, my) = mouse_position();

        // Reset hover state
        self.btn_hovered = -1;
        self.btn_pressed = -1;

        // Check button hover based on dialog type
        let check_btn_bounds = |btn: &DialogButton, x: f32, y: f32| -> bool {
            let w = btn.width.max(40.0);
            let h = btn.height.max(20.0);
            mx >= x && mx <= x + w && my >= y && my <= y + h
        };

        match self.dialog_type {
            DialogType::Ok | DialogType::Next => {
                // Single button at x=220
                let btn = if self.dialog_type == DialogType::Ok { &self.btn_ok } else { &self.btn_next };
                if check_btn_bounds(btn, self.x + 220.0, btn_base_y) {
                    self.btn_hovered = 0;
                    if is_mouse_button_down(MouseButton::Left) {
                        self.btn_pressed = 0;
                    }
                }
            }
            DialogType::YesNo | DialogType::AcceptDecline => {
                // Two buttons: left at x=90, right at x=220
                if check_btn_bounds(&self.btn_yes, self.x + 90.0, btn_base_y) {
                    self.btn_hovered = 0; // Yes/Accept button
                    if is_mouse_button_down(MouseButton::Left) {
                        self.btn_pressed = 0;
                    }
                } else if check_btn_bounds(&self.btn_no, self.x + 220.0, btn_base_y) {
                    self.btn_hovered = 1; // No/Decline button
                    if is_mouse_button_down(MouseButton::Left) {
                        self.btn_pressed = 1;
                    }
                }
            }
            DialogType::Selection => {
                // Selection list + OK button
                // TODO: Handle selection list click
                if check_btn_bounds(&self.btn_ok, self.x + 220.0, btn_base_y) {
                    self.btn_hovered = 0;
                    if is_mouse_button_down(MouseButton::Left) {
                        self.btn_pressed = 0;
                    }
                }
            }
            DialogType::Style => {
                // Style preview + OK/Cancel
                // TODO: Handle style arrows
                if check_btn_bounds(&self.btn_ok, self.x + 180.0, btn_base_y) {
                    self.btn_hovered = 0;
                    if is_mouse_button_down(MouseButton::Left) {
                        self.btn_pressed = 0;
                    }
                } else if check_btn_bounds(&self.btn_no, self.x + 100.0, btn_base_y) {
                    self.btn_hovered = 1; // Cancel
                    if is_mouse_button_down(MouseButton::Left) {
                        self.btn_pressed = 1;
                    }
                }
            }
        }

        // Handle button click
        if is_mouse_button_released(MouseButton::Left) && self.btn_hovered >= 0 {
            self.last_response = match (self.dialog_type, self.btn_hovered) {
                (DialogType::Ok, 0) => DialogResponse::Ok,
                (DialogType::Next, 0) => DialogResponse::Next,
                (DialogType::YesNo, 0) => DialogResponse::Yes,
                (DialogType::YesNo, 1) => DialogResponse::No,
                (DialogType::AcceptDecline, 0) => DialogResponse::Accept,
                (DialogType::AcceptDecline, 1) => DialogResponse::Decline,
                (DialogType::Selection, 0) => DialogResponse::Selection(self.selected_index),
                (DialogType::Style, 0) => DialogResponse::Style(0), // TODO: Actual style ID
                (DialogType::Style, 1) => DialogResponse::None, // Cancel
                _ => DialogResponse::None,
            };

            // Don't close for Next dialog - let script handle it
            if self.dialog_type != DialogType::Next {
                self.close_dialog();
            }
            return;
        }

        // Keyboard shortcuts
        if is_key_pressed(KeyCode::Enter) {
            self.last_response = match self.dialog_type {
                DialogType::Ok => DialogResponse::Ok,
                DialogType::Next => DialogResponse::Next,
                DialogType::Selection => DialogResponse::Selection(self.selected_index),
                _ => DialogResponse::None,
            };
            if self.dialog_type != DialogType::Next {
                self.close_dialog();
            }
            return;
        }

        if is_key_pressed(KeyCode::Escape) {
            self.last_response = DialogResponse::None;
            self.close_dialog();
            return;
        }

        // Handle dragging only on top area
        let is_in_drag_area = mx >= self.x && mx <= self.x + width
            && my >= self.y && my <= self.y + t_h;

        if is_mouse_button_pressed(MouseButton::Left) && is_in_drag_area && self.btn_hovered < 0 {
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

        let t_h = self.tex_t.as_ref().map(|t| t.height()).unwrap_or(10.0);
        let c_h = self.tex_c.as_ref().map(|t| t.height()).unwrap_or(14.0);
        let s_h = self.tex_s.as_ref().map(|t| t.height()).unwrap_or(10.0);

        let has_textures = self.tex_t.is_some() && self.tex_c.is_some() && self.tex_s.is_some();

        if has_textures {
            // Draw TOP
            if let Some(t) = &self.tex_t {
                draw_texture(t, self.x, self.y, WHITE);
            }

            // Draw FILL (middle) - repeat vtile times (dynamic)
            let mut y = self.y + t_h;
            for _ in 0..self.vtile {
                if let Some(c) = &self.tex_c {
                    draw_texture(c, self.x, y, WHITE);
                }
                y += c_h;
            }

            // Draw BOTTOM
            if let Some(s) = &self.tex_s {
                draw_texture(s, self.x, y, WHITE);
            }

            let content_height = self.vtile as f32 * c_h;
            let total_height = t_h + content_height + s_h;

            // Draw name tag bar at position (25, 100) relative to dialog (from C++ reference)
            if let Some(bar) = &self.tex_bar {
                draw_texture(bar, self.x + 25.0, self.y + 100.0, WHITE);
            }

            // Draw NPC name centered at (80, 99) in white, size 11 (from C++ reference)
            if !self.npc_name.is_empty() {
                let name_width = if let Some(font) = &self.font {
                    measure_text(&self.npc_name, Some(font), 11, 1.0).width
                } else {
                    measure_text(&self.npc_name, None, 11, 1.0).width
                };

                if let Some(font) = &self.font {
                    draw_text_ex(&self.npc_name, self.x + 80.0 - name_width / 2.0, self.y + 99.0,
                                 TextParams { font: Some(font), font_size: 11, color: WHITE, ..Default::default() });
                } else {
                    draw_text(&self.npc_name, self.x + 80.0 - name_width / 2.0, self.y + 99.0, 11.0, WHITE);
                }
            }

            // Draw NPC speaker at position (80, 100) with flip flag (from C++ reference)
            if let Some(npc_tex) = &dialog.npc_texture {
                let npc_x = self.x + 80.0;
                let npc_y = self.y + 100.0;
                draw_texture(npc_tex, npc_x, npc_y, WHITE);
            }

            // Draw text at x + 156, y + 16 + ((vtile * c_h - text_h) / 2) - vertically centered (from C++ reference)
            let text_x = self.x + 156.0;
            let line_height = 14.0;
            let lines = self.wrap_text(&dialog.text, 320.0, 12.0);
            let total_text_height = lines.len() as f32 * line_height;
            let text_y = self.y + t_h + 16.0 + (content_height - total_text_height) / 2.0;

            let mut ty = text_y;
            for line in &lines {
                if let Some(font) = &self.font {
                    draw_text_ex(line, text_x, ty, TextParams {
                        font: Some(font), font_size: 12, color: Color::from_rgba(80, 80, 80, 255), ..Default::default()
                    });
                } else {
                    draw_text(line, text_x, ty, 12.0, Color::from_rgba(80, 80, 80, 255));
                }
                ty += line_height;
            }

            // Draw buttons based on dialog type (from C++ reference positions)
            let btn_base_y = self.y + t_h + content_height + 48.0;

            match self.dialog_type {
                DialogType::Ok => {
                    // OK button at x=220
                    self.btn_ok.draw(self.x + 220.0, btn_base_y, self.btn_hovered == 0, self.btn_pressed == 0);
                }
                DialogType::Next => {
                    // Next button at x=220
                    self.btn_next.draw(self.x + 220.0, btn_base_y, self.btn_hovered == 0, self.btn_pressed == 0);
                }
                DialogType::YesNo | DialogType::AcceptDecline => {
                    // Yes/Accept button at x=90
                    self.btn_yes.draw(self.x + 90.0, btn_base_y, self.btn_hovered == 0, self.btn_pressed == 0);
                    // No/Decline button at x=220
                    self.btn_no.draw(self.x + 220.0, btn_base_y, self.btn_hovered == 1, self.btn_pressed == 1);
                }
                DialogType::Selection => {
                    // Selection list rendering - TODO: Implement in Phase 3
                    // For now, just show OK button
                    self.btn_ok.draw(self.x + 220.0, btn_base_y, self.btn_hovered == 0, self.btn_pressed == 0);
                }
                DialogType::Style => {
                    // Style preview - TODO: Implement in Phase 4
                    // For now, show OK/Cancel buttons
                    self.btn_ok.draw(self.x + 180.0, btn_base_y, self.btn_hovered == 0, self.btn_pressed == 0);
                    self.btn_no.draw(self.x + 100.0, btn_base_y, self.btn_hovered == 1, self.btn_pressed == 1);
                }
            }

            // Draw Close/END button at x=20 for all dialogs
            self.btn_close.draw(self.x + 20.0, btn_base_y, false, false);

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

            // Buttons based on dialog type
            let btn_y = self.y + height - 30.0;
            let btn_hovered_0 = self.btn_hovered == 0;
            let btn_pressed_0 = self.btn_pressed == 0;

            let btn_color = |hovered: bool, pressed: bool| {
                if pressed {
                    Color::from_rgba(180, 140, 80, 255)
                } else if hovered {
                    Color::from_rgba(220, 180, 120, 255)
                } else {
                    Color::from_rgba(200, 160, 100, 255)
                }
            };

            match self.dialog_type {
                DialogType::Ok | DialogType::Next => {
                    let btn_x = self.x + width - 60.0;
                    let label = if self.dialog_type == DialogType::Ok { "OK" } else { "Next" };
                    draw_rectangle(btn_x, btn_y, 50.0, 20.0, btn_color(btn_hovered_0, btn_pressed_0));
                    draw_rectangle_lines(btn_x, btn_y, 50.0, 20.0, 1.0, Color::from_rgba(139, 90, 43, 255));
                    draw_text(label, btn_x + 12.0, btn_y + 14.0, 14.0, BLACK);
                }
                DialogType::YesNo | DialogType::AcceptDecline => {
                    let yes_x = self.x + 10.0;
                    let no_x = self.x + width - 60.0;
                    // Yes
                    draw_rectangle(yes_x, btn_y, 50.0, 20.0, btn_color(self.btn_hovered == 0, self.btn_pressed == 0));
                    draw_rectangle_lines(yes_x, btn_y, 50.0, 20.0, 1.0, Color::from_rgba(139, 90, 43, 255));
                    draw_text("Yes", yes_x + 14.0, btn_y + 14.0, 14.0, BLACK);
                    // No
                    draw_rectangle(no_x, btn_y, 50.0, 20.0, btn_color(self.btn_hovered == 1, self.btn_pressed == 1));
                    draw_rectangle_lines(no_x, btn_y, 50.0, 20.0, 1.0, Color::from_rgba(139, 90, 43, 255));
                    draw_text("No", no_x + 16.0, btn_y + 14.0, 14.0, BLACK);
                }
                _ => {}
            }
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
