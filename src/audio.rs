use macroquad::{audio, prelude::*};
use crate::assets::AssetManager;
use std::sync::Arc;
use wz_reader::{WzNode, WzNodeArc, WzNodeCast, WzReader, WzImage};
use wz_reader::version::guess_iv_from_wz_img;

/// Audio manager for handling background music and sound effects
pub struct AudioManager {
    current_bgm: Option<audio::Sound>,
    current_bgm_name: String,
    // Alternative Web Audio API playback (for Chrome compatibility)
    #[cfg(target_arch = "wasm32")]
    web_audio_context_id: Option<u32>, // ID for Web Audio API context
    #[cfg(target_arch = "wasm32")]
    pending_bgm: Option<(Vec<u8>, String)>, // Store pending BGM until user interaction
    #[cfg(target_arch = "wasm32")]
    audio_context_resumed: bool, // Track if AudioContext has been resumed
}

impl AudioManager {
    /// Create a new audio manager
    pub fn new() -> Self {
        Self {
            current_bgm: None,
            current_bgm_name: String::new(),
            #[cfg(target_arch = "wasm32")]
            web_audio_context_id: None,
            #[cfg(target_arch = "wasm32")]
            pending_bgm: None,
            #[cfg(target_arch = "wasm32")]
            audio_context_resumed: false,
        }
    }
    
