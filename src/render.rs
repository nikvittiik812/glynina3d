// Отрисовка: свет и туман на CPU, мир, существа, база, транспорт, HUD.

use crate::game::*;
use crate::world::*;
use macroquad::prelude::*;

pub struct Gfx {
    pub font: Font,
}

// ---------- свет и туман ----------
pub struct Shade {
    pub cam: Vec3,
    pub fwd: Vec3,
    pub fog: Color,
    pub density: f32,
    pub lamp_r: f32,
    pub lights: Vec<(Vec3, f32, Color)>,
    pub above: bool,
}

pub fn water_fog(y: f32) -> Color {
    let d = (-y).max(0.0);
    let a = Color::new(0.13, 0.52, 0.64, 1.0);
    let b = Color::new(0.03, 0.2, 0.32, 1.0);
    let c = Color::new(0.004, 0.018, 0.035, 1.0);
    if d < 90.0 {
        mix(a, b, d / 90.0)
    } else {
        mix(b, c, ((d - 90.0) / 110.0).min(1.0))
    }
}

impl Shade {
    pub fn new(g: &Game) -> Shade {
        let cam = g.eye();
        let fwd = (g.cam_target() - cam).normalize_or_zero();
        let above = cam.y > 0.0;
        let mut lights = Vec::new();
        for m in &g.modules {
            match m.kind {
                ModKind::Lamp => lights.push((m.pos + vec3(0.0, 2.0, 0.0), 30.0, Color::new(1.0, 0.95, 0.8, 1.0))),
                ModKind::Room | ModKind::Dome | ModKind::Corridor if g.powered(m.pos) => {
                    lights.push((m.pos, 12.0, Color::new(0.8, 0.9, 1.0, 1.0)))
                }
                ModKind::Generator => lights.push((m.pos, 10.0, Color::new(1.0, 0.85, 0.4, 1.0))),
                ModKind::Resonator => lights.push((m.pos, 40.0, Color::new(1.0, 0.5, 0.8, 1.0))),
                _ => {}
            }
        }
        for v in &g.vehicles {
            let r = if v.kind == VehKind::Kit { 45.0 } else { 28.0 };
            lights.push((v.pos + v.fwd() * r * 0.4, r, Color::new(1.0, 1.0, 0.9, 1.0)));
        }
        for c in &g.world.cores {
            if !c.taken {
                lights.push((c.pos, 14.0, Color::new(1.0, 0.4, 0.6, 1.0)));
            }
        }
        // светящиеся растения рядом
        for f in &g.world.flora {
            let r = match f.kind {
                FloraKind::Stalk => 10.0,
                FloraKind::Spike => 8.0,
                FloraKind::Vent => 12.0,
                _ => 0.0,
            };
            if r > 0.0 && f.pos.distance(cam) < 90.0 && lights.len() < 60 {
                let col = match f.kind {
                    FloraKind::Stalk => Color::new(0.8, 1.0, 0.5, 1.0),
                    FloraKind::Spike => f.color,
                    _ => Color::new(1.0, 0.5, 0.2, 1.0),
                };
                let lp = if f.kind == FloraKind::Stalk { f.pos + vec3(0.0, 22.0 * f.scale, 0.0) } else { f.pos };
                lights.push((lp, r * f.scale.max(1.0), col));
            }
        }
        let (fog, density) = if above {
            (Color::new(0.68, 0.84, 0.95, 1.0), 0.004)
        } else {
            (water_fog(cam.y), 0.017 + (-cam.y).max(0.0) * 0.00006)
        };
        let lamp_r = if g.pilot.is_some() { 0.0 } else { 24.0 * g.lamp_mul };
        Shade { cam, fwd, fog, density, lamp_r, lights, above }
    }

    pub fn view_dist(&self) -> f32 {
        (4.6 / self.density).min(330.0)
    }

    pub fn col(&self, c: Color, p: Vec3, emissive: f32) -> Color {
        let d = p - self.cam;
        let dist = d.length();
        let amb = if p.y > 0.0 { 1.0 } else { 0.05 + 0.95 * (p.y / 55.0).exp() };
        let mut lr = amb;
        let mut lg = amb;
        let mut lb = amb;
        if self.lamp_r > 0.0 && dist < self.lamp_r {
            let cone = (d / dist.max(0.01)).dot(self.fwd).max(0.0).powi(3);
            let k = (1.0 - dist / self.lamp_r).powf(1.2) * 1.5 * cone + (1.0 - dist / 6.0).max(0.0) * 0.5;
            lr += k;
            lg += k;
            lb += k * 0.9;
        }
        for (lp, r, lc) in &self.lights {
            let ld = lp.distance(p);
            if ld < *r {
                let k = (1.0 - ld / r).powi(2) * 0.9;
                lr += k * lc.r;
                lg += k * lc.g;
                lb += k * lc.b;
            }
        }
        let e = emissive;
        let lit = Color::new(
            c.r * lr.min(1.3).max(e),
            c.g * lg.min(1.3).max(e),
            c.b * lb.min(1.3).max(e),
            c.a,
        );
        let f = (1.0 - (-dist * self.density).exp()) * (1.0 - e * 0.45);
        let mut o = mix(lit, self.fog, f);
        o.a = c.a;
        o
    }
}

// ---------- примитивы ----------
fn cube(p: Vec3, s: Vec3, c: Color) {
    draw_cube(p, s, None, c);
}

fn sph(p: Vec3, r: f32, c: Color, detail: usize) {
    draw_sphere_ex(p, r, None, c, DrawSphereParams { rings: detail, slices: detail, draw_mode: DrawMode::Triangles });
}

fn cyl(p: Vec3, rt: f32, rb: f32, h: f32, c: Color) {
    draw_cylinder_ex(p, rt, rb, h, None, c, DrawCylinderParams { sides: 8, draw_mode: DrawMode::Triangles });
}

fn push(m: Mat4) {
    unsafe { get_internal_gl() }.quad_gl.push_model_matrix(m);
}

fn pop() {
    unsafe { get_internal_gl() }.quad_gl.pop_model_matrix();
}

fn tf(pos: Vec3, yaw: f32, pitch: f32, scale: Vec3) -> Mat4 {
    Mat4::from_translation(pos)
        * Mat4::from_rotation_y(-yaw)
        * Mat4::from_rotation_z(pitch)
        * Mat4::from_scale(scale)
}

