#![windows_subsystem = "windows"]
// «Глубина 3D» — подводная выживалка на Rust + macroquad.
// 4 биома, своя флора и фауна, строительство базы, два вида транспорта.
// Цель: найти 4 ядра и запустить Резонатор.

mod game;
mod render;
mod world;

use game::*;
use macroquad::prelude::*;
use render::*;

fn conf() -> Conf {
    Conf {
        window_title: "Глубина 3D".to_owned(),
        window_width: 1280,
        window_height: 720,
        high_dpi: false,
        sample_count: 4,
        ..Default::default()
    }
}

#[macroquad::main(conf)]
async fn main() {
    rand::srand((macroquad::miniquad::date::now() * 1000.0) as u64);
    let font = load_ttf_font_from_bytes(include_bytes!("../assets/font.ttf")).unwrap();
    let gx = Gfx { font };
    let mut g = Game::new();
    let mut released = false;

    loop {
        let dt = get_frame_time().min(0.05);
        let menu_open = g.menu != Menu::None;
        let playing = g.state == State::Play;
        let grab = playing && !menu_open && !released;
        set_cursor_grab(grab);
        show_mouse(!grab);

        if playing {
            // взгляд
            if grab {
                let d = mouse_delta_position();
                g.yaw -= d.x * 1.8;
                g.pitch += d.y * 1.4;
            }
            let ks = 1.8 * dt;
            if is_key_down(KeyCode::Left) {
                g.yaw -= ks;
            }
            if is_key_down(KeyCode::Right) {
                g.yaw += ks;
            }
            if is_key_down(KeyCode::Up) {
                g.pitch += ks;
            }
            if is_key_down(KeyCode::Down) {
                g.pitch -= ks;
            }
            g.pitch = g.pitch.clamp(-1.5, 1.5);
            g.yaw = wrap_angle(g.yaw);

            // клавиши
            if is_key_pressed(KeyCode::Escape) {
                if g.placing.is_some() {
                    g.placing = None;
                } else if menu_open {
                    g.menu = Menu::None;
                } else {
                    released = !released;
                }
            }
            if is_key_pressed(KeyCode::H) {
                g.help = !g.help;
            }
            if is_key_pressed(KeyCode::Tab) {
                g.menu = if g.menu == Menu::Craft { Menu::None } else { Menu::Craft };
                g.placing = None;
            }
            if is_key_pressed(KeyCode::B) {
                if g.placing.is_some() {
                    g.placing = None;
                } else if g.pilot.is_none() {
                    g.menu = if g.menu == Menu::Build { Menu::None } else { Menu::Build };
                }
            }
            if g.menu == Menu::None {
                if is_key_pressed(KeyCode::E) {
                    g.interact();
                }
                if is_key_pressed(KeyCode::Q) {
                    g.sonar();
                }
                if g.placing.is_some() {
                    if is_key_pressed(KeyCode::R) {
                        g.build_rot = !g.build_rot;
                    }
                    if is_key_pressed(KeyCode::F) || (grab && is_mouse_button_pressed(MouseButton::Left)) {
                        g.place();
                    }
                } else if is_key_pressed(KeyCode::X) && g.pilot.is_none() {
                    g.deconstruct();
                }
                if released && is_mouse_button_pressed(MouseButton::Left) {
                    released = false;
                }
            } else {
                let keys = [
                    KeyCode::Key1,
                    KeyCode::Key2,
                    KeyCode::Key3,
                    KeyCode::Key4,
                    KeyCode::Key5,
                    KeyCode::Key6,
                    KeyCode::Key7,
                    KeyCode::Key8,
                    KeyCode::Key9,
                ];
                let mut pick = None;
                for (i, k) in keys.iter().enumerate() {
                    if is_key_pressed(*k) {
                        pick = Some(i);
                    }
                }
                let rows = if g.menu == Menu::Craft { RECIPES.len() } else { MODS.len() };
                if is_mouse_button_pressed(MouseButton::Left) {
                    let (mx, my) = mouse_position();
                    let m = vec2(mx, my);
                    for i in 0..rows {
                        if menu_row(rows, i).contains(m) {
                            pick = Some(i);
                        }
                    }
                    if !menu_rect(rows).contains(m) {
                        g.menu = Menu::None;
                    }
                }
                if let Some(i) = pick {
                    if i < rows {
                        if g.menu == Menu::Craft {
                            g.craft(i);
                        } else {
                            g.placing = Some(MODS[i]);
                            g.menu = Menu::None;
                        }
                    }
                }
            }

            let mut mv = Vec3::ZERO;
            if g.menu == Menu::None {
                if is_key_down(KeyCode::W) {
                    mv.z += 1.0;
                }
                if is_key_down(KeyCode::S) {
                    mv.z -= 1.0;
                }
                if is_key_down(KeyCode::D) {
                    mv.x += 1.0;
                }
                if is_key_down(KeyCode::A) {
                    mv.x -= 1.0;
                }
                if is_key_down(KeyCode::Space) {
                    mv.y += 1.0;
                }
                if is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::C) {
                    mv.y -= 1.0;
                }
            }
            let sprint = is_key_down(KeyCode::LeftShift);
            g.update(dt, mv, sprint);
        } else {
            match g.state {
                State::Dead(_) => {
                    if is_key_pressed(KeyCode::R) || is_key_pressed(KeyCode::Enter) || is_mouse_button_pressed(MouseButton::Left) {
                        g.respawn();
                        released = false;
                    }
                }
                State::Won => {
                    if is_key_pressed(KeyCode::Enter) {
                        g = Game::new();
                    }
                    if is_key_pressed(KeyCode::Escape) {
                        g.state = State::Play;
                    }
                }
                State::Play => {}
            }
        }

        // 3D
        let sh = Shade::new(&g);
        clear_background(sh.fog);
        let cam = Camera3D {
            position: sh.cam,
            target: g.cam_target(),
            up: Vec3::Y,
            fovy: 70f32.to_radians(),
            z_near: 0.1,
            z_far: 700.0,
            ..Default::default()
        };
        set_camera(&cam);
        draw_world(&g, &sh);
        let view = cam.matrix();

        // 2D поверх
        set_default_camera();
        draw_hud(&g, &gx, &view);

        next_frame().await;
    }
}

