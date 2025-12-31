use macroquad::prelude::*;
use crate::assets::AssetManager;
use std::collections::HashMap;
use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzReader, WzNodeCast};

const CHATBALLOON_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/UI/ChatBalloon.img";
const CHATBALLOON_CACHE: &str = "/01/UI/ChatBalloon.img";

/// Texture with origin point
#[derive(Clone)]
struct TextureWithOrigin {
    texture: Texture2D,
    origin: Vec2,
}

/// Chat balloon frame pieces (9-slice style)
#[derive(Clone)]
struct BalloonFrame {
    // Corners
    nw: Option<TextureWithOrigin>,
    ne: Option<TextureWithOrigin>,
    sw: Option<TextureWithOrigin>,
    se: Option<TextureWithOrigin>,
    // Edges
    n: Option<TextureWithOrigin>,
    s: Option<TextureWithOrigin>,
    e: Option<TextureWithOrigin>,
    w: Option<TextureWithOrigin>,
    // Center
    c: Option<TextureWithOrigin>,
    // Arrow (tail pointing down)
    arrow: Option<TextureWithOrigin>,
}

impl Default for BalloonFrame {
    fn default() -> Self {
        Self {
            nw: None, ne: None, sw: None, se: None,
            n: None, s: None, e: None, w: None,
            c: None, arrow: None,
        }
    }
}

/// Active chat balloon instance
pub struct ActiveBalloon {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub offset_y: f32,  // Offset from anchor point (for following)
    pub follows_player: bool,
    pub lifetime: f32,
    pub max_lifetime: f32,
    pub balloon_type: usize,
}

/// ChatBalloon system for NPC dialogs and tooltips
pub struct ChatBalloonSystem {
    loaded: bool,
    // Different balloon styles (0 = default, 1 = NPC, etc.)
    balloon_frames: HashMap<usize, BalloonFrame>,
    // Active balloons
    active_balloons: Vec<ActiveBalloon>,
    // Font for text
    font: Option<Font>,
}

impl ChatBalloonSystem {
    pub fn new() -> Self {
        Self {
            loaded: false,
            balloon_frames: HashMap::new(),
            active_balloons: Vec::new(),
            font: None,
        }
    }

    /// Load ChatBalloon assets
    pub async fn load_assets(&mut self) {
        info!("Loading ChatBalloon assets...");
        
        match Self::load_from_wz().await {
            Ok(frames) => {
                self.balloon_frames = frames;
                self.loaded = true;
                info!("ChatBalloon assets loaded: {} styles", self.balloon_frames.len());
            }
            Err(e) => {
                error!("Failed to load ChatBalloon assets: {}", e);
            }
        }

        // Load font
        match load_ttf_font("https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/MaplestoryLight.ttf").await {
            Ok(font) => {
                self.font = Some(font);
            }
            Err(e) => {
                warn!("Failed to load font for chat balloons: {:?}", e);
            }
        }
    }

    async fn load_from_wz() -> Result<HashMap<usize, BalloonFrame>, String> {
        let bytes = AssetManager::fetch_and_cache(CHATBALLOON_URL, CHATBALLOON_CACHE).await
            .map_err(|e| format!("Failed to fetch ChatBalloon.img: {}", e))?;

        let wz_iv = guess_iv_from_wz_img(&bytes)
            .ok_or_else(|| "Unable to guess version from ChatBalloon.img".to_string())?;

        let byte_len = bytes.len();
        let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
        let cache_name_ref: wz_reader::WzNodeName = CHATBALLOON_CACHE.to_string().into();
        let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
        let root_node: WzNodeArc = WzNode::new(&CHATBALLOON_CACHE.into(), wz_image, None).into();

        root_node.write().unwrap().parse(&root_node)
            .map_err(|e| format!("Failed to parse ChatBalloon.img: {:?}", e))?;

        let mut frames = HashMap::new();

        // Load balloon styles 0-5 for player chat
        for style in 0..6 {
            let base_path = format!("{}", style);
            if let Ok(frame) = Self::load_balloon_frame(&root_node, &base_path).await {
                frames.insert(style, frame);
            }
        }

        // Load NPC balloon frame from ChatBalloon/npc (style 100 = npc)
        if let Ok(frame) = Self::load_balloon_frame(&root_node, "npc").await {
            frames.insert(100, frame);  // Use 100 for NPC balloon type
            info!("Loaded NPC chat balloon frame from ChatBalloon/npc");
        } else {
            warn!("Failed to load NPC chat balloon frame from ChatBalloon/npc");
        }

        Ok(frames)
    }