// ---------- мир ----------
pub fn draw_world(g: &Game, sh: &Shade) {
    let t = g.time;
    let vd = sh.view_dist();
    let cam = sh.cam;

    // рельеф кусками (лимит индексов на один вызов)
    let chunks = GRID / CHUNK;
    for cj in 0..chunks {
        for ci in 0..chunks {
            let cx = -SPAN / 2.0 + (ci as f32 + 0.5) * CHUNK as f32 * CELL;
            let cz = -SPAN / 2.0 + (cj as f32 + 0.5) * CHUNK as f32 * CELL;
            let half = CHUNK as f32 * CELL * 0.72;
            let to = vec2(cx - cam.x, cz - cam.z);
            if to.length() > vd + half {
                continue;
            }
            // грубое отсечение того, что за спиной
            let f2 = vec2(sh.fwd.x, sh.fwd.z).normalize_or_zero();
            if to.length() > half * 1.5 && to.normalize().dot(f2) < -0.45 && sh.fwd.y.abs() < 0.8 {
                continue;
            }
            let n = CHUNK + 1;
            let mut verts = Vec::with_capacity(n * n);
            for j in 0..n {
                for i in 0..n {
                    let (p, k) = g.world.vertex(ci * CHUNK + i, cj * CHUNK + j);
                    let base = g.world.base_col[k];
                    let rl = g.world.relief[k];
                    let c = sh.col(Color::new(base.r * rl, base.g * rl, base.b * rl, 1.0), p, 0.0);
                    verts.push(Vertex::new2(p, vec2(0.0, 0.0), c));
                }
            }
            let mut idx: Vec<u16> = Vec::with_capacity(CHUNK * CHUNK * 6);
            for j in 0..CHUNK {
                for i in 0..CHUNK {
                    let a = (j * n + i) as u16;
                    let b = a + 1;
                    let c = a + n as u16;
                    let d = c + 1;
                    idx.extend_from_slice(&[a, c, b, b, c, d]);
                }
            }
            draw_mesh(&Mesh { vertices: verts, indices: idx, texture: None });
        }
    }

    // плот на поверхности — точка старта
    let rp = g.raft;
    let wood = sh.col(Color::new(0.55, 0.38, 0.22, 1.0), rp, 0.0);
    cube(rp + vec3(0.0, 0.15, 0.0), vec3(4.5, 0.5, 4.5), wood);
    cube(rp + vec3(1.5, 1.6, 1.5), vec3(0.15, 3.0, 0.15), wood);
    cube(rp + vec3(1.9, 2.7, 1.5), vec3(0.8, 0.5, 0.05), sh.col(Color::new(1.0, 0.35, 0.2, 1.0), rp, 0.6));
    for k in 0..4 {
        let a = k as f32 * 1.57;
        cube(rp + vec3(a.cos() * 2.4, -0.05, a.sin() * 2.4), vec3(0.9, 0.7, 0.9), sh.col(Color::new(0.9, 0.5, 0.15, 1.0), rp, 0.0));
    }

    // флора
    for f in &g.world.flora {
        let dist = f.pos.distance(cam);
        if dist > vd {
            continue;
        }
        let s = f.scale;
        let p = f.pos;
        let sway = (t * 0.8 + f.phase).sin();
        match f.kind {
            FloraKind::Fan => {
                let c = sh.col(f.color, p, 0.0);
                if f.phase > 3.14 {
                    cube(p + vec3(0.0, 1.3 * s, 0.0), vec3(2.6 * s, 2.2 * s, 0.12), c);
                } else {
                    cube(p + vec3(0.0, 1.3 * s, 0.0), vec3(0.12, 2.2 * s, 2.6 * s), c);
                }
                cube(p + vec3(0.0, 0.2, 0.0), vec3(0.4, 0.5, 0.4), sh.col(Color::new(0.4, 0.3, 0.3, 1.0), p, 0.0));
            }
            FloraKind::Tube => {
                for k in 0..4 {
                    let a = f.phase + k as f32 * 1.6;
                    let h = (1.2 + (k as f32 * 0.7 + f.phase).sin().abs() * 1.8) * s;
                    let q = p + vec3(a.cos() * 0.6 * s, 0.0, a.sin() * 0.6 * s);
                    cyl(q, 0.3 * s, 0.25 * s, h, sh.col(f.color, q, 0.0));
                    if dist < 50.0 {
                        sph(q + vec3(0.0, h, 0.0), 0.22 * s, sh.col(Color::new(1.0, 0.4, 0.3, 1.0), q, 0.3), 5);
                    }
                }
            }
            FloraKind::Grass => {
                if dist > vd * 0.6 {
                    continue;
                }
                let c = sh.col(f.color, p, 0.0);
                for k in 0..3 {
                    let off = vec3((k as f32 - 1.0) * 0.4, 0.0, (f.phase + k as f32).sin() * 0.4);
                    let mut q = p + off;
                    for seg in 0..3 {
                        let lean = vec3(sway * 0.25 * seg as f32, 0.0, 0.0);
                        cube(q + vec3(0.0, 0.5 * s, 0.0) + lean, vec3(0.1, 1.0 * s, 0.25), c);
                        q.y += 1.0 * s;
                    }
                }
            }
            FloraKind::Stalk => {
                let segs = 6;
                let seg_h = 4.0 * s;
                let mut q = p;
                let stem = sh.col(f.color, p + vec3(0.0, 10.0, 0.0), 0.0);
                for k in 0..segs {
                    let kk = k as f32 / segs as f32;
                    let next = p + vec3(sway * 2.0 * kk * kk, seg_h * (k + 1) as f32, (t * 0.6 + f.phase).cos() * 1.5 * kk * kk);
                    let r = 0.55 * s * (1.0 - kk * 0.5);
                    cyl(q, r * 0.85, r, seg_h + 0.1, stem);
                    if k % 2 == 1 && dist < 120.0 {
                        let side = vec3((f.phase + k as f32).cos(), 0.0, (f.phase + k as f32).sin()) * r * 2.0;
                        sph(next + side, 0.35 * s, sh.col(Color::new(0.85, 1.0, 0.45, 1.0), next, 0.9), 5);
                    }
                    q = next;
                }
                let pulse = 0.85 + (t * 1.5 + f.phase).sin() * 0.15;
                sph(q, 1.2 * s * pulse, sh.col(Color::new(0.85, 1.0, 0.5, 1.0), q, 1.0), 7);
            }
            FloraKind::Spike => {
                let pulse = 0.65 + (t * 2.2 + f.phase).sin() * 0.25;
                let c = sh.col(f.color, p, pulse);
                cyl(p - vec3(0.0, 0.5, 0.0), 0.0, 0.9 * s, 4.5 * s, c);
                if dist < 110.0 {
                    cyl(p + vec3(0.9 * s, -0.4, 0.3 * s), 0.0, 0.5 * s, 2.6 * s, c);
                    cyl(p + vec3(-0.6 * s, -0.4, -0.7 * s), 0.0, 0.45 * s, 2.0 * s, c);
                }
            }
            FloraKind::Vent => {
                cyl(p - vec3(0.0, 0.3, 0.0), 1.1 * s, 2.0 * s, 2.7 * s, sh.col(f.color, p, 0.0));
                let pulse = 0.8 + (t * 3.0 + f.phase).sin() * 0.2;
                sph(p + vec3(0.0, 2.4 * s, 0.0), 0.9 * s * pulse, sh.col(Color::new(1.0, 0.45, 0.15, 1.0), p, 1.0), 6);
            }
            FloraKind::Spire => {
                cyl(p - vec3(0.0, 0.5, 0.0), 0.15 * s, 1.6 * s, 12.0 * s, sh.col(f.color, p + vec3(0.0, 6.0 * s, 0.0), 0.0));
            }
        }
    }

    // ресурсы
    for n in &g.world.nodes {
        if !n.alive {
            continue;
        }
        let dist = n.pos.distance(cam);
        if dist > vd * 0.8 {
            continue;
        }
        let e = if n.kind.glows() { 0.75 + (t * 2.0 + n.pos.x).sin() * 0.2 } else { 0.0 };
        let rock = sh.col(Color::new(0.35, 0.33, 0.36, 1.0), n.pos, 0.0);
        sph(n.pos, 0.75, rock, 6);
        let c = sh.col(n.kind.color(), n.pos, e);
        match n.kind {
            Res::Quartz | Res::Crystal => {
                cyl(n.pos + vec3(0.0, 0.2, 0.0), 0.0, 0.3, 1.4, c);
                cyl(n.pos + vec3(0.35, 0.1, 0.2), 0.0, 0.2, 0.9, c);
            }
            _ => {
                sph(n.pos + vec3(0.35, 0.35, 0.2), 0.4, c, 5);
                sph(n.pos + vec3(-0.3, 0.25, -0.25), 0.3, c, 5);
            }
        }
    }

    // ядра
    for c in &g.world.cores {
        if c.taken || c.pos.distance(cam) > vd {
            continue;
        }
        let p = c.pos;
        let pulse = 1.0 + (t * 3.0).sin() * 0.12;
        cyl(p - vec3(0.0, 2.0, 0.0), 1.2, 1.8, 1.2, sh.col(Color::new(0.3, 0.3, 0.35, 1.0), p, 0.0));
        push(Mat4::from_translation(p) * Mat4::from_rotation_y(t) * Mat4::from_rotation_x(0.6));
        cube(Vec3::ZERO, Vec3::splat(0.9 * pulse), sh.col(Color::new(1.0, 0.35, 0.55, 1.0), p, 1.0));
        pop();
        sph(p, 0.4, sh.col(Color::new(1.0, 0.9, 0.95, 1.0), p, 1.0), 6);
    }

    draw_creatures(g, sh);
    draw_base(g, sh, false);
    draw_vehicles(g, sh);

    // пузыри
    for p in &g.parts {
        let pd = p.pos.distance(cam);
        if pd < 60.0 && pd > 1.2 {
            sph(p.pos, p.size, sh.col(Color::new(0.85, 0.95, 1.0, 0.55), p.pos, 0.5), 4);
        }
    }

    // поверхность воды
    let surf_c = if sh.above {
        Color::new(0.1, 0.42, 0.55, 1.0)
    } else {
        let k = (-(cam.y.abs() * 1.5 + 10.0) * sh.density).exp();
        mix(sh.fog, Color::new(0.6, 0.88, 0.98, 1.0), k)
    };
    draw_plane(vec3(cam.x, 0.0, cam.z), vec2(400.0, 400.0), None, surf_c);

    // прозрачное — последним
    draw_base(g, sh, true);
    draw_ghost(g, sh);
}

