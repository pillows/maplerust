use macroquad::prelude::*;
use crate::assets::AssetManager;
use std::sync::Arc;
use std::collections::HashMap;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzReader, WzNodeCast};

const CHARACTER_BODY_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/Character/00002000.img";
const CHARACTER_BODY_CACHE: &str = "/01/Character/00002000.img";
const CHARACTER_HEAD_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/Character/00012010.img";
const CHARACTER_HEAD_CACHE: &str = "/01/Character/00012010.img";

#[derive(Clone, Copy, PartialEq)]
pub enum CharacterState { Stand, Move, Jump, Fall }

#[derive(Clone)]
struct CharacterFrame {
    body_texture: Texture2D,
    body_origin: Vec2,
    head_texture: Option<Texture2D>,
    head_origin: Vec2,
    head_offset: Vec2, // neck position from body
    delay: u32,
}

#[derive(Clone)]
struct CharacterAnimation {
    frames: Vec<CharacterFrame>,
    current_frame: usize,
    frame_timer: f32,
}

impl CharacterAnimation {
    fn new(frames: Vec<CharacterFrame>) -> Self {
        Self { frames, current_frame: 0, frame_timer: 0.0 }
    }

    fn update(&mut self, dt: f32) {
        if self.frames.is_empty() { return; }
        self.frame_timer += dt * 1000.0;
        let delay = self.frames[self.current_frame].delay as f32;
        if self.frame_timer >= delay {
            self.frame_timer = 0.0;
            self.current_frame = (self.current_frame + 1) % self.frames.len();
        }
    }

    fn get_current_frame(&self) -> Option<&CharacterFrame> {
        self.frames.get(self.current_frame)
    }
}

#[derive(Clone)]
pub struct CharacterRenderer {
    loaded: bool,
    animations: HashMap<String, CharacterAnimation>,
    facing_right: bool,
}

impl CharacterRenderer {
    pub fn new() -> Self {
        Self { loaded: false, animations: HashMap::new(), facing_right: true }
    }

    pub async fn load_assets(&mut self) {
        info!("CharacterRenderer: Loading assets...");
        match Self::load_from_wz().await {
            Ok(anims) => {
                let count = anims.len();
                self.animations = anims;
                self.loaded = !self.animations.is_empty();
                info!("CharacterRenderer: Loaded {} animations", count);
            }
            Err(e) => {
                error!("CharacterRenderer: Failed: {}", e);
                self.loaded = false;
            }
        }
    }

    async fn load_from_wz() -> Result<HashMap<String, CharacterAnimation>, String> {
        // Load body
        let body_bytes = AssetManager::fetch_and_cache(CHARACTER_BODY_URL, CHARACTER_BODY_CACHE).await
            .map_err(|e| format!("Failed to fetch body: {}", e))?;
        let body_iv = guess_iv_from_wz_img(&body_bytes).ok_or("No body IV")?;
        let body_len = body_bytes.len();
        let body_reader = Arc::new(WzReader::from_buff(&body_bytes).with_iv(body_iv));
        let body_name: wz_reader::WzNodeName = CHARACTER_BODY_CACHE.to_string().into();
        let body_img = WzImage::new(&body_name, 0, body_len, &body_reader);
        let body_node: WzNodeArc = WzNode::new(&CHARACTER_BODY_CACHE.into(), body_img, None).into();
        body_node.write().unwrap().parse(&body_node).map_err(|e| format!("Body parse: {:?}", e))?;

        // Load head
        let head_bytes = AssetManager::fetch_and_cache(CHARACTER_HEAD_URL, CHARACTER_HEAD_CACHE).await
            .map_err(|e| format!("Failed to fetch head: {}", e))?;
        let head_iv = guess_iv_from_wz_img(&head_bytes).ok_or("No head IV")?;
        let head_len = head_bytes.len();
        let head_reader = Arc::new(WzReader::from_buff(&head_bytes).with_iv(head_iv));
        let head_name: wz_reader::WzNodeName = CHARACTER_HEAD_CACHE.to_string().into();
        let head_img = WzImage::new(&head_name, 0, head_len, &head_reader);
        let head_node: WzNodeArc = WzNode::new(&CHARACTER_HEAD_CACHE.into(), head_img, None).into();
        head_node.write().unwrap().parse(&head_node).map_err(|e| format!("Head parse: {:?}", e))?;

        let mut animations = HashMap::new();

        for (anim_name, key) in [("stand", "stand1"), ("move", "walk1"), ("jump", "jump")] {
            if let Ok(frames) = Self::load_animation(&body_node, &head_node, key).await {
                if !frames.is_empty() {
                    info!("Loaded {} with {} frames", anim_name, frames.len());
                    animations.insert(anim_name.to_string(), CharacterAnimation::new(frames));
                }
            }
        }

        Ok(animations)
    }