    async fn load_balloon_frame(root_node: &WzNodeArc, base_path: &str) -> Result<BalloonFrame, String> {
        let mut frame = BalloonFrame::default();

        // Load all frame pieces
        let pieces = [
            ("nw", &mut frame.nw),
            ("ne", &mut frame.ne),
            ("sw", &mut frame.sw),
            ("se", &mut frame.se),
            ("n", &mut frame.n),
            ("s", &mut frame.s),
            ("e", &mut frame.e),
            ("w", &mut frame.w),
            ("c", &mut frame.c),
            ("arrow", &mut frame.arrow),
        ];

        for (name, target) in pieces {
            let path = format!("{}/{}", base_path, name);
            if let Ok(tex) = Self::load_texture(root_node, &path).await {
                *target = Some(tex);
            }
        }

        Ok(frame)
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

    /// Show a chat balloon at the given position
    pub fn show_balloon(&mut self, text: &str, x: f32, y: f32, balloon_type: usize, lifetime: f32) {
        self.active_balloons.push(ActiveBalloon {
            text: text.to_string(),
            x,
            y,
            offset_y: 0.0,
            follows_player: false,
            lifetime,
            max_lifetime: lifetime,
            balloon_type,
        });
    }

    /// Show NPC dialog balloon
    pub fn show_npc_dialog(&mut self, text: &str, npc_x: f32, npc_y: f32) {
        // Position balloon above NPC, use NPC balloon type (100)
        self.show_balloon(text, npc_x, npc_y - 80.0, 100, 5.0);
    }

    /// Show player chat balloon above player (follows player)
    pub fn show_player_chat(&mut self, text: &str, player_x: f32, player_y: f32) {
        // Remove any existing player balloons
        self.active_balloons.retain(|b| !b.follows_player);
        
        self.active_balloons.push(ActiveBalloon {
            text: text.to_string(),
            x: player_x,
            y: player_y,
            offset_y: -70.0,  // Above player head
            follows_player: true,
            lifetime: 4.0,
            max_lifetime: 4.0,
            balloon_type: 0,
        });
    }

    /// Update player balloon position
    pub fn update_player_position(&mut self, player_x: f32, player_y: f32) {
        for balloon in &mut self.active_balloons {
            if balloon.follows_player {
                balloon.x = player_x;
                balloon.y = player_y;
            }
        }
    }

    /// Update balloons (decrease lifetime, remove expired)
    pub fn update(&mut self, dt: f32) {
        // Update lifetimes
        for balloon in &mut self.active_balloons {
            balloon.lifetime -= dt;
        }

        // Remove expired balloons
        self.active_balloons.retain(|b| b.lifetime > 0.0);
    }

    /// Draw all active balloons
    pub fn draw(&self, camera_x: f32, camera_y: f32) {
        for balloon in &self.active_balloons {
            self.draw_balloon(balloon, camera_x, camera_y);
        }
    }

    fn draw_balloon(&self, balloon: &ActiveBalloon, camera_x: f32, camera_y: f32) {
        let screen_x = balloon.x - camera_x;
        let screen_y = balloon.y + balloon.offset_y - camera_y;

        // Calculate text dimensions
        let font_size = 12.0;
        let padding = 10.0;
        let max_width = 200.0;

        // Word wrap text
        let lines = self.wrap_text(&balloon.text, max_width, font_size);
        let line_height = 16.0;
        let text_height = lines.len() as f32 * line_height;
        let text_width = lines.iter()
            .map(|l| measure_text(l, self.font.as_ref(), font_size as u16, 1.0).width)
            .fold(0.0f32, |a, b| a.max(b));

        let balloon_width = text_width + padding * 2.0;
        let balloon_height = text_height + padding * 2.0;

        // Get balloon frame
        let frame = self.balloon_frames.get(&balloon.balloon_type);

        // Calculate fade alpha based on remaining lifetime
        let alpha = if balloon.lifetime < 1.0 {
            (balloon.lifetime * 255.0) as u8
        } else {
            255
        };

        // Draw balloon background
        if let Some(frame) = frame {
            self.draw_9slice_balloon(frame, screen_x - balloon_width / 2.0, screen_y, balloon_width, balloon_height, alpha);
        } else {
            // Fallback: simple rectangle
            let bg_color = Color::from_rgba(255, 255, 255, alpha);
            let border_color = Color::from_rgba(0, 0, 0, alpha);
            
            draw_rectangle(
                screen_x - balloon_width / 2.0,
                screen_y,
                balloon_width,
                balloon_height,
                bg_color,
            );
            draw_rectangle_lines(
                screen_x - balloon_width / 2.0,
                screen_y,
                balloon_width,
                balloon_height,
                1.0,
                border_color,
            );

            // Draw arrow pointing down
            let arrow_x = screen_x;
            let arrow_y = screen_y + balloon_height;
            draw_triangle(
                Vec2::new(arrow_x - 8.0, arrow_y),
                Vec2::new(arrow_x + 8.0, arrow_y),
                Vec2::new(arrow_x, arrow_y + 10.0),
                bg_color,
            );
        }

        // Draw text
        let text_color = Color::from_rgba(0, 0, 0, alpha);
        let mut y = screen_y + padding + line_height;
        for line in &lines {
            let line_width = measure_text(line, self.font.as_ref(), font_size as u16, 1.0).width;
            let x = screen_x - line_width / 2.0;
            
            if let Some(font) = &self.font {
                draw_text_ex(line, x, y, TextParams {
                    font: Some(font),
                    font_size: font_size as u16,
                    color: text_color,
                    ..Default::default()
                });
            } else {
                draw_text(line, x, y, font_size, text_color);
            }
            y += line_height;
        }
    }

    fn draw_9slice_balloon(&self, frame: &BalloonFrame, x: f32, y: f32, width: f32, height: f32, alpha: u8) {
        let color = Color::from_rgba(255, 255, 255, alpha);

        // Get corner sizes
        let corner_w = frame.nw.as_ref().map(|t| t.texture.width()).unwrap_or(8.0);
        let corner_h = frame.nw.as_ref().map(|t| t.texture.height()).unwrap_or(8.0);

        // Draw corners
        if let Some(nw) = &frame.nw {
            draw_texture(&nw.texture, x, y, color);
        }
        if let Some(ne) = &frame.ne {
            draw_texture(&ne.texture, x + width - corner_w, y, color);
        }
        if let Some(sw) = &frame.sw {
            draw_texture(&sw.texture, x, y + height - corner_h, color);
        }
        if let Some(se) = &frame.se {
            draw_texture(&se.texture, x + width - corner_w, y + height - corner_h, color);
        }

        // Draw edges (stretched)
        if let Some(n) = &frame.n {
            let edge_width = width - corner_w * 2.0;
            draw_texture_ex(&n.texture, x + corner_w, y, color, DrawTextureParams {
                dest_size: Some(Vec2::new(edge_width, n.texture.height())),
                ..Default::default()
            });
        }
        if let Some(s) = &frame.s {
            let edge_width = width - corner_w * 2.0;
            draw_texture_ex(&s.texture, x + corner_w, y + height - corner_h, color, DrawTextureParams {
                dest_size: Some(Vec2::new(edge_width, s.texture.height())),
                ..Default::default()
            });
        }
        if let Some(w_tex) = &frame.w {
            let edge_height = height - corner_h * 2.0;
            draw_texture_ex(&w_tex.texture, x, y + corner_h, color, DrawTextureParams {
                dest_size: Some(Vec2::new(w_tex.texture.width(), edge_height)),
                ..Default::default()
            });
        }
        if let Some(e_tex) = &frame.e {
            let edge_height = height - corner_h * 2.0;
            draw_texture_ex(&e_tex.texture, x + width - corner_w, y + corner_h, color, DrawTextureParams {
                dest_size: Some(Vec2::new(e_tex.texture.width(), edge_height)),
                ..Default::default()
            });
        }

        // Draw center (stretched)
        if let Some(c) = &frame.c {
            let center_width = width - corner_w * 2.0;
            let center_height = height - corner_h * 2.0;
            draw_texture_ex(&c.texture, x + corner_w, y + corner_h, color, DrawTextureParams {
                dest_size: Some(Vec2::new(center_width, center_height)),
                ..Default::default()
            });
        }

        // Draw arrow
        if let Some(arrow) = &frame.arrow {
            let arrow_x = x + width / 2.0 - arrow.texture.width() / 2.0;
            let arrow_y = y + height;
            draw_texture(&arrow.texture, arrow_x, arrow_y, color);
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

    /// Check if loaded
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }
}

impl Default for ChatBalloonSystem {
    fn default() -> Self {
        Self::new()
    }
}