fn draw_creatures(g: &Game, sh: &Shade) {
    let t = g.time;
    let vd = sh.view_dist();
    for c in &g.creatures {
        let dist = c.pos.distance(sh.cam);
        if dist > vd {
            continue;
        }
        let v = c.vel;
        let yaw = v.z.atan2(v.x);
        let pitch = (v.y / v.length().max(0.01)).clamp(-0.6, 0.6).asin();
        match c.kind {
            CrKind::Fish => {
                if dist > vd * 0.7 {
                    continue;
                }
                let col = sh.col(c.color, c.pos, 0.0);
                push(tf(c.pos, yaw, pitch, Vec3::splat(c.size)));
                sph(Vec3::ZERO, 1.0, col, 6);
                let wag = (t * 12.0 + c.t).sin() * 0.4;
                cube(vec3(-1.4, 0.0, wag), vec3(0.9, 1.2, 0.15), col);
                cube(vec3(0.6, 0.35, 0.45), Vec3::splat(0.25), sh.col(BLACK, c.pos, 0.0));
                pop();
            }
            CrKind::Drifter => {
                // парусник: плоское тело, высокий парус-плавник, длинные светящиеся щупальца
                let col = sh.col(c.color, c.pos, 0.15);
                let glow = sh.col(Color::new(0.5, 0.75, 1.0, 1.0), c.pos, 0.95);
                push(tf(c.pos, yaw, 0.0, Vec3::ONE));
                push(Mat4::from_scale(vec3(4.5, 1.6, 2.6)));
                sph(Vec3::ZERO, 1.0, col, 9);
                pop();
                let flap = (t * 0.9 + c.t).sin() * 0.5;
                cube(vec3(0.0, 2.6, 0.0), vec3(5.0, 3.5, 0.15), sh.col(Color::new(0.8, 0.6, 0.95, 0.9), c.pos, 0.2));
                cube(vec3(0.0, -0.3 + flap, 3.0), vec3(3.5, 0.15, 3.0), col);
                cube(vec3(0.0, -0.3 - flap, -3.0), vec3(3.5, 0.15, 3.0), col);
                for k in 0..5 {
                    sph(vec3(3.0 - k as f32 * 1.4, 0.6, 1.9), 0.25, glow, 4);
                    sph(vec3(3.0 - k as f32 * 1.4, 0.6, -1.9), 0.25, glow, 4);
                }
                pop();
                for k in 0..4 {
                    let base = c.pos + vec3((k as f32 - 1.5) * 1.2, -1.3, 0.0);
                    let sw = (t + k as f32).sin() * 0.6;
                    cube(base + vec3(sw, -2.5, 0.0), vec3(0.12, 5.0, 0.12), glow);
                }
            }
            CrKind::Clawback => {
                // клешнеспин: панцирь с кристаллическими шипами, две клешни
                let col = sh.col(c.color, c.pos, 0.0);
                let crys = sh.col(Color::new(0.5, 0.9, 1.0, 1.0), c.pos, 0.7);
                push(tf(c.pos, yaw, pitch * 0.5, Vec3::splat(1.3)));
                push(Mat4::from_scale(vec3(1.6, 0.8, 1.1)));
                sph(Vec3::ZERO, 1.0, col, 8);
                pop();
                for k in 0..3 {
                    cyl(vec3(-0.6 + k as f32 * 0.6, 0.5, 0.0), 0.0, 0.25, 1.0, crys);
                }
                let snap = (t * 6.0 + c.t).sin().abs() * 0.3;
                for s in [-1.0f32, 1.0] {
                    cube(vec3(1.6, -0.1, 0.8 * s), vec3(1.2, 0.3, 0.3), col);
                    cube(vec3(2.3, 0.1 + snap, 0.8 * s), vec3(0.8, 0.2, 0.45), crys);
                    cube(vec3(2.3, -0.3 - snap, 0.8 * s), vec3(0.8, 0.2, 0.45), crys);
                    for l in 0..3 {
                        let kick = (t * 10.0 + l as f32 + c.t).sin() * 0.2;
                        cube(vec3(0.6 - l as f32 * 0.6 + kick, -0.6, 1.0 * s), vec3(0.15, 0.8, 0.15), col);
                    }
                }
                cube(vec3(1.4, 0.4, 0.3), Vec3::splat(0.2), sh.col(Color::new(1.0, 1.0, 0.4, 1.0), c.pos, 1.0));
                cube(vec3(1.4, 0.4, -0.3), Vec3::splat(0.2), sh.col(Color::new(1.0, 1.0, 0.4, 1.0), c.pos, 1.0));
                pop();
            }
            CrKind::Gloomfang => {
                let n = c.segs.len();
                for (i, s) in c.segs.iter().enumerate().rev() {
                    let k = 1.0 - i as f32 / n as f32;
                    let r = 0.4 + 1.3 * k;
                    sph(*s, r, sh.col(c.color, *s, 0.0), 7);
                    if i % 3 == 1 {
                        let fin = sh.col(Color::new(0.45, 0.2, 0.5, 1.0), *s, 0.2);
                        cyl(*s + vec3(0.0, r * 0.6, 0.0), 0.0, 0.35, 1.2 + k, fin);
                    }
                    if i % 4 == 2 {
                        sph(*s + vec3(0.0, -r * 0.7, 0.0), 0.15, sh.col(Color::new(0.4, 0.6, 1.0, 1.0), *s, 1.0), 4);
                    }
                }
                let open = 0.25 + (t * 7.0).sin().abs() * 0.35;
                push(tf(c.pos, yaw, pitch, Vec3::ONE));
                let jaw = sh.col(Color::new(0.3, 0.15, 0.35, 1.0), c.pos, 0.0);
                push(Mat4::from_rotation_z(open));
                cube(vec3(1.6, 0.2, 0.0), vec3(2.2, 0.35, 1.4), jaw);
                pop();
                push(Mat4::from_rotation_z(-open));
                cube(vec3(1.6, -0.2, 0.0), vec3(2.2, 0.35, 1.4), jaw);
                pop();
                let eye = sh.col(Color::new(1.0, 0.15, 0.1, 1.0), c.pos, 1.0);
                sph(vec3(0.9, 0.7, 0.6), 0.22, eye, 5);
                sph(vec3(0.9, 0.7, -0.6), 0.22, eye, 5);
                pop();
            }
        }
    }
}

