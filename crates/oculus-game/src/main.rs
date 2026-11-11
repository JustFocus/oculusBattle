mod assets;
mod effects;
mod particles;
mod render;
mod timing;
mod ui;
mod view;

use assets::Assets;
use effects::Effects;
use macroquad::prelude::*;
use oculus_core::{Game, Phase, Point, TICK_SECONDS};
use render::BoardState;
use timing::{clamp_dt, Stepper};
use view::View;

fn window_conf() -> Conf {
    Conf {
        window_title: "Oculus Battle".to_owned(),
        window_width: 1000,
        window_height: 625,
        high_dpi: true,
        ..Default::default()
    }
}

#[derive(PartialEq, Eq)]
enum Screen {
    Title,
    Playing,
}

#[macroquad::main(window_conf)]
async fn main() {
    let assets = Assets::load();
    next_frame().await; // flush the atlas upgrade before the first real frame
    let mut fx = Effects::new();
    let seed = (macroquad::miniquad::date::now() * 1000.0) as u64;
    let mut game = Game::new(seed);
    // Seed visual randomness (particles) differently on every run as well.
    macroquad::rand::srand(seed ^ 0x9E37_79B9_7F4A_7C15);
    let mut screen = Screen::Title;
    let mut stepper = Stepper::new();

    loop {
        let dt = clamp_dt(get_frame_time());
        let view = View::fit(screen_width(), screen_height(), ui::HUD_HEIGHT);
        let mouse = view.to_world(Vec2::from(mouse_position()));

        if is_mouse_button_pressed(MouseButton::Left) {
            match screen {
                Screen::Title => {
                    screen = Screen::Playing;
                    stepper.reset();
                }
                Screen::Playing => {
                    game.click(Point::new(mouse.x, mouse.y));
                    fx.handle_events(&game.drain_events(), &game);
                }
            }
        }

        if screen == Screen::Playing {
            for _ in 0..stepper.advance(dt, TICK_SECONDS) {
                game.tick();
                fx.handle_events(&game.drain_events(), &game);
            }
            fx.trails(&game, dt);
        }
        fx.update(dt);

        let interactive = screen == Screen::Playing && game.phase() == Phase::Playing;
        let state = BoardState {
            hover: if interactive {
                render::actionable_hover(&game, mouse)
            } else {
                None
            },
            time: get_time(),
        };

        clear_background(render::BAR_COLOR);
        set_camera(&view.camera());
        render::draw_world(&assets, &game, &fx, &view, &state, screen_dpi_scale());
        set_default_camera();
        ui::draw_bars(&view);
        if screen == Screen::Playing {
            ui::draw_base_values(&assets, &game, &fx, &view);
        }
        // The title and intermission screens carry their own text; the HUD would only show a
        // stale level number and empty unit totals there.
        if interactive {
            ui::draw_hud(&assets, &game, &view);
        }
        match screen {
            Screen::Title => ui::draw_title(&assets, &view, state.time),
            Screen::Playing => ui::draw_intermission(&assets, &game, &view, state.time),
        }

        next_frame().await;
    }
}
