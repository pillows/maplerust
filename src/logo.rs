use macroquad::prelude::*;
use crate::assets::FrameData;

#[derive(PartialEq)]
pub enum LogoState {
    PlayingNexon,
    PlayingWizet,
    Completed,
}

/// Animation state management
pub struct LogoAnimation {
    pub state: LogoState,
    pub current_frame: usize,
    pub frame_timer: f32,
    pub frame_duration: f32,
}

impl LogoAnimation {
    pub fn new() -> Self {
        Self {
            state: LogoState::PlayingWizet,
            current_frame: 0,
            frame_timer: 0.0,
            frame_duration: 0.02, // 50ms per frame (20 FPS)
        }
    }
}

/// Update logo animation state - plays Wizet first, then Nexon
pub fn update_logo_animation(
    animation: &mut LogoAnimation,
    dt: f32,
    wizet_frames: &[FrameData],
    nexon_frames: &[FrameData],
) {
    match animation.state {
        LogoState::PlayingWizet => {
            if wizet_frames.is_empty() {
                // If no frames, skip to Nexon
                info!("Wizet frames empty, skipping to Nexon");
                animation.state = LogoState::PlayingNexon;
                animation.current_frame = 0;
                animation.frame_timer = 0.0;
            } else {
                animation.frame_timer += dt;
                if animation.frame_timer >= animation.frame_duration {
                    animation.frame_timer = 0.0;
                    animation.current_frame += 1;
                    
                    // If we've completed Wizet animation, move to Nexon
                    if animation.current_frame >= wizet_frames.len() {
                        info!("Wizet animation complete, moving to Nexon");
                        animation.current_frame = 0;
                        animation.frame_timer = 0.0;
                        animation.state = LogoState::PlayingNexon;
                    }
                }
            }
        }
        LogoState::PlayingNexon => {
            if nexon_frames.is_empty() {
                // If no frames, mark as completed
                info!("Nexon frames empty, marking as completed");
                animation.state = LogoState::Completed;
            } else {
                animation.frame_timer += dt;
                if animation.frame_timer >= animation.frame_duration {
                    animation.frame_timer = 0.0;
                    animation.current_frame += 1;
                    
                    // If we've completed Nexon animation, mark as completed
                    if animation.current_frame >= nexon_frames.len() {
                        info!("Nexon animation complete");
                        animation.current_frame = 0;
                        animation.state = LogoState::Completed;
                    }
                }
            }
        }
        LogoState::Completed => {
            // Animation complete, do nothing
        }
    }
}

/// Generate debug text showing current animation state
pub fn get_debug_text(
    animation: &LogoAnimation,
    wizet_frames: &[FrameData],
    nexon_frames: &[FrameData],
) -> String {
    match animation.state {
        LogoState::PlayingWizet => {
            if !wizet_frames.is_empty() && animation.current_frame < wizet_frames.len() {
                let frame = &wizet_frames[animation.current_frame];
                let draw_x = frame.origin.x;
                let draw_y = frame.origin.y;
                format!(
                    "State: PlayingWizet (frame {}/{}) | Draw: ({:.0}, {:.0}) | Origin: ({:.0}, {:.0})",
                    animation.current_frame + 1,
                    wizet_frames.len(),
                    draw_x,
                    draw_y,
                    frame.origin.x,
                    frame.origin.y
                )
            } else {
                format!("State: PlayingWizet (frame {}/{})", animation.current_frame + 1, wizet_frames.len())
            }
        }
        LogoState::PlayingNexon => {
            if !nexon_frames.is_empty() && animation.current_frame < nexon_frames.len() {
                let frame = &nexon_frames[animation.current_frame];
                let wizet_origin_x = if !wizet_frames.is_empty() {
                    wizet_frames[0].origin.x
                } else {
                    0.0
                };
                let wizet_origin_y = if !wizet_frames.is_empty() {
                    wizet_frames[0].origin.y
                } else {
                    0.0
                };
                let draw_x = wizet_origin_x + frame.origin.x;
                let draw_y = wizet_origin_y + frame.origin.y;
                format!(
                    "State: PlayingNexon (frame {}/{}) | Draw: ({:.0}, {:.0}) | Wizet origin: ({:.0}, {:.0}) | Nexon origin: ({:.0}, {:.0})",
                    animation.current_frame + 1,
                    nexon_frames.len(),
                    draw_x,
                    draw_y,
                    wizet_origin_x,
                    wizet_origin_y,
                    frame.origin.x,
                    frame.origin.y
                )
            } else {
                format!("State: PlayingNexon (frame {}/{})", animation.current_frame + 1, nexon_frames.len())
            }
        }
        LogoState::Completed => "State: Completed".to_string(),
    }
}

/// Display the logo animation frames based on the current state
/// 
/// # Parameters
/// - `logo_state`: The current animation state (Wizet, Nexon, or Completed)
/// - `current_frame`: The current frame index
/// - `wizet_frames`: Vector of Wizet animation frames with origin data
/// - `nexon_frames`: Vector of Nexon animation frames with origin data
pub fn display_logos(
    logo_state: &LogoState,
    current_frame: usize,
    wizet_frames: &[FrameData],
    nexon_frames: &[FrameData],
) {
    match logo_state {
        LogoState::PlayingWizet => {
            if !wizet_frames.is_empty() && current_frame < wizet_frames.len() {
                let frame = &wizet_frames[current_frame];
                // Wizet logo: use origin coordinates directly, offset from (0,0)
                // The origin coordinates specify the absolute position on screen
                let draw_x = frame.origin.x;
                let draw_y = frame.origin.y;
                draw_texture(&frame.texture, draw_x, draw_y, WHITE);
                
                // Draw green border around the image
                let border_rect = Rect::new(
                    draw_x,
                    draw_y,
                    frame.texture.width(),
                    frame.texture.height(),
                );
                draw_rectangle_lines(border_rect.x, border_rect.y, border_rect.w, border_rect.h, 2.0, GREEN);
            } else {
                // Debug: Show why frame isn't displaying
                draw_text(
                    &format!("Wizet: {} frames, frame {}", wizet_frames.len(), current_frame),
                    300.0,
                    250.0,
                    20.0,
                    RED,
                );
            }
        }
        LogoState::PlayingNexon => {
            if !nexon_frames.is_empty() && current_frame < nexon_frames.len() {
                let frame = &nexon_frames[current_frame];
                // Nexon logo: centered on screen
                // Origin in WZ files is offset from top-left to anchor point
                // Calculate draw position: center position minus origin offset
                let screen_center_x = screen_width() / 2.0;
                let screen_center_y = screen_height() / 2.0;
                let draw_x = screen_center_x - frame.origin.x;
                let draw_y = screen_center_y - frame.origin.y;
                draw_texture(&frame.texture, draw_x, draw_y, WHITE);
                
                // Draw green border around the image
                let border_rect = Rect::new(
                    draw_x,
                    draw_y,
                    frame.texture.width(),
                    frame.texture.height(),
                );
                draw_rectangle_lines(border_rect.x, border_rect.y, border_rect.w, border_rect.h, 2.0, GREEN);
            } else {
                // Debug: Show why frame isn't displaying
                draw_text(
                    &format!("Nexon: {} frames, frame {}", nexon_frames.len(), current_frame),
                    300.0,
                    250.0,
                    20.0,
                    RED,
                );
            }
        }
        LogoState::Completed => {
            // Show completion text
            draw_text(
                "Logo animations completed!",
                250.0,
                300.0,
                30.0,
                GREEN,
            );
        }
    }
}