fn draw_module(kind: ModKind, pos: Vec3, rot: bool, powered: bool, sh: &Shade, t: f32, glass: bool, ghost: Option<Color>) {
    let s = kind.size(rot);
    let shell = |c: Color| match ghost {
        Some(g) => g,
        None => sh.col(c, pos, 0.0),
    };
    let glow = |c: Color, e: f32| match ghost {
        Some(g) => g,
        None => sh.col(c, pos, if powered { e } else { 0.1 }),
    };
    let hull = Color::new(0.78, 0.8, 0.82, 1.0);
    let dark = Color::new(0.35, 0.38, 0.42, 1.0);
    let win = Color::new(0.5, 0.8, 1.0, 1.0);
    if glass {
        if kind == ModKind::Dome {
            let c = match ghost {
                Some(g) => g,
                None => {
                    let mut c = sh.col(Color::new(0.6, 0.85, 1.0, 1.0), pos, if powered { 0.3 } else { 0.0 });
                    c.a = 0.3;
                    c
                }
            };
            sph(pos, 4.0, c, 14);
        }
        return;
    }
    match kind {
        ModKind::Foundation => {
            cube(pos, s, shell(dark));
            cube(pos + vec3(0.0, 0.52, 0.0), vec3(s.x - 1.0, 0.05, s.z - 1.0), shell(hull));
        }
        ModKind::Corridor => {
            cube(pos, s, shell(hull));
            let along = if rot { vec3(0.0, 0.0, 1.0) } else { vec3(1.0, 0.0, 0.0) };
            let side = vec3(along.z, 0.0, along.x);
            for sgn in [-1.0f32, 1.0] {
                let ws = along * (s.x.max(s.z) - 1.5) + side * 0.05 + vec3(0.0, 0.8, 0.0);
                cube(pos + side * sgn * (s.x.min(s.z) / 2.0), ws, glow(win, 0.7));
            }
        }
        ModKind::Room => {
            cube(pos, s, shell(hull));
            cube(pos + vec3(0.0, s.y / 2.0, 0.0), vec3(s.x - 1.5, 0.3, s.z - 1.5), shell(dark));
            for (a, b) in [(1.0f32, 0.0f32), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                let at = pos + vec3(a * s.x / 2.0, 0.6, b * s.z / 2.0);
                let sz = vec3(if a != 0.0 { 0.1 } else { s.x - 2.5 }, 1.4, if b != 0.0 { 0.1 } else { s.z - 2.5 });
                cube(at, sz, glow(win, 0.7));
            }
        }
        ModKind::Dome => {
            cube(pos - vec3(0.0, 3.7, 0.0), vec3(8.4, 0.6, 8.4), shell(dark));
        }
        ModKind::Generator => {
            cyl(pos - vec3(0.0, 1.5, 0.0), 0.8, 1.0, 2.2, shell(Color::new(0.85, 0.7, 0.25, 1.0)));
            let pulse = 0.8 + (t * 4.0).sin() * 0.2;
            sph(pos + vec3(0.0, 1.0, 0.0), 0.5 * pulse, glow(Color::new(1.0, 0.85, 0.3, 1.0), 1.0), 6);
        }
        ModKind::Lamp => {
            cube(pos, vec3(0.3, s.y, 0.3), shell(dark));
            sph(pos + vec3(0.0, s.y / 2.0, 0.0), 0.45, glow(Color::new(1.0, 0.95, 0.8, 1.0), 1.0), 6);
        }
        ModKind::Pillar => {
            cube(pos, s, shell(hull));
            cube(pos - vec3(0.0, s.y / 2.0 - 0.3, 0.0), vec3(1.8, 0.6, 1.8), shell(dark));
        }
        ModKind::Shipyard => {
            cube(pos, s, shell(Color::new(0.85, 0.7, 0.15, 1.0)));
            cube(pos + vec3(0.0, 0.52, 0.0), vec3(s.x - 2.0, 0.05, s.z - 2.0), shell(Color::new(0.15, 0.15, 0.17, 1.0)));
            for (a, b) in [(1.0f32, 1.0f32), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
                cube(pos + vec3(a * 5.5, 3.5, b * 5.5), vec3(0.6, 6.0, 0.6), shell(dark));
            }
            cube(pos + vec3(0.0, 6.5, 5.5), vec3(11.6, 0.5, 0.5), shell(dark));
            cube(pos + vec3(0.0, 6.5, -5.5), vec3(11.6, 0.5, 0.5), shell(dark));
            let arm = (t * 0.8).sin() * 3.0;
            cube(pos + vec3(arm, 6.0, 0.0), vec3(0.6, 0.6, 11.6), glow(Color::new(0.4, 0.9, 1.0, 1.0), 0.8));
        }
        ModKind::Resonator => {
            cyl(pos - vec3(0.0, s.y / 2.0, 0.0), 1.5, 2.5, 2.0, shell(dark));
            cyl(pos - vec3(0.0, s.y / 2.0 - 2.0, 0.0), 0.2, 0.9, 7.5, shell(hull));
            for k in 0..4 {
                let a = t * 0.7 + k as f32 * 1.5708;
                let q = pos + vec3(a.cos() * 2.0, -1.0 + (t * 2.0 + k as f32).sin() * 0.5, a.sin() * 2.0);
                sph(q, 0.5, glow(Color::new(1.0, 0.35, 0.6, 1.0), 1.0), 6);
            }
            sph(pos + vec3(0.0, s.y / 2.0, 0.0), 0.9, glow(Color::new(1.0, 0.8, 0.95, 1.0), 1.0), 8);
        }
    }
}

