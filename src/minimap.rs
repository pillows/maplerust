use macroquad::prelude::*;
use crate::assets::AssetManager;
use crate::map::data::MapData;
use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzReader, WzNodeCast};

const UIWINDOW2_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/UI/UIWindow2.img";
const UIWINDOW2_CACHE: &str = "/01/UI/UIWindow2.img";

/// Texture with origin point
struct TextureWithOrigin {
    texture: Texture2D,
    origin: Vec2,
}

/// MiniMap display mode
#[derive(PartialEq, Clone, Copy)]
enum MiniMapMode {
    Normal,   // MinMap - medium size
    Min,      // Min - collapsed/small
    Max,      // MaxMap - large/expanded
}

/// Button state
#[derive(PartialEq, Clone, Copy, Default)]
enum ButtonState {
    #[default]
    Normal,
    MouseOver,
    Pressed,
}

/// Simple button for minimap controls
#[derive(Default)]
struct MiniMapButton {
    normal: Option<Texture2D>,
    mouse_over: Option<Texture2D>,
    pressed: Option<Texture2D>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    state: ButtonState,
}

impl MiniMapButton {
    fn new() -> Self {
        Self {
            normal: None,
            mouse_over: None,
            pressed: None,
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            state: ButtonState::Normal,
        }
    }

    fn update(&mut self) {
        let (mouse_x, mouse_y) = mouse_position();
        let in_bounds = mouse_x >= self.x && mouse_x <= self.x + self.width
            && mouse_y >= self.y && mouse_y <= self.y + self.height;

        if in_bounds {
            if is_mouse_button_down(MouseButton::Left) {
                self.state = ButtonState::Pressed;
            } else {
                self.state = ButtonState::MouseOver;
            }
        } else {
            self.state = ButtonState::Normal;
        }
    }

    fn is_clicked(&self) -> bool {
        let (mouse_x, mouse_y) = mouse_position();
        let in_bounds = mouse_x >= self.x && mouse_x <= self.x + self.width
            && mouse_y >= self.y && mouse_y <= self.y + self.height;
        in_bounds && is_mouse_button_released(MouseButton::Left)
    }

    fn draw(&self) {
        let texture = match self.state {
            ButtonState::MouseOver if self.mouse_over.is_some() => &self.mouse_over,
            ButtonState::Pressed if self.pressed.is_some() => &self.pressed,
            _ => &self.normal,
        };

        if let Some(tex) = texture {
            draw_texture(tex, self.x, self.y, WHITE);
        }
    }
}

/// MiniMap UI component
pub struct MiniMap {
    // Frame textures for MinMap (normal mode)
    minmap_nw: Option<Texture2D>,
    minmap_n: Option<Texture2D>,
    minmap_ne: Option<Texture2D>,
    minmap_w: Option<Texture2D>,
    minmap_c: Option<Texture2D>,
    minmap_e: Option<Texture2D>,
    minmap_sw: Option<Texture2D>,
    minmap_s: Option<Texture2D>,
    minmap_se: Option<Texture2D>,

    // Frame textures for Min (collapsed mode)
    min_w: Option<Texture2D>,
    min_c: Option<Texture2D>,
    min_e: Option<Texture2D>,

    // Buttons
    bt_min: MiniMapButton,
    bt_max: MiniMapButton,
    bt_map: MiniMapButton,

    // State
    mode: MiniMapMode,
    position: Vec2,
    loaded: bool,
    visible: bool,

    // Map canvas from map data
    map_canvas: Option<Texture2D>,
    map_width: i32,
    map_height: i32,
    map_center_x: i32,
    map_center_y: i32,
    map_mag: i32,
}


impl MiniMap {
    pub fn new() -> Self {
        Self {
            minmap_nw: None,
            minmap_n: None,
            minmap_ne: None,
            minmap_w: None,
            minmap_c: None,
            minmap_e: None,
            minmap_sw: None,
            minmap_s: None,
            minmap_se: None,
            min_w: None,
            min_c: None,
            min_e: None,
            bt_min: MiniMapButton::new(),
            bt_max: MiniMapButton::new(),
            bt_map: MiniMapButton::new(),
            mode: MiniMapMode::Normal,
            position: Vec2::new(10.0, 10.0),
            loaded: false,
            visible: true,
            map_canvas: None,
            map_width: 0,
            map_height: 0,
            map_center_x: 0,
            map_center_y: 0,
            map_mag: 1,
        }
    }

