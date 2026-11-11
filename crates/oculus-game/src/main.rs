#[allow(dead_code)]
mod particles;

use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Oculus Battle".to_owned(),
        window_width: 800,
        window_height: 500,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let map = Texture2D::from_file_with_format(include_bytes!("../../../assets/map.jpg"), None);
    loop {
        clear_background(BLACK);
        draw_texture_ex(
            &map,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(screen_width(), screen_height())),
                ..Default::default()
            },
        );
        draw_text("scaffold", 20.0, 40.0, 32.0, WHITE);
        next_frame().await;
    }
}