fn draw_base(g: &Game, sh: &Shade, glass: bool) {
    let vd = sh.view_dist();
    for m in &g.modules {
        if m.pos.distance(sh.cam) > vd + 10.0 {
            continue;
        }
        let powered = g.powered(m.pos);
        draw_module(m.kind, m.pos, m.rot, powered, sh, g.time, glass, None);
    }
}

fn draw_ghost(g: &Game, sh: &Shade) {
    if let Some((kind, pos, size, ok)) = g.ghost() {
        let c = if ok { Color::new(0.3, 1.0, 0.5, 0.35) } else { Color::new(1.0, 0.25, 0.25, 0.35) };
        draw_module(kind, pos, g.build_rot, false, sh, g.time, false, Some(c));
        draw_module(kind, pos, g.build_rot, false, sh, g.time, true, Some(c));
        draw_cube_wires(pos, size, Color::new(c.r, c.g, c.b, 0.9));
    }
    if g.placing.is_none() && g.pilot.is_none() && g.menu == Menu::None {
        if let Some(i) = g.aimed_module() {
            let m = &g.modules[i];
            if m.pos.distance(g.pos) < 16.0 && is_key_down(KeyCode::X) {
                draw_cube_wires(m.pos, m.kind.size(m.rot), Color::new(1.0, 0.6, 0.2, 1.0));
            }
        }
    }
}

fn draw_vehicles(g: &Game, sh: &Shade) {
    let t = g.time;
    for v in &g.vehicles {
        if v.pos.distance(sh.cam) > sh.view_dist() + 20.0 {
            continue;
        }
        let p = v.pos;
        match v.kind {
            VehKind::Skat => {
                // Скат: плоский клиновидный корпус, узкий фонарь сверху, хвостовой руль
                push(tf(p, v.yaw, v.pitch, Vec3::ONE));
                let hull = sh.col(Color::new(0.95, 0.75, 0.2, 1.0), p, 0.0);
                let dark = sh.col(Color::new(0.2, 0.22, 0.25, 1.0), p, 0.0);
                cube(vec3(0.0, 0.0, 0.0), vec3(4.4, 0.7, 3.4), hull);
                cube(vec3(2.6, -0.05, 0.0), vec3(1.2, 0.5, 2.2), hull);
                cube(vec3(3.4, -0.1, 0.0), vec3(0.6, 0.35, 1.2), hull);
                cube(vec3(0.0, -0.4, 0.0), vec3(4.2, 0.15, 3.5), dark);
                cube(vec3(-0.6, 0.0, 1.75), vec3(3.0, 0.3, 0.1), dark);
                cube(vec3(-0.6, 0.0, -1.75), vec3(3.0, 0.3, 0.1), dark);
                cube(vec3(0.9, 0.5, 0.0), vec3(1.8, 0.35, 0.9), sh.col(Color::new(0.3, 0.8, 0.95, 1.0), p, 0.5));
                cube(vec3(-2.3, 0.7, 0.0), vec3(0.9, 1.2, 0.12), hull);
                cube(vec3(-2.4, 0.0, 0.0), vec3(0.3, 0.45, 1.6), dark);
                let jet = sh.col(Color::new(0.4, 0.9, 1.0, 1.0), p, 1.0);
                let thrust = (v.vel.length() / 22.0).min(1.0);
                sph(vec3(-2.6, 0.0, 0.5), 0.15 + 0.2 * thrust, jet, 5);
                sph(vec3(-2.6, 0.0, -0.5), 0.15 + 0.2 * thrust, jet, 5);
                let lamp = sh.col(Color::new(1.0, 1.0, 0.85, 1.0), p, 1.0);
                sph(vec3(3.7, -0.1, 0.45), 0.18, lamp, 5);
                sph(vec3(3.7, -0.1, -0.45), 0.18, lamp, 5);
                pop();
            }
            VehKind::Kit => {
                // Кит: длинный корпус-кашалот, рубка посередине, ряд иллюминаторов, винт
                push(tf(p, v.yaw, v.pitch, Vec3::ONE));
                let hull = sh.col(Color::new(0.24, 0.32, 0.4, 1.0), p, 0.0);
                let belly = sh.col(Color::new(0.6, 0.62, 0.6, 1.0), p, 0.0);
                let stripe = sh.col(Color::new(0.75, 0.2, 0.15, 1.0), p, 0.0);
                cube(vec3(0.0, 0.0, 0.0), vec3(20.0, 5.0, 6.0), hull);
                cube(vec3(11.5, 0.3, 0.0), vec3(3.0, 4.4, 5.2), hull);
                cube(vec3(13.6, 0.5, 0.0), vec3(1.4, 3.2, 3.8), hull);
                cube(vec3(-11.5, 0.0, 0.0), vec3(3.0, 4.0, 4.6), hull);
                cube(vec3(-13.6, 0.0, 0.0), vec3(1.6, 2.6, 3.0), hull);
                cube(vec3(0.0, -2.3, 0.0), vec3(24.0, 0.6, 5.4), belly);
                cube(vec3(0.0, -1.4, 0.0), vec3(26.0, 0.35, 6.1), stripe);
                cube(vec3(1.0, 4.0, 0.0), vec3(6.0, 3.2, 2.8), hull);
                cube(vec3(1.0, 5.8, 0.0), vec3(4.0, 0.4, 2.0), belly);
                let port = sh.col(Color::new(1.0, 0.9, 0.6, 1.0), p, 0.9);
                for k in 0..8 {
                    let x = -9.0 + k as f32 * 2.6;
                    cube(vec3(x, 0.8, 3.02), vec3(0.8, 0.8, 0.08), port);
                    cube(vec3(x, 0.8, -3.02), vec3(0.8, 0.8, 0.08), port);
                }
                cube(vec3(2.5, 4.3, 1.42), vec3(2.5, 0.7, 0.06), port);
                cube(vec3(2.5, 4.3, -1.42), vec3(2.5, 0.7, 0.06), port);
                // стабилизаторы
                cube(vec3(-11.0, 3.0, 0.0), vec3(3.0, 2.6, 0.3), hull);
                cube(vec3(-11.0, 0.0, 3.4), vec3(3.0, 0.3, 2.0), hull);
                cube(vec3(-11.0, 0.0, -3.4), vec3(3.0, 0.3, 2.0), hull);
                // винт
                push(Mat4::from_translation(vec3(-14.6, 0.0, 0.0)) * Mat4::from_rotation_x(v.prop * 3.0 + t * 0.3));
                let blade = sh.col(Color::new(0.7, 0.6, 0.3, 1.0), p, 0.0);
                cube(Vec3::ZERO, vec3(0.3, 4.2, 0.6), blade);
                cube(Vec3::ZERO, vec3(0.3, 0.6, 4.2), blade);
                pop();
                let lamp = sh.col(Color::new(1.0, 1.0, 0.85, 1.0), p, 1.0);
                sph(vec3(14.3, -0.6, 1.2), 0.35, lamp, 6);
                sph(vec3(14.3, -0.6, -1.2), 0.35, lamp, 6);
                pop();
            }
        }
    }
}