    /// Resume audio context on first user interaction
    /// This should be called when user interacts with the page (click, keypress, etc.)
    #[cfg(target_arch = "wasm32")]
    pub async fn resume_audio_context(&mut self) {
        if !self.audio_context_resumed {
            extern "C" {
                fn web_audio_resume_context();
            }
            unsafe {
                web_audio_resume_context();
            }
            self.audio_context_resumed = true;
            info!("AudioContext resumed after user interaction");
            
            // Play any pending BGM
            if let Some((sound_data, bgm_name)) = self.pending_bgm.take() {
                info!("Playing pending BGM: {}", bgm_name);
                // Update current_bgm_name BEFORE playing so it doesn't get stopped
                self.current_bgm_name = bgm_name.clone();
                match self.play_with_web_audio(&sound_data, &bgm_name).await {
                    Ok(_) => {
                        info!("  → Pending BGM playback started successfully");
                    }
                    Err(e) => {
                        warn!("Failed to play pending BGM: {}", e);
                        // Clear the name if playback failed
                        self.current_bgm_name.clear();
                    }
                }
            }
        }
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn resume_audio_context(&mut self) {
        // No-op for non-WASM
    }

    /// Load and play BGM from map info
    /// The BGM string is in format: "Bgm13/Leafre"
    /// This will fetch from: "01/Sound/Bgm13.img/Leafre"
    pub async fn play_bgm(&mut self, bgm: &str) {
        info!("play_bgm called with: '{}'", bgm);

        // Don't reload if it's the same BGM
        if self.current_bgm_name == bgm {
            #[cfg(target_arch = "wasm32")]
            {
                // If AudioContext was just resumed and we have pending BGM, play it
                if self.audio_context_resumed && self.pending_bgm.is_some() {
                    info!("AudioContext resumed, will play pending BGM");
                } else if self.web_audio_context_id.is_some() {
                    info!("BGM '{}' is already playing, skipping reload", bgm);
                    return;
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                if self.current_bgm.is_some() {
                    info!("BGM '{}' is already playing, skipping reload", bgm);
                    return;
                }
            }
        }

        // Stop current BGM if any (this will stop ALL Web Audio sources)
        info!("play_bgm: Stopping any previous BGM before playing '{}'", bgm);
        self.stop_bgm();

        // Parse BGM string (format: "ImgName/TrackName")
        if bgm.is_empty() {
            warn!("Empty BGM string provided");
            return;
        }

        let parts: Vec<&str> = bgm.split('/').collect();
        if parts.len() != 2 {
            warn!("Invalid BGM format: '{}' (expected 'ImgName/TrackName')", bgm);
            return;
        }

        let img_name = parts[0];
        let track_name = parts[1];

        info!("Loading BGM: {} from {}.img", track_name, img_name);

        // Load the sound from WZ and try both macroquad and Web Audio API
        match self.load_sound_from_wz_with_web_audio(img_name, track_name).await {
            Ok((sound, sound_data)) => {
                info!("Sound loaded successfully, starting playback...");
                
                // Use Web Audio API directly (primary method for Chrome compatibility)
                // Skip macroquad audio to avoid duplicate playback
                #[cfg(target_arch = "wasm32")]
                {
                    // Check if AudioContext has been resumed (user interaction required)
                    if !self.audio_context_resumed {
                        // Store pending BGM until user interaction
                        info!("  → AudioContext not resumed yet, storing BGM for later playback");
                        self.pending_bgm = Some((sound_data, bgm.to_string()));
                        self.current_bgm = Some(sound);
                        self.current_bgm_name = bgm.to_string();
                        info!("  → BGM will play automatically after first user interaction");
                        return;
                    }
                    
                    // Use Web Audio API - it's more reliable in Chrome
                    // Set current_bgm_name BEFORE playing to prevent it from being stopped
                    self.current_bgm_name = bgm.to_string();
                    match self.play_with_web_audio(&sound_data, track_name).await {
                        Ok(_) => {
                            info!("  → Web Audio API playback started successfully");
                            self.current_bgm = Some(sound);
                            self.current_bgm_name = bgm.to_string();
                        }
                        Err(e) => {
                            warn!("Web Audio API playback failed: {}", e);
                            // Fallback: try macroquad audio if Web Audio fails
                            info!("  → Falling back to macroquad audio");
                            audio::play_sound(
                                &sound,
                                audio::PlaySoundParams {
                                    looped: true,
                                    volume: 0.5,
                                },
                            );
                            self.current_bgm = Some(sound);
                            self.current_bgm_name = bgm.to_string();
                        }
                    }
                }
                
                #[cfg(not(target_arch = "wasm32"))]
                {
                    // For non-WASM builds, use macroquad audio
                    audio::play_sound(
                        &sound,
                        audio::PlaySoundParams {
                            looped: true,
                            volume: 0.5,
                        },
                    );
                    self.current_bgm = Some(sound);
                    self.current_bgm_name = bgm.to_string();
                }
                
                info!("✓ BGM playback started: {}", bgm);
            }
            Err(e) => {
                error!("✗ Failed to load BGM '{}': {}", bgm, e);
            }
        }
    }

    /// Stop the current BGM
    pub fn stop_bgm(&mut self) {
        if self.current_bgm_name.is_empty() && self.current_bgm.is_none() {
            info!("stop_bgm called but no BGM is currently playing");
            return;
        }

        info!("stop_bgm: Stopping BGM '{}'", self.current_bgm_name);

        // Stop macroquad audio
        if let Some(sound) = &self.current_bgm {
            audio::stop_sound(sound);
            self.current_bgm = None;
            info!("  → Macroquad audio stopped");
        }

        // Stop Web Audio API playback - stop ALL sources
        #[cfg(target_arch = "wasm32")]
        {
            self.stop_web_audio();
            // Clear pending BGM if any
            if self.pending_bgm.is_some() {
                info!("  → Cleared pending BGM");
                self.pending_bgm = None;
            }
        }

        self.current_bgm_name.clear();
        info!("  → BGM stopped completely");
    }
    
    /// Play sound using Web Audio API directly (Chrome compatibility)
    #[cfg(target_arch = "wasm32")]
    async fn play_with_web_audio(&mut self, sound_data: &[u8], _track_name: &str) -> Result<(), String> {
        extern "C" {
            fn web_audio_play(data_ptr: *const u8, data_len: u32, looped: u32, volume: f32) -> u32;
        }
        
        unsafe {
            let context_id = web_audio_play(
                sound_data.as_ptr(),
                sound_data.len() as u32,
                1, // looped = true
                0.5, // volume = 50%
            );
            
            if context_id != 0 {
                self.web_audio_context_id = Some(context_id);
                info!("  → Web Audio API playback initiated (context ID: {})", context_id);
                info!("  → Note: Audio will start after user interaction (browser autoplay policy)");
                Ok(())
            } else {
                warn!("  → Web Audio API playback failed to start (returned 0)");
                Err("Web Audio API returned 0".to_string())
            }
        }
    }
    
    /// Stop Web Audio API playback
    #[cfg(target_arch = "wasm32")]
    fn stop_web_audio(&mut self) {
        extern "C" {
            fn web_audio_stop(context_id: u32);
        }
        // Always stop ALL sources by passing 0 to ensure no audio leaks when changing maps
        info!("  → Calling web_audio_stop(0) to stop ALL Web Audio sources");
        unsafe {
            web_audio_stop(0); // 0 means stop all sources
        }
        self.web_audio_context_id = None;
        info!("  → All Web Audio sources stopped (web_audio_context_id cleared)");
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    async fn play_with_web_audio(&mut self, _sound_data: &[u8], _track_name: &str) -> Result<(), String> {
        Ok(()) // No-op for non-WASM
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    fn stop_web_audio(&mut self) {
        // No-op for non-WASM
    }

    /// Set BGM volume (0.0 to 1.0)
    pub fn set_volume(&self, volume: f32) {
        if let Some(sound) = &self.current_bgm {
            audio::set_sound_volume(sound, volume.clamp(0.0, 1.0));
        }
    }

    /// Load a sound from WZ file and return both macroquad Sound and raw bytes for Web Audio API
    async fn load_sound_from_wz_with_web_audio(&self, img_name: &str, track_name: &str) -> Result<(audio::Sound, Vec<u8>), String> {
        // First load the sound data
        let sound_data = self.load_sound_data_from_wz(img_name, track_name).await?;
        
        // Then load into macroquad
        let sound = audio::load_sound_from_bytes(&sound_data)
            .await
            .map_err(|e| format!("Failed to load sound from bytes: {:?}", e))?;
        
        Ok((sound, sound_data))
    }
    
    /// Load raw sound data from WZ file
    async fn load_sound_data_from_wz(&self, img_name: &str, track_name: &str) -> Result<Vec<u8>, String> {
        // Construct the URL and cache path
        let url = format!(
            "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/Sound/{}.img",
            img_name
        );
        let cache_path = format!("/01/Sound/{}.img", img_name);

        info!("  → Fetching sound WZ from: {}", url);

        // Fetch and load the WZ file
        let bytes = AssetManager::fetch_and_cache(&url, &cache_path).await?;
        info!("  → WZ file fetched: {} bytes", bytes.len());

        // Guess IV from the WZ file
        let wz_iv = guess_iv_from_wz_img(&bytes)
            .ok_or_else(|| "Unable to guess version from sound file".to_string())?;
        info!("  → IV guessed successfully");

        let byte_len = bytes.len();

        // Create WZ reader
        let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
        info!("  → WZ reader created");

        // Create WZ image
        let cache_name_ref: wz_reader::WzNodeName = cache_path.clone().into();
        let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);

        // Create root node
        let root_node: WzNodeArc = WzNode::new(&cache_path.clone().into(), wz_image, None).into();

        // Parse the root node
        root_node
            .write()
            .unwrap()
            .parse(&root_node)
            .map_err(|e| format!("Failed to parse sound WZ: {:?}", e))?;
        info!("  → WZ parsed successfully");

        // Navigate to the sound track - ensure it's parsed first
        info!("  → Looking for track: {}", track_name);
        let track_node = root_node
            .read()
            .unwrap()
            .at_path_parsed(track_name)
            .map_err(|e| format!("Failed to find sound track '{}': {:?}", track_name, e))?;
        info!("  → Track node found");
        
        // Ensure the node is parsed (same as example pattern)
        track_node
            .write()
            .unwrap()
            .parse(&track_node)
            .map_err(|e| format!("Failed to parse sound node '{}': {:?}", track_name, e))?;
        info!("  → Track node parsed");

        // Extract sound data using the same method as the example
        let sound_data = {
            let node_read = track_node.read().unwrap();
            let sound = node_read
                .try_as_sound()
                .ok_or(format!("Node '{}' is not a sound", track_name))?;
            
            // Log sound type for debugging
            info!("  → Sound type: {:?}, duration: {}ms", sound.sound_type, sound.duration);
            
            // Use get_buffer() which handles WAV headers automatically
            // For WAV: adds 44-byte header + buffer
            // For MP3/Binary: returns raw buffer
            // This matches the pattern from the example (sound.save() internally uses get_buffer())
            let buffer = sound.get_buffer();
            info!("  → Sound buffer extracted: {} bytes (type: {:?})", buffer.len(), sound.sound_type);
            
            buffer
        };
        info!("  → Sound data extracted: {} bytes", sound_data.len());

        Ok(sound_data)
    }

    /// Download MP3 file for debugging (WASM only)
    /// Uses extern "C" function to call JavaScript (same pattern as assets.rs)
    #[cfg(target_arch = "wasm32")]
    async fn download_mp3_for_debug(sound_data: &[u8], track_name: &str) {
        // Use extern "C" function to trigger download via JavaScript
        // This matches the pattern used in assets.rs for console_save
        extern "C" {
            fn download_mp3(
                filename_ptr: *const u8,
                filename_len: u32,
                data_ptr: *const u8,
                data_len: u32,
            );
        }
        
        let filename = format!("{}.mp3", track_name);
        let filename_bytes = filename.as_bytes();
        
        unsafe {
            download_mp3(
                filename_bytes.as_ptr(),
                filename_bytes.len() as u32,
                sound_data.as_ptr(),
                sound_data.len() as u32,
            );
        }
        
        info!("  → MP3 download triggered: {}.mp3 ({} bytes)", track_name, sound_data.len());
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    async fn download_mp3_for_debug(_sound_data: &[u8], _track_name: &str) {
        // No-op for non-WASM builds
    }
}

impl Default for AudioManager {
    fn default() -> Self {
        Self::new()
    }
}
