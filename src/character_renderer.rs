use macroquad::prelude::*;
use crate::assets::AssetManager;
use std::sync::Arc;
use std::collections::HashMap;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzReader, WzNodeCast};

// Character head asset - 00012010.img contains head sprites for different animations
const CHARACTER_HEAD_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/Character/00012010.img";
const CHARACTER_HEAD_CACHE: &str = "/01/Character/00012010.img";
// Body asset
const CHARACTER_BODY_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/Character/00002000.img";
const CHARACTER_BODY_CACHE: &str = "/01/Character/00002000.img";

/// Character animation state
#[derive(Clone, Copy, PartialEq)]
pub enum CharacterState {
    Stand,
    Move,
    Jump,
    Fall,
}

/// Frame data for character animations
#[derive(Clone)]
struct CharacterFrame {
    texture: Texture2D,
    origin: Vec2,
    delay: u32,  // Delay in milliseconds
}

/// Character animation data
struct CharacterAnimation {
    frames: Vec<CharacterFrame>,
    current_frame: usize,
    frame_timer: f32,
}

impl CharacterAnimation {
    fn new(frames: Vec<CharacterFrame>) -> Self {
        Self {
            frames,
            current_frame: 0,
            frame_timer: 0.0,
        }
    }

    fn update(&mut self, dt: f32) {
        if self.frames.is_empty() {
            return;
        }

        self.frame_timer += dt * 1000.0; // Convert to milliseconds

        let current_delay = self.frames[self.current_frame].delay as f32;
        if self.frame_timer >= current_delay {
            self.frame_timer = 0.0;
            self.current_frame = (self.current_frame + 1) % self.frames.len();
        }
    }

    fn get_current_frame(&self) -> Option<&CharacterFrame> {
        self.frames.get(self.current_frame)
    }
}

/// Character renderer
pub struct CharacterRenderer {
    loaded: bool,
    animations: HashMap<String, CharacterAnimation>,
    facing_right: bool,
}

impl CharacterRenderer {
    pub fn new() -> Self {
        Self {
            loaded: false,
            animations: HashMap::new(),
            facing_right: true,
        }
    }

    pub async fn load_assets(&mut self) {
        match Self::load_from_wz().await {
            Ok(anims) => {
                self.animations = anims;
                self.loaded = true;
                info!("Character renderer loaded successfully");
            }
            Err(e) => {
                error!("Failed to load character assets: {}", e);
                self.loaded = false;
            }
        }
    }

    async fn load_from_wz() -> Result<HashMap<String, CharacterAnimation>, String> {
        // Load head sprites from 00012010.img
        let bytes = AssetManager::fetch_and_cache(CHARACTER_HEAD_URL, CHARACTER_HEAD_CACHE).await
            .map_err(|e| format!("Failed to fetch Character/00012010.img: {}", e))?;

        let wz_iv = guess_iv_from_wz_img(&bytes)
            .ok_or_else(|| "Unable to guess WZ version from Character file".to_string())?;

        let byte_len = bytes.len();
        let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
        let cache_name_ref: wz_reader::WzNodeName = CHARACTER_HEAD_CACHE.to_string().into();
        let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
        let root_node: WzNodeArc = WzNode::new(&CHARACTER_HEAD_CACHE.into(), wz_image, None).into();

        root_node.write().unwrap().parse(&root_node)
            .map_err(|e| format!("Failed to parse Character file: {:?}", e))?;

        let mut animations = HashMap::new();

        // Load stand animation from stand1 path (based on 00012010_structure.txt)
        if let Ok(frames) = Self::load_animation(&root_node, "stand1").await {
            if !frames.is_empty() {
                animations.insert("stand".to_string(), CharacterAnimation::new(frames));
            }
        }

        // Load walk animation from walk1 path
        if let Ok(frames) = Self::load_animation(&root_node, "walk1").await {
            if !frames.is_empty() {
                animations.insert("move".to_string(), CharacterAnimation::new(frames));
            }
        }

        // Load jump animation
        if let Ok(frames) = Self::load_animation(&root_node, "jump").await {
            if !frames.is_empty() {
                animations.insert("jump".to_string(), CharacterAnimation::new(frames));
            }
        }

        // Load alert animation as fallback
        if let Ok(frames) = Self::load_animation(&root_node, "alert").await {
            if !frames.is_empty() && !animations.contains_key("stand") {
                animations.insert("stand".to_string(), CharacterAnimation::new(frames));
            }
        }

        Ok(animations)
    }