    async fn load_animation(body_root: &WzNodeArc, head_root: &WzNodeArc, anim_name: &str) -> Result<Vec<CharacterFrame>, String> {
        let mut frames = Vec::new();

        let body_anim = {
            let r = body_root.read().unwrap();
            match r.at_path(anim_name) { Some(n) => n.clone(), None => return Ok(frames) }
        };
        body_anim.write().unwrap().parse(&body_anim).ok();

        let head_anim = {
            let r = head_root.read().unwrap();
            r.at_path(anim_name).map(|n| n.clone())
        };
        if let Some(ref ha) = head_anim {
            ha.write().unwrap().parse(ha).ok();
        }

        let frame_nums: Vec<i32> = {
            let r = body_anim.read().unwrap();
            let mut nums: Vec<i32> = r.children.keys().filter_map(|k| k.parse().ok()).collect();
            nums.sort();
            nums
        };

        for frame_num in frame_nums {
            let frame_str = frame_num.to_string();
            
            // Load body
            let body_frame = {
                let r = body_anim.read().unwrap();
                r.at_path(&frame_str).map(|n| n.clone())
            };
            let body_frame = match body_frame { Some(f) => f, None => continue };
            body_frame.write().unwrap().parse(&body_frame).ok();

            let (body_tex, body_origin, neck_pos) = {
                let fr = body_frame.read().unwrap();
                let body_node = match fr.at_path("body") { Some(n) => n.clone(), None => continue };
                body_node.write().unwrap().parse(&body_node).ok();
                
                let br = body_node.read().unwrap();
                let png = match br.try_as_png() { Some(p) => p, None => continue };
                let img = match png.extract_png() { Ok(i) => i, Err(_) => continue };
                let rgba = img.to_rgba8();
                let tex = Texture2D::from_rgba8(rgba.width() as u16, rgba.height() as u16, &rgba.into_raw());

                let origin = br.children.get("origin").and_then(|o| {
                    o.read().unwrap().try_as_vector2d().map(|v| Vec2::new(v.0 as f32, v.1 as f32))
                }).unwrap_or(Vec2::ZERO);

                info!("Frame {} body origin: ({}, {}), tex size: {}x{}", frame_num, origin.x, origin.y, tex.width(), tex.height());

                // Get neck position from body/map/neck
                let neck = br.at_path("map/neck").and_then(|n| {
                    n.read().unwrap().try_as_vector2d().map(|v| Vec2::new(v.0 as f32, v.1 as f32))
                }).unwrap_or(Vec2::new(0.0, -10.0));

                (tex, origin, neck)
            };

            // Load head
            let (head_tex, head_origin, head_neck) = if let Some(ref ha) = head_anim {
                let head_frame = {
                    let r = ha.read().unwrap();
                    r.at_path(&frame_str).map(|n| n.clone())
                };
                if let Some(hf) = head_frame {
                    hf.write().unwrap().parse(&hf).ok();
                    let hfr = hf.read().unwrap();
                    
                    // Try front/head first
                    let head_node = hfr.at_path("front/head").or_else(|| hfr.at_path("head"));
                    if let Some(hn) = head_node {
                        hn.write().unwrap().parse(&hn).ok();
                        let hr = hn.read().unwrap();
                        if let Some(png) = hr.try_as_png() {
                            if let Ok(img) = png.extract_png() {
                                let rgba = img.to_rgba8();
                                let tex = Texture2D::from_rgba8(rgba.width() as u16, rgba.height() as u16, &rgba.into_raw());
                                let origin = hr.children.get("origin").and_then(|o| {
                                    o.read().unwrap().try_as_vector2d().map(|v| Vec2::new(v.0 as f32, v.1 as f32))
                                }).unwrap_or(Vec2::ZERO);
                                let neck = hr.at_path("map/neck").and_then(|n| {
                                    n.read().unwrap().try_as_vector2d().map(|v| Vec2::new(v.0 as f32, v.1 as f32))
                                }).unwrap_or(Vec2::ZERO);
                                (Some(tex), origin, neck)
                            } else { (None, Vec2::ZERO, Vec2::ZERO) }
                        } else { (None, Vec2::ZERO, Vec2::ZERO) }
                    } else { (None, Vec2::ZERO, Vec2::ZERO) }
                } else { (None, Vec2::ZERO, Vec2::ZERO) }
            } else { (None, Vec2::ZERO, Vec2::ZERO) };

            // Calculate head offset: body_neck - head_neck
            let head_offset = neck_pos - head_neck;

            frames.push(CharacterFrame {
                body_texture: body_tex,
                body_origin,
                head_texture: head_tex,
                head_origin,
                head_offset,
                delay: 200,
            });
        }

        Ok(frames)
    }