    /// Load minimap UI assets from UIWindow2.img
    pub async fn load_assets(&mut self) {
        match Self::load_from_wz().await {
            Ok(data) => {
                self.minmap_nw = data.minmap_nw;
                self.minmap_n = data.minmap_n;
                self.minmap_ne = data.minmap_ne;
                self.minmap_w = data.minmap_w;
                self.minmap_c = data.minmap_c;
                self.minmap_e = data.minmap_e;
                self.minmap_sw = data.minmap_sw;
                self.minmap_s = data.minmap_s;
                self.minmap_se = data.minmap_se;
                self.min_w = data.min_w;
                self.min_c = data.min_c;
                self.min_e = data.min_e;
                self.bt_min = data.bt_min;
                self.bt_max = data.bt_max;
                self.bt_map = data.bt_map;
                self.loaded = true;
                info!("MiniMap UI loaded successfully");
            }
            Err(e) => {
                error!("Failed to load MiniMap UI: {}", e);
                self.loaded = false;
            }
        }
    }

    async fn load_from_wz() -> Result<MiniMapData, String> {
        let bytes = AssetManager::fetch_and_cache(UIWINDOW2_URL, UIWINDOW2_CACHE).await
            .map_err(|e| format!("Failed to fetch UIWindow2.img: {}", e))?;

        let wz_iv = guess_iv_from_wz_img(&bytes)
            .ok_or_else(|| "Unable to guess WZ version from UIWindow2.img".to_string())?;

        let byte_len = bytes.len();
        let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
        let cache_name_ref: wz_reader::WzNodeName = UIWINDOW2_CACHE.to_string().into();
        let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
        let root_node: WzNodeArc = WzNode::new(&UIWINDOW2_CACHE.to_string().into(), wz_image, None).into();

        root_node.write().unwrap().parse(&root_node)
            .map_err(|e| format!("Failed to parse UIWindow2.img: {:?}", e))?;

        let mut data = MiniMapData::default();

        // Load MinMap frame textures
        data.minmap_nw = Self::load_texture(&root_node, "MiniMap/MinMap/nw").await.ok();
        data.minmap_n = Self::load_texture(&root_node, "MiniMap/MinMap/n").await.ok();
        data.minmap_ne = Self::load_texture(&root_node, "MiniMap/MinMap/ne").await.ok();
        data.minmap_w = Self::load_texture(&root_node, "MiniMap/MinMap/w").await.ok();
        data.minmap_c = Self::load_texture(&root_node, "MiniMap/MinMap/c").await.ok();
        data.minmap_e = Self::load_texture(&root_node, "MiniMap/MinMap/e").await.ok();
        data.minmap_sw = Self::load_texture(&root_node, "MiniMap/MinMap/sw").await.ok();
        data.minmap_s = Self::load_texture(&root_node, "MiniMap/MinMap/s").await.ok();
        data.minmap_se = Self::load_texture(&root_node, "MiniMap/MinMap/se").await.ok();

        // Load Min (collapsed) frame textures
        data.min_w = Self::load_texture(&root_node, "MiniMap/Min/w").await.ok();
        data.min_c = Self::load_texture(&root_node, "MiniMap/Min/c").await.ok();
        data.min_e = Self::load_texture(&root_node, "MiniMap/Min/e").await.ok();

        // Load buttons
        data.bt_min = Self::load_button(&root_node, "MiniMap/BtMin").await?;
        data.bt_max = Self::load_button(&root_node, "MiniMap/BtMax").await?;
        data.bt_map = Self::load_button(&root_node, "MiniMap/BtMap").await?;

        Ok(data)
    }

    async fn load_texture(root_node: &WzNodeArc, path: &str) -> Result<Texture2D, String> {
        let node = root_node.read().unwrap().at_path_parsed(path)
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

        Ok(Texture2D::from_rgba8(width, height, &bytes))
    }

