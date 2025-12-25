mod assets;
mod game;
mod logo;
mod login;

#[macroquad::main("RustMaple")]
async fn main() {
    game::run().await;
}