    async fn load_animation(root_node: &WzNodeArc, anim_name: &str) -> Result<Vec<CharacterFrame>, String> {
        let mut frames = Vec::new();

        // Try to get animation node - return empty if not found
        let anim_node = {
            let root_read = root_node.read().unwrap();
            match root_read.at_path(anim_name) {
                Some(node) => node.clone(),
                None => return Ok(frames), // Return empty frames if animation doesn't exist
            }
        };

        if let Err(e) = anim_node.write().unwrap().parse(&anim_node) {
            return Ok(frames); // Return empty on parse error
        }

        let anim_read = anim_node.read().unwrap();
        
        // Get all frame numbers (0, 1, 2, etc.)
        let mut frame_numbers: Vec<i32> = anim_read.children.keys()
            .filter_map(|key| key.parse::<i32>().ok())
            .collect();
        frame_numbers.sort();

        for frame_num in frame_numbers {
            // Try to load head from front/head path within each frame
            let frame_path = format!("{}/front/head", frame_num);
            if let Ok(frame) = Self::load_frame_from_node(&anim_node, &frame_path).await {
                frames.push(frame);
            }
        }

        Ok(frames)
    }

    async fn load_frame_from_node(parent_node: &WzNodeArc, frame_path: &str) -> Result<CharacterFrame, String> {
        let node = {
            let parent_read = parent_node.read().unwrap();
            parent_read.at_path(frame_path)
                .ok_or_else(|| format!("Frame '{}' not found", frame_path))?
                .clone()
        };

        node.write().unwrap().parse(&node)
            .map_err(|e| format!("Failed to parse frame '{}': {:?}", frame_path, e))?;

        let node_read = node.read().unwrap();
        let png = node_read.try_as_png()
            .ok_or_else(|| format!("Frame '{}' is not a PNG", frame_path))?;

        let png_data = png.extract_png()
            .map_err(|e| format!("Failed to extract PNG at '{}': {:?}", frame_path, e))?;

        let rgba_img = png_data.to_rgba8();
        let width = rgba_img.width() as u16;
        let height = rgba_img.height() as u16;
        let bytes = rgba_img.into_raw();
        let texture = Texture2D::from_rgba8(width, height, &bytes);

        // Get origin
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

        // Get delay (default to 100ms if not specified)
        let delay = 100u32;

        Ok(CharacterFrame {
            texture,
            origin,
            delay,
        })
    }

    pub fn update(&mut self, dt: f32, state: CharacterState, facing_right: bool) {
        if !self.loaded {
            return;
        }

        self.facing_right = facing_right;

        // Update the current animation based on state
        let anim_name = match state {
            CharacterState::Stand => "stand",
            CharacterState::Move => "move",
            CharacterState::Jump | CharacterState::Fall => "jump",
        };

        if let Some(anim) = self.animations.get_mut(anim_name) {
            anim.update(dt);
        }
    }

    pub fn draw(&self, x: f32, y: f32, state: CharacterState) {
        if !self.loaded {
            // Fallback: draw blue rectangle
            draw_rectangle(x - 15.0, y - 30.0, 30.0, 60.0, BLUE);
            return;
        }

        // Determine which animation to use based on state
        let anim_name = match state {
            CharacterState::Stand => "stand",
            CharacterState::Move => "move",
            CharacterState::Jump | CharacterState::Fall => "jump",
        };

        if let Some(anim) = self.animations.get(anim_name) {
            if let Some(frame) = anim.get_current_frame() {
                let screen_x = x - frame.origin.x;
                let screen_y = y - frame.origin.y;

                let params = DrawTextureParams {
                    flip_x: !self.facing_right,
                    flip_y: false,
                    ..Default::default()
                };

                draw_texture_ex(&frame.texture, screen_x, screen_y, WHITE, params);
                return;
            }
        }

        // Fallback if animation not found
        draw_rectangle(x - 15.0, y - 30.0, 30.0, 60.0, BLUE);
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }
}

impl Default for CharacterRenderer {
    fn default() -> Self {
        Self::new()
    }
}