// ---------- HUD ----------
pub fn txt(gx: &Gfx, s: &str, x: f32, y: f32, size: f32, c: Color) {
    draw_text_ex(s, x, y, TextParams { font: Some(&gx.font), font_size: size as u16, color: c, ..Default::default() });
}

pub fn txt_w(gx: &Gfx, s: &str, size: f32) -> f32 {
    measure_text(s, Some(&gx.font), size as u16, 1.0).width
}

pub fn txt_c(gx: &Gfx, s: &str, cx: f32, y: f32, size: f32, c: Color) {
    let w = txt_w(gx, s, size);
    txt(gx, s, cx - w / 2.0, y, size, c);
}

fn shadow_txt(gx: &Gfx, s: &str, x: f32, y: f32, size: f32, c: Color) {
    txt(gx, s, x + 1.5, y + 1.5, size, Color::new(0.0, 0.0, 0.0, 0.7 * c.a));
    txt(gx, s, x, y, size, c);
}

fn bar(x: f32, y: f32, w: f32, h: f32, frac: f32, c: Color) {
    draw_rectangle(x, y, w, h, Color::new(0.0, 0.0, 0.0, 0.5));
    draw_rectangle(x, y, w * frac.clamp(0.0, 1.0), h, c);
    draw_rectangle_lines(x, y, w, h, 1.5, Color::new(1.0, 1.0, 1.0, 0.35));
}

fn to_screen(m: &Mat4, p: Vec3) -> Option<Vec2> {
    let c = *m * p.extend(1.0);
    if c.w <= 0.05 {
        return None;
    }
    let n = c.truncate() / c.w;
    Some(vec2((n.x + 1.0) * 0.5 * screen_width(), (1.0 - n.y) * 0.5 * screen_height()))
}

pub fn scale() -> f32 {
    (screen_height() / 720.0).clamp(0.7, 1.8)
}

pub fn menu_rect(rows: usize) -> Rect {
    let s = scale();
    let w = (680.0 * s).min(screen_width() - 20.0);
    let h = (90.0 + rows as f32 * 54.0) * s;
    Rect::new((screen_width() - w) / 2.0, (screen_height() - h) / 2.0, w, h)
}

pub fn menu_row(rows: usize, i: usize) -> Rect {
    let s = scale();
    let r = menu_rect(rows);
    Rect::new(r.x + 12.0 * s, r.y + 70.0 * s + i as f32 * 54.0 * s, r.w - 24.0 * s, 48.0 * s)
}

fn draw_cost(gx: &Gfx, g: &Game, cost: &[u32; 6], right: f32, y: f32, s: f32) {
    let mut x = right;
    for k in RES.iter().rev() {
        let need = cost[k.idx()];
        if need == 0 {
            continue;
        }
        x -= 56.0 * s;
        draw_rectangle(x, y - 12.0 * s, 12.0 * s, 12.0 * s, k.color());
        let ok = g.inv[k.idx()] >= need;
        txt(gx, &format!("{}", need), x + 16.0 * s, y, 18.0 * s, if ok { WHITE } else { Color::new(1.0, 0.45, 0.45, 1.0) });
    }
}