    pub fn update(&mut self, dt: f32, state: CharacterState, facing_right: bool) {
        self.facing_right = facing_right;
        if !self.loaded { return; }
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
        let anim_name = match state {
            CharacterState::Stand => "stand",
            CharacterState::Move => "move",
            CharacterState::Jump | CharacterState::Fall => "jump",
        };

        if self.loaded {
            if let Some(anim) = self.animations.get(anim_name) {
                if let Some(frame) = anim.get_current_frame() {
                    // MapleStory sprites face LEFT by default, so flip when facing right
                    let flip = self.facing_right;
                    
                    // Draw body - when flipping, need to adjust X origin
                    let body_x = if flip {
                        x - (frame.body_texture.width() - frame.body_origin.x)
                    } else {
                        x - frame.body_origin.x
                    };
                    let body_y = y - frame.body_origin.y;
                    draw_texture_ex(&frame.body_texture, body_x, body_y, WHITE, DrawTextureParams {
                        flip_x: flip, ..Default::default()
                    });

                    // Draw head - when flipping, need to adjust X origin
                    if let Some(ref head_tex) = frame.head_texture {
                        let head_x = if flip {
                            x - (head_tex.width() - frame.head_origin.x) - frame.head_offset.x
                        } else {
                            x - frame.head_origin.x + frame.head_offset.x
                        };
                        let head_y = y - frame.head_origin.y + frame.head_offset.y;
                        draw_texture_ex(head_tex, head_x, head_y, WHITE, DrawTextureParams {
                            flip_x: flip, ..Default::default()
                        });
                    }
                    return;
                }
            }
        }

        // Fallback
        draw_rectangle(x - 15.0, y - 50.0, 30.0, 50.0, BLUE);
    }

    /// Draw with a specific timer value (for fake players)
    pub fn draw_with_timer(&self, x: f32, y: f32, state: CharacterState, facing_right: bool, timer: f32) {
        let anim_name = match state {
            CharacterState::Stand => "stand",
            CharacterState::Move => "move",
            CharacterState::Jump | CharacterState::Fall => "jump",
        };

        if self.loaded {
            if let Some(anim) = self.animations.get(anim_name) {
                if !anim.frames.is_empty() {
                    // Calculate frame index from timer
                    let total_delay: f32 = anim.frames.iter().map(|f| f.delay as f32).sum();
                    let time_ms = (timer * 1000.0) % total_delay.max(1.0);
                    let mut accumulated = 0.0;
                    let mut frame_idx = 0;
                    for (i, frame) in anim.frames.iter().enumerate() {
                        accumulated += frame.delay as f32;
                        if time_ms < accumulated {
                            frame_idx = i;
                            break;
                        }
                    }
                    
                    if let Some(frame) = anim.frames.get(frame_idx) {
                        let flip = facing_right;
                        let body_x = if flip {
                            x - (frame.body_texture.width() - frame.body_origin.x)
                        } else {
                            x - frame.body_origin.x
                        };
                        let body_y = y - frame.body_origin.y;
                        draw_texture_ex(&frame.body_texture, body_x, body_y, WHITE, DrawTextureParams {
                            flip_x: flip, ..Default::default()
                        });

                        if let Some(ref head_tex) = frame.head_texture {
                            let head_x = if flip {
                                x - (head_tex.width() - frame.head_origin.x) - frame.head_offset.x
                            } else {
                                x - frame.head_origin.x + frame.head_offset.x
                            };
                            let head_y = y - frame.head_origin.y + frame.head_offset.y;
                            draw_texture_ex(head_tex, head_x, head_y, WHITE, DrawTextureParams {
                                flip_x: flip, ..Default::default()
                            });
                        }
                        return;
                    }
                }
            }
        }
        // Fallback
        draw_rectangle(x - 15.0, y - 50.0, 30.0, 50.0, BLUE);
    }

    pub fn is_loaded(&self) -> bool { self.loaded }
}


impl Default for CharacterRenderer {
    fn default() -> Self { Self::new() }
}
