use macroquad::prelude::*;

const UIWINDOW2_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/01/UI/UIWindow2.img";
const UIWINDOW2_CACHE: &str = "/01/UI/UIWindow2.img";

#[derive(Clone)]
struct TextureWithOrigin {
    texture: Texture2D,
    origin_x: i32,
    origin_y: i32,
}

#[derive(Clone)]
pub struct InventoryWindow {
    pub visible: bool,
}

impl InventoryWindow {
    pub fn new() -> Self {
        Self { visible: false }
    }

    pub async fn load_assets(&mut self) {
        // TODO: Load inventory window assets
    }

    pub fn update(&mut self) {
        // TODO: Update inventory window state
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn draw(&self) {
        if !self.visible {
            return;
        }
        // TODO: Draw inventory window
    }
}

#[derive(Clone)]
pub struct EquipWindow {
    pub visible: bool,
}

impl EquipWindow {
    pub fn new() -> Self {
        Self { visible: false }
    }

    pub async fn load_assets(&mut self) {
        // TODO: Load equip window assets
    }

    pub fn update(&mut self) {
        // TODO: Update equip window state
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn draw(&self) {
        if !self.visible {
            return;
        }
        // TODO: Draw equip window
    }
}

#[derive(Clone)]
pub struct UserInfoWindow {
    pub visible: bool,
}

impl UserInfoWindow {
    pub fn new() -> Self {
        Self { visible: false }
    }

    pub async fn load_assets(&mut self) {
        // TODO: Load user info window assets
    }

    pub fn update(&mut self) {
        // TODO: Update user info window state
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn draw(&self, _name: &str, _level: u32) {
        if !self.visible {
            return;
        }
        // TODO: Draw user info window
    }
}