    async fn load_button(root_node: &WzNodeArc, path: &str) -> Result<MiniMapButton, String> {
        let mut button = MiniMapButton::new();

        for state in ["normal", "mouseOver", "pressed"] {
            let tex_path = format!("{}/{}/0", path, state);
            if let Ok(texture) = Self::load_texture(root_node, &tex_path).await {
                button.width = texture.width();
                button.height = texture.height();
                match state {
                    "normal" => button.normal = Some(texture),
                    "mouseOver" => button.mouse_over = Some(texture),
                    "pressed" => button.pressed = Some(texture),
                    _ => {}
                }
            }
        }

        Ok(button)
    }

    /// Set the map canvas from map data
    pub fn set_map_data(&mut self, map: &MapData) {
        // Map minimap data is stored in the map's info
        // For now, we'll create a simple representation
        self.map_width = map.get_width();
        self.map_height = map.get_height();
        // TODO: Load actual minimap canvas from map data if available
    }

    /// Update minimap state
    pub fn update(&mut self) {
        if !self.loaded || !self.visible {
            return;
        }

        // Update button positions based on current mode and position
        let frame_width = self.get_frame_width();
        let frame_height = self.get_frame_height();
        
        // Position buttons at top-right of minimap frame
        // BtMin (minimize) and BtMax (maximize/restore) buttons
        self.bt_min.x = self.position.x + frame_width - 28.0;
        self.bt_min.y = self.position.y + 3.0;
        
        self.bt_max.x = self.position.x + frame_width - 14.0;
        self.bt_max.y = self.position.y + 3.0;

        // Update buttons
        self.bt_min.update();
        self.bt_max.update();
        self.bt_map.update();

        // Handle button clicks
        if self.bt_min.is_clicked() {
            // Minimize button: toggle between Min and Normal
            self.mode = match self.mode {
                MiniMapMode::Min => MiniMapMode::Normal,
                _ => MiniMapMode::Min,
            };
        }
        if self.bt_max.is_clicked() {
            // Maximize button: toggle between Normal and Max
            self.mode = match self.mode {
                MiniMapMode::Max => MiniMapMode::Normal,
                MiniMapMode::Normal => MiniMapMode::Max,
                MiniMapMode::Min => MiniMapMode::Normal,
            };
        }

        // Toggle visibility with M key
        if is_key_pressed(KeyCode::M) {
            self.visible = !self.visible;
        }
    }

    fn get_frame_width(&self) -> f32 {
        match self.mode {
            MiniMapMode::Min => 150.0,
            MiniMapMode::Normal => 180.0,
            MiniMapMode::Max => 250.0,
        }
    }

    fn get_frame_height(&self) -> f32 {
        match self.mode {
            MiniMapMode::Min => 18.0,
            MiniMapMode::Normal => 120.0,
            MiniMapMode::Max => 180.0,
        }
    }

    /// Draw the minimap
    pub fn draw(&self, player_x: f32, player_y: f32, map: &MapData) {
        if !self.loaded || !self.visible {
            return;
        }

        let x = self.position.x;
        let y = self.position.y;
        let width = self.get_frame_width();
        let height = self.get_frame_height();

        match self.mode {
            MiniMapMode::Min => self.draw_min_mode(x, y, width, map),
            MiniMapMode::Normal | MiniMapMode::Max => self.draw_normal_mode(x, y, width, height, player_x, player_y, map),
        }
    }

    fn draw_min_mode(&self, x: f32, y: f32, width: f32, map: &MapData) {
        // Draw collapsed minimap (just a title bar)
        let bar_height = 18.0;
        
        // Draw background bar
        draw_rectangle(x, y, width, bar_height, Color::from_rgba(0, 0, 0, 180));
        draw_rectangle_lines(x, y, width, bar_height, 1.0, Color::from_rgba(100, 100, 100, 200));

        // Draw map name
        let map_name = if !map.info.map_name.is_empty() {
            &map.info.map_name
        } else {
            "Unknown"
        };
        draw_text(map_name, x + 5.0, y + 13.0, 12.0, WHITE);

        // Draw buttons
        self.bt_min.draw();
        self.bt_max.draw();
    }

