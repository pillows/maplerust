use macroquad::prelude::*;
use crate::assets::{AssetManager, FrameData};
use std::sync::Arc;
use wz_reader::{WzNode, WzNodeArc, WzNodeCast, WzReader, WzImage};
use wz_reader::version::guess_iv_from_wz_img;

/// Cursor state determines which cursor animation to display
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CursorState {
    Default,       // Cursor/0 - normal cursor
    NpcHover,      // Cursor/1 - hovering over NPC (animated)
    Dragging,      // Cursor/5 - dragging something
    RightClick,    // Cursor/13 - right-clickable item
}

/// Manages MapleStory cursors with animations
pub struct CursorManager {
    // Cursor animations for each state
    default_cursor: Vec<FrameData>,
    npc_hover_cursor: Vec<FrameData>,
    dragging_cursor: Vec<FrameData>,
    right_click_cursor: Vec<FrameData>,

    // Current state and animation
    current_state: CursorState,
    current_frame: usize,
    frame_timer: f32,
    frame_duration: f32, // Time per frame in seconds

    loaded: bool,
}

impl CursorManager {
    pub fn new() -> Self {
        Self {
            default_cursor: Vec::new(),
            npc_hover_cursor: Vec::new(),
            dragging_cursor: Vec::new(),
            right_click_cursor: Vec::new(),
            current_state: CursorState::Default,
            current_frame: 0,
            frame_timer: 0.0,
            frame_duration: 0.3, // 300ms per frame (~3 FPS)
            loaded: false,
        }
    }

    /// Load all cursor animations from WZ
    pub async fn load_cursors(&mut self) {
        let url = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/UI/Basic.img";
        let cache_path = "/01/UI/Basic.img";

        match Self::load_cursor_from_wz(url, cache_path).await {
            Ok((default, npc_hover, dragging, right_click)) => {
                // Verify all cursors have at least one frame
                if default.is_empty() || npc_hover.is_empty() || dragging.is_empty() || right_click.is_empty() {
                    error!("Cursor animations incomplete");
                    self.loaded = false;
                    return;
                }

                self.default_cursor = default;
                self.npc_hover_cursor = npc_hover;
                self.dragging_cursor = dragging;
                self.right_click_cursor = right_click;
                self.loaded = true;
            }
            Err(e) => {
                error!("Failed to load cursors: {}", e);
                self.loaded = false;
            }
        }
    }

    /// Load cursor animations from WZ file
    async fn load_cursor_from_wz(
        url: &str,
        cache_path: &str,
    ) -> Result<(Vec<FrameData>, Vec<FrameData>, Vec<FrameData>, Vec<FrameData>), String> {
        // Fetch the WZ file
        let bytes = AssetManager::fetch_and_cache(url, cache_path).await?;

        // Parse WZ file
        let wz_iv = guess_iv_from_wz_img(&bytes)
            .ok_or_else(|| "Unable to guess version from Basic.img".to_string())?;

        let byte_len = bytes.len();
        let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
        let cache_name_ref: wz_reader::WzNodeName = cache_path.to_string().into();
        let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
        let root_node: WzNodeArc = WzNode::new(&cache_path.to_string().into(), wz_image, None).into();

        root_node.write().unwrap().parse(&root_node)
            .map_err(|e| format!("Failed to parse Basic.img: {:?}", e))?;

        // Load each cursor type
        let default = Self::load_cursor_animation(&root_node, "Cursor/0").await?;
        let npc_hover = Self::load_cursor_animation(&root_node, "Cursor/1").await?;
        let dragging = Self::load_cursor_animation(&root_node, "Cursor/5").await?;
        let right_click = Self::load_cursor_animation(&root_node, "Cursor/13").await?;

        Ok((default, npc_hover, dragging, right_click))
    }