pub fn draw_hud(g: &Game, gx: &Gfx, view: &Mat4) {
    let s = scale();
    let (sw, shh) = (screen_width(), screen_height());
    let t = g.time;
    let fs = 19.0 * s;
    let pad = 14.0 * s;

    // сонар
    if g.sonar_t > 0.0 {
        let a = g.sonar_t.min(1.0);
        for c in &g.world.cores {
            if c.taken {
                continue;
            }
            if let Some(sp) = to_screen(view, c.pos) {
                let d = c.pos.distance(g.eye());
                draw_circle_lines(sp.x, sp.y, 10.0 * s, 2.0, Color::new(1.0, 0.4, 0.65, a));
                shadow_txt(gx, &format!("Ядро · {:.0} м", d), sp.x + 14.0 * s, sp.y + 5.0 * s, fs * 0.8, Color::new(1.0, 0.6, 0.8, a));
            }
        }
        for k in RES {
            let mut best: Option<(f32, Vec3)> = None;
            for n in &g.world.nodes {
                if n.alive && n.kind == k {
                    let d = n.pos.distance(g.eye());
                    if d < 150.0 && best.map_or(true, |b| d < b.0) {
                        best = Some((d, n.pos));
                    }
                }
            }
            if let Some((d, p)) = best {
                if let Some(sp) = to_screen(view, p) {
                    let mut c = k.color();
                    c.a = a;
                    draw_circle_lines(sp.x, sp.y, 6.0 * s, 2.0, c);
                    shadow_txt(gx, &format!("{} {:.0} м", k.name(), d), sp.x + 10.0 * s, sp.y + 4.0 * s, fs * 0.7, c);
                }
            }
        }
        for m in &g.modules {
            if m.kind == ModKind::Room || m.kind == ModKind::Shipyard {
                if let Some(sp) = to_screen(view, m.pos) {
                    shadow_txt(gx, &format!("База · {:.0} м", m.pos.distance(g.eye())), sp.x, sp.y, fs * 0.75, Color::new(0.6, 1.0, 0.7, a));
                }
            }
        }
        if let Some(sp) = to_screen(view, g.raft) {
            shadow_txt(gx, &format!("Плот · {:.0} м", g.raft.distance(g.eye())), sp.x, sp.y, fs * 0.75, Color::new(1.0, 0.85, 0.5, a));
        }
    }

    // прицел
    if g.pilot.is_none() && g.menu == Menu::None {
        draw_circle(sw / 2.0, shh / 2.0, 2.5 * s, Color::new(1.0, 1.0, 1.0, 0.8));
    }

    // панель состояния
    draw_rectangle(pad - 6.0, pad - 6.0, 330.0 * s, 168.0 * s, Color::new(0.0, 0.04, 0.08, 0.55));
    let low = g.o2 < g.max_o2 * 0.25 && (t * 6.0).sin() > 0.0;
    shadow_txt(gx, "O₂", pad, pad + fs * 0.9, fs, if low { RED } else { WHITE });
    bar(pad + 40.0 * s, pad, 250.0 * s, fs, g.o2 / g.max_o2, Color::new(0.3, 0.85, 1.0, 1.0));
    txt(gx, &format!("{:.0}", g.o2), pad + 46.0 * s, pad + fs * 0.85, fs * 0.8, BLACK);
    shadow_txt(gx, "HP", pad, pad + fs * 2.3, fs, WHITE);
    bar(pad + 40.0 * s, pad + fs * 1.45, 250.0 * s, fs, g.hp / 100.0, Color::new(1.0, 0.35, 0.35, 1.0));

    let depth = g.depth_m();
    let limit = match g.pilot {
        Some(i) => g.vehicles[i].kind.limit(),
        None => g.suit_limit,
    };
    let danger = depth > limit;
    let b = g.body();
    shadow_txt(
        gx,
        &format!("{:.0} м · предел {:.0} м", depth, limit),
        pad,
        pad + fs * 3.7,
        fs * 0.9,
        if danger { RED } else { WHITE },
    );
    shadow_txt(gx, g.world.biome_at(b.x, b.z).name(), pad, pad + fs * 4.9, fs * 0.85, Color::new(0.7, 0.9, 1.0, 1.0));
    let mut x = pad;
    let iy = pad + fs * 5.9;
    for k in RES {
        draw_rectangle(x, iy, 12.0 * s, 12.0 * s, k.color());
        txt(gx, &format!("{}", g.inv[k.idx()]), x + 15.0 * s, iy + 12.0 * s, fs * 0.8, WHITE);
        x += 52.0 * s;
    }
    shadow_txt(gx, &format!("Ядра: {}/4", g.cores), pad, pad + fs * 7.9, fs * 0.9, Color::new(1.0, 0.6, 0.8, 1.0));

    if let Some(i) = g.pilot {
        let v = &g.vehicles[i];
        let y = pad + 172.0 * s;
        draw_rectangle(pad - 6.0, y - 6.0, 330.0 * s, 54.0 * s, Color::new(0.0, 0.04, 0.08, 0.55));
        shadow_txt(gx, v.kind.name(), pad, y + fs * 0.9, fs, Color::new(1.0, 0.85, 0.4, 1.0));
        bar(pad + 70.0 * s, y, 220.0 * s, fs, v.hull / v.kind.max_hull(), Color::new(0.95, 0.75, 0.3, 1.0));
        shadow_txt(gx, &format!("скорость {:.0} м/с", v.vel.length()), pad, y + fs * 2.2, fs * 0.8, WHITE);
    }

    // цель
    let goal = if g.cores < 4 {
        "Цель: собери 4 ядра (по одному в каждом биоме) · Q — сонар"
    } else {
        "Цель: построй Резонатор на базе (B → 9)"
    };
    shadow_txt(gx, goal, pad, shh - pad, fs * 0.85, Color::new(1.0, 0.75, 0.85, 1.0));

    if danger && (t * 5.0).sin() > -0.2 {
        txt_c(gx, "ДАВЛЕНИЕ ВЫШЕ ПРЕДЕЛА!", sw / 2.0, 80.0 * s, fs * 1.3, RED);
    }
    if g.o2 <= 0.0 {
        txt_c(gx, "НЕЧЕМ ДЫШАТЬ — НАВЕРХ ИЛИ НА БАЗУ!", sw / 2.0, 110.0 * s, fs * 1.3, RED);
    }

    // подсказка взаимодействия
    let hint = match g.target() {
        Some(Target::Core(_)) => Some("[E] забрать ядро".to_string()),
        Some(Target::Vehicle(i)) => Some(format!("[E] сесть в {}", g.vehicles[i].kind.name())),
        Some(Target::Node(i)) => Some(format!("[E] собрать: {}", g.world.nodes[i].kind.name())),
        None => None,
    };
    if let Some(h) = hint {
        if g.menu == Menu::None {
            let w = txt_w(gx, &h, fs);
            shadow_txt(gx, &h, sw / 2.0 - w / 2.0, shh / 2.0 + 40.0 * s, fs, WHITE);
        }
    }
    if g.pilot.is_some() && g.menu == Menu::None {
        txt_c(gx, "[E] выйти · Q — сонар", sw / 2.0, shh - 60.0 * s, fs * 0.85, Color::new(1.0, 1.0, 1.0, 0.7));
    }
    if let Some(k) = g.placing {
        let line = format!("Строишь: {} · ЛКМ/F — поставить · R — повернуть · B — отмена", k.name());
        let w = txt_w(gx, &line, fs * 0.9) + 20.0 * s;
        draw_rectangle(sw / 2.0 - w / 2.0, shh - 92.0 * s, w, fs * 1.6, Color::new(0.0, 0.0, 0.0, 0.5));
        txt_c(gx, &line, sw / 2.0, shh - 92.0 * s + fs * 1.1, fs * 0.9, Color::new(0.6, 1.0, 0.7, 1.0));
    } else if g.pilot.is_none() && g.menu == Menu::None {
        if let Some(i) = g.aimed_module() {
            if g.modules[i].pos.distance(g.pos) < 16.0 && g.modules[i].kind != ModKind::Resonator {
                txt_c(gx, &format!("[X] разобрать: {}", g.modules[i].kind.name()), sw / 2.0, shh / 2.0 + 70.0 * s, fs * 0.85, Color::new(1.0, 0.75, 0.5, 0.9));
            }
        }
    }

    if g.msg_t > 0.0 {
        let a = g.msg_t.min(1.0);
        let w = txt_w(gx, &g.msg, fs) + 30.0 * s;
        let y = shh - 140.0 * s;
        draw_rectangle(sw / 2.0 - w / 2.0, y, w, fs * 1.8, Color::new(0.0, 0.0, 0.0, 0.6 * a));
        txt_c(gx, &g.msg, sw / 2.0, y + fs * 1.25, fs, Color::new(1.0, 1.0, 1.0, a));
    }

    if g.help {
        let lines = [
            "Мышь — смотреть · WASD — плыть",
            "Space/Ctrl — вверх/вниз · Shift — рывок",
            "E — взять / сесть / выйти · Q — сонар",
            "Tab — крафт · B — стройка · X — разобрать",
            "Esc — отпустить мышь · H — скрыть подсказки",
        ];
        let w = 450.0 * s;
        draw_rectangle(sw - w - pad + 6.0, pad - 6.0, w, fs * 1.25 * lines.len() as f32 + 12.0 * s, Color::new(0.0, 0.04, 0.08, 0.5));
        for (i, l) in lines.iter().enumerate() {
            txt(gx, l, sw - w - pad + 16.0 * s, pad + fs * (0.9 + i as f32 * 1.25), fs * 0.8, Color::new(1.0, 1.0, 1.0, 0.85));
        }
    }

    match g.menu {
        Menu::Craft => {
            let n = RECIPES.len();
            let r = menu_rect(n);
            draw_rectangle(0.0, 0.0, sw, shh, Color::new(0.0, 0.0, 0.0, 0.45));
            draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.03, 0.1, 0.16, 0.96));
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::new(0.3, 0.85, 1.0, 0.7));
            txt(gx, "Крафт", r.x + 16.0 * s, r.y + 40.0 * s, fs * 1.4, WHITE);
            txt(gx, "1–7 или клик · Tab — закрыть", r.x + 130.0 * s, r.y + 38.0 * s, fs * 0.75, GRAY);
            let yard = g.near_shipyard().is_some();
            for (i, rec) in RECIPES.iter().enumerate() {
                let row = menu_row(n, i);
                let done = !rec.vehicle && g.upgrades[i];
                let ok = g.affordable(&rec.cost) && (!rec.vehicle || yard);
                draw_rectangle(row.x, row.y, row.w, row.h, if done {
                    Color::new(0.1, 0.25, 0.15, 0.8)
                } else if ok {
                    Color::new(0.1, 0.25, 0.35, 0.9)
                } else {
                    Color::new(0.08, 0.08, 0.1, 0.8)
                });
                let nc = if done { Color::new(0.5, 1.0, 0.6, 1.0) } else if ok { WHITE } else { GRAY };
                txt(gx, &format!("{}. {}", i + 1, rec.name), row.x + 10.0 * s, row.y + 21.0 * s, fs * 0.9, nc);
                let d = if done {
                    "готово"
                } else if rec.vehicle && !yard {
                    "нужна Верфь рядом"
                } else {
                    rec.desc
                };
                txt(gx, d, row.x + 10.0 * s, row.y + 41.0 * s, fs * 0.7, Color::new(0.75, 0.85, 0.9, 1.0));
                if !done {
                    draw_cost(gx, g, &rec.cost, row.x + row.w - 6.0 * s, row.y + 30.0 * s, s);
                }
            }
        }
        Menu::Build => {
            let n = MODS.len();
            let r = menu_rect(n);
            draw_rectangle(0.0, 0.0, sw, shh, Color::new(0.0, 0.0, 0.0, 0.45));
            draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.04, 0.12, 0.08, 0.96));
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::new(0.4, 1.0, 0.6, 0.7));
            txt(gx, "Стройка", r.x + 16.0 * s, r.y + 40.0 * s, fs * 1.4, WHITE);
            txt(gx, "1–9 или клик · B — закрыть", r.x + 150.0 * s, r.y + 38.0 * s, fs * 0.75, GRAY);
            for (i, k) in MODS.iter().enumerate() {
                let row = menu_row(n, i);
                let mut ok = g.affordable(&k.cost());
                if *k == ModKind::Resonator && g.cores < 4 {
                    ok = false;
                }
                draw_rectangle(row.x, row.y, row.w, row.h, if ok { Color::new(0.1, 0.3, 0.2, 0.9) } else { Color::new(0.08, 0.08, 0.1, 0.8) });
                txt(gx, &format!("{}. {}", i + 1, k.name()), row.x + 10.0 * s, row.y + 21.0 * s, fs * 0.9, if ok { WHITE } else { GRAY });
                let d = if *k == ModKind::Resonator {
                    format!("{} · ядра {}/4", k.desc(), g.cores)
                } else {
                    k.desc().to_string()
                };
                txt(gx, &d, row.x + 10.0 * s, row.y + 41.0 * s, fs * 0.7, Color::new(0.75, 0.9, 0.8, 1.0));
                draw_cost(gx, g, &k.cost(), row.x + row.w - 6.0 * s, row.y + 30.0 * s, s);
            }
        }
        Menu::None => {}
    }

    if g.hurt > 0.0 {
        draw_rectangle(0.0, 0.0, sw, shh, Color::new(0.8, 0.0, 0.0, g.hurt * 0.4));
    }

    match g.state {
        State::Dead(why) => {
            draw_rectangle(0.0, 0.0, sw, shh, Color::new(0.0, 0.0, 0.0, 0.75));
            txt_c(gx, "ТЫ ПОГИБ", sw / 2.0, shh / 2.0 - 30.0 * s, 56.0 * s, RED);
            txt_c(gx, why, sw / 2.0, shh / 2.0 + 20.0 * s, 24.0 * s, WHITE);
            txt_c(gx, "R / клик — очнуться (база или плот, половина груза пропадёт)", sw / 2.0, shh / 2.0 + 64.0 * s, 18.0 * s, GRAY);
        }
        State::Won => {
            draw_rectangle(0.0, 0.0, sw, shh, Color::new(0.05, 0.0, 0.08, 0.7));
            txt_c(gx, "РЕЗОНАТОР ЗАПУЩЕН", sw / 2.0, shh / 2.0 - 40.0 * s, 52.0 * s, Color::new(1.0, 0.55, 0.8, 1.0));
            txt_c(gx, "Сигнал ушёл сквозь толщу воды. Тебя услышали.", sw / 2.0, shh / 2.0 + 10.0 * s, 24.0 * s, WHITE);
            let m = (g.time / 60.0) as i32;
            txt_c(gx, &format!("Время: {}:{:02}", m, (g.time % 60.0) as i32), sw / 2.0, shh / 2.0 + 48.0 * s, 22.0 * s, WHITE);
            txt_c(gx, "Enter — новый мир · Esc — плавать дальше", sw / 2.0, shh / 2.0 + 90.0 * s, 18.0 * s, GRAY);
        }
        State::Play => {}
    }
}