    fn draw_normal_mode(&self, x: f32, y: f32, width: f32, height: f32, player_x: f32, player_y: f32, map: &MapData) {
        // Draw semi-transparent background
        draw_rectangle(x, y, width, height, Color::from_rgba(0, 0, 0, 180));
        draw_rectangle_lines(x, y, width, height, 1.0, Color::from_rgba(100, 100, 100, 200));

        // Draw title bar
        let title_height = 18.0;
        draw_rectangle(x, y, width, title_height, Color::from_rgba(40, 40, 60, 220));
        draw_line(x, y + title_height, x + width, y + title_height, 1.0, Color::from_rgba(100, 100, 100, 200));

        // Draw map name at top
        let map_name = if !map.info.map_name.is_empty() {
            &map.info.map_name
        } else {
            "Unknown"
        };
        draw_text(map_name, x + 5.0, y + 13.0, 11.0, WHITE);

        // Draw map content area
        let content_x = x + 5.0;
        let content_y = y + title_height + 5.0;
        let content_width = width - 10.0;
        let content_height = height - title_height - 10.0;

        // Draw simplified map representation
        self.draw_map_content(content_x, content_y, content_width, content_height, player_x, player_y, map);

        // Draw buttons
        self.bt_min.draw();
        self.bt_max.draw();
    }

    fn draw_map_content(&self, x: f32, y: f32, width: f32, height: f32, player_x: f32, player_y: f32, map: &MapData) {
        // Calculate scale to fit map in minimap area
        let map_width = map.get_width() as f32;
        let map_height = map.get_height() as f32;
        
        if map_width <= 0.0 || map_height <= 0.0 {
            return;
        }

        let scale_x = width / map_width;
        let scale_y = height / map_height;
        let scale = scale_x.min(scale_y);

        let map_left = map.info.vr_left as f32;
        let map_top = map.info.vr_top as f32;

        // Draw footholds as lines
        for fh in &map.footholds {
            let x1 = x + (fh.x1 as f32 - map_left) * scale;
            let y1 = y + (fh.y1 as f32 - map_top) * scale;
            let x2 = x + (fh.x2 as f32 - map_left) * scale;
            let y2 = y + (fh.y2 as f32 - map_top) * scale;
            
            draw_line(x1, y1, x2, y2, 1.0, Color::from_rgba(100, 100, 100, 200));
        }

        // Draw portals as small circles
        for portal in &map.portals {
            if portal.pt == 2 || portal.pt == 7 { // Regular portals
                let px = x + (portal.x as f32 - map_left) * scale;
                let py = y + (portal.y as f32 - map_top) * scale;
                draw_circle(px, py, 3.0, Color::from_rgba(0, 200, 255, 200));
            }
        }

        // Draw NPCs as small dots
        for life in &map.life {
            if life.life_type == "n" && !life.hide {
                let lx = x + (life.x as f32 - map_left) * scale;
                let ly = y + (life.y as f32 - map_top) * scale;
                draw_circle(lx, ly, 2.0, Color::from_rgba(255, 255, 0, 200));
            }
        }

        // Draw player position
        let player_mini_x = x + (player_x - map_left) * scale;
        let player_mini_y = y + (player_y - map_top) * scale;
        
        // Player marker (small triangle or dot)
        draw_circle(player_mini_x, player_mini_y, 3.0, Color::from_rgba(255, 100, 100, 255));
        draw_circle_lines(player_mini_x, player_mini_y, 4.0, 1.0, WHITE);
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }
}

impl Default for MiniMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Temporary structure for loading minimap data
#[derive(Default)]
struct MiniMapData {
    minmap_nw: Option<Texture2D>,
    minmap_n: Option<Texture2D>,
    minmap_ne: Option<Texture2D>,
    minmap_w: Option<Texture2D>,
    minmap_c: Option<Texture2D>,
    minmap_e: Option<Texture2D>,
    minmap_sw: Option<Texture2D>,
    minmap_s: Option<Texture2D>,
    minmap_se: Option<Texture2D>,
    min_w: Option<Texture2D>,
    min_c: Option<Texture2D>,
    min_e: Option<Texture2D>,
    bt_min: MiniMapButton,
    bt_max: MiniMapButton,
    bt_map: MiniMapButton,
}