    /// Load a specific cursor animation path (e.g., "Cursor/0")
    async fn load_cursor_animation(root_node: &WzNodeArc, path: &str) -> Result<Vec<FrameData>, String> {
        let cursor_node = root_node
            .read()
            .unwrap()
            .at_path_parsed(path)
            .map_err(|e| format!("Cursor path '{}' not found: {:?}", path, e))?;

        let mut frames = Vec::new();
        let cursor_read = cursor_node.read().unwrap();

        // Iterate through numbered frames (0, 1, 2, ...)
        let mut frame_num = 0;
        loop {
            let frame_path = frame_num.to_string();
            if let Some(frame_node) = cursor_read.children.get(frame_path.as_str()) {
                // Load the PNG texture and origin
                if let Ok(texture_data) = Self::load_frame_texture(frame_node).await {
                    frames.push(texture_data);
                    frame_num += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if frames.is_empty() {
            return Err(format!("No frames found for cursor path: {}", path));
        }

        Ok(frames)
    }

    /// Load texture and origin from a cursor frame node
    async fn load_frame_texture(frame_node: &WzNodeArc) -> Result<FrameData, String> {
        let frame_read = frame_node.read().unwrap();

        // Parse the frame node to ensure PNG is loaded
        drop(frame_read);
        frame_node.write().unwrap().parse(frame_node)
            .map_err(|e| format!("Failed to parse frame: {:?}", e))?;

        let frame_read = frame_node.read().unwrap();

        // Get PNG data
        let png = frame_read.try_as_png()
            .ok_or("Frame is not a PNG")?;

        let png_data = png.extract_png()
            .map_err(|e| format!("Failed to extract PNG: {:?}", e))?;

        // Convert DynamicImage to RGBA8 bytes for texture creation
        let rgba_img = png_data.to_rgba8();
        let width = rgba_img.width() as u16;
        let height = rgba_img.height() as u16;
        let bytes = rgba_img.into_raw();

        // Load texture from RGBA bytes
        let texture = Texture2D::from_rgba8(width, height, &bytes);

        // Get origin (cursor hotspot)
        let origin_x = if let Some(origin_node) = frame_read.children.get("origin") {
            let origin_read = origin_node.read().unwrap();
            if let Some(vec) = origin_read.try_as_vector2d() {
                vec.0  // Vector2D is a tuple struct (x, y)
            } else {
                0
            }
        } else {
            0
        };

        let origin_y = if let Some(origin_node) = frame_read.children.get("origin") {
            let origin_read = origin_node.read().unwrap();
            if let Some(vec) = origin_read.try_as_vector2d() {
                vec.1  // Vector2D is a tuple struct (x, y)
            } else {
                0
            }
        } else {
            0
        };

        Ok(FrameData {
            texture,
            origin: Vec2::new(origin_x as f32, origin_y as f32),
        })
    }

    /// Update cursor animation
    pub fn update(&mut self, dt: f32) {
        if !self.loaded {
            return;
        }

        self.frame_timer += dt;

        // Get current animation frame count
        let frame_count = match self.current_state {
            CursorState::Default => self.default_cursor.len(),
            CursorState::NpcHover => self.npc_hover_cursor.len(),
            CursorState::Dragging => self.dragging_cursor.len(),
            CursorState::RightClick => self.right_click_cursor.len(),
        };

        // Safety check: if frame_count is 0, don't update
        if frame_count == 0 {
            return;
        }

        // Advance frame if needed
        if frame_count > 1 && self.frame_timer >= self.frame_duration {
            self.frame_timer = 0.0;
            self.current_frame = (self.current_frame + 1) % frame_count;
        }
    }

    /// Set the cursor state
    pub fn set_state(&mut self, state: CursorState) {
        if self.current_state != state {
            self.current_state = state;
            self.current_frame = 0;
            self.frame_timer = 0.0;
        }
    }

    /// Draw the cursor at the mouse position
    pub fn draw(&self) {
        if !self.loaded {
            return;
        }

        // Get mouse position
        let (mouse_x, mouse_y) = mouse_position();

        // Get current animation frames
        let frames = match self.current_state {
            CursorState::Default => &self.default_cursor,
            CursorState::NpcHover => &self.npc_hover_cursor,
            CursorState::Dragging => &self.dragging_cursor,
            CursorState::RightClick => &self.right_click_cursor,
        };

        if frames.is_empty() {
            return;
        }

        // Safety check: ensure current_frame is within bounds
        let frame_index = if self.current_frame < frames.len() {
            self.current_frame
        } else {
            0
        };

        // Get current frame safely
        let frame = &frames[frame_index];

        // Draw cursor at mouse position, offset by origin (hotspot)
        draw_texture(
            &frame.texture,
            mouse_x - frame.origin.x,
            mouse_y - frame.origin.y,
            WHITE,
        );
    }

    /// Check if cursors are loaded
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }
}

impl Default for CursorManager {
    fn default() -> Self {
        Self::new()
    }
}
