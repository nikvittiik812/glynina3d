// Мир: рельеф дна, биомы, флора, ресурсы, ядра.

use macroquad::prelude::*;
use macroquad::rand::gen_range;
use std::f32::consts::PI;

pub const WORLD_R: f32 = 300.0;
pub const GRID: usize = 96;
pub const SPAN: f32 = 640.0;
pub const CELL: f32 = SPAN / GRID as f32;
pub const CHUNK: usize = 16;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Biome {
    Shallows,
    Forest,
    Crystal,
    Abyss,
}

pub const BIOMES: [Biome; 4] = [Biome::Shallows, Biome::Forest, Biome::Crystal, Biome::Abyss];

impl Biome {
    pub fn name(self) -> &'static str {
        match self {
            Biome::Shallows => "Солнечные отмели",
            Biome::Forest => "Стеблевой лес",
            Biome::Crystal => "Поющие кристаллы",
            Biome::Abyss => "Пепельная бездна",
        }
    }
    fn ground(self) -> Color {
        match self {
            Biome::Shallows => Color::new(0.86, 0.76, 0.55, 1.0),
            Biome::Forest => Color::new(0.30, 0.36, 0.22, 1.0),
            Biome::Crystal => Color::new(0.48, 0.42, 0.62, 1.0),
            Biome::Abyss => Color::new(0.20, 0.13, 0.12, 1.0),
        }
    }
    // направление сектора и глубина
    fn sector(self) -> (f32, f32) {
        match self {
            Biome::Shallows => (0.0, 22.0),
            Biome::Forest => (0.8, 85.0),
            Biome::Crystal => (PI, 140.0),
            Biome::Abyss => (-PI / 2.0, 265.0),
        }
    }
}

// ---------- ресурсы ----------
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Res {
    Copper,
    Titanium,
    Quartz,
    Lithium,
    Crystal,
    Magma,
}
pub const RES: [Res; 6] = [Res::Copper, Res::Titanium, Res::Quartz, Res::Lithium, Res::Crystal, Res::Magma];

impl Res {
    pub fn idx(self) -> usize {
        self as usize
    }
    pub fn name(self) -> &'static str {
        match self {
            Res::Copper => "Медь",
            Res::Titanium => "Титан",
            Res::Quartz => "Кварц",
            Res::Lithium => "Литий",
            Res::Crystal => "Кристалл",
            Res::Magma => "Магмит",
        }
    }
    pub fn color(self) -> Color {
        match self {
            Res::Copper => Color::new(0.95, 0.55, 0.25, 1.0),
            Res::Titanium => Color::new(0.75, 0.78, 0.86, 1.0),
            Res::Quartz => Color::new(0.92, 0.97, 1.0, 1.0),
            Res::Lithium => Color::new(0.75, 0.95, 0.55, 1.0),
            Res::Crystal => Color::new(0.65, 0.45, 1.0, 1.0),
            Res::Magma => Color::new(1.0, 0.35, 0.1, 1.0),
        }
    }
    pub fn glows(self) -> bool {
        matches!(self, Res::Crystal | Res::Magma)
    }
}

pub struct Node {
    pub pos: Vec3,
    pub kind: Res,
    pub alive: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum FloraKind {
    Fan,    // коралловый веер
    Tube,   // трубчатый коралл
    Grass,  // морская трава
    Stalk,  // гигантский стебель со светящимися почками
    Spike,  // поющий кристалл
    Vent,   // жерло
    Spire,  // пепельный шпиль
}

pub struct Flora {
    pub pos: Vec3,
    pub kind: FloraKind,
    pub scale: f32,
    pub color: Color,
    pub phase: f32,
}

pub struct Core {
    pub pos: Vec3,
    pub biome: Biome,
    pub taken: bool,
}

pub struct World {
    ph: [f32; 8],
    pub heights: Vec<f32>,
    pub base_col: Vec<Color>,
    pub relief: Vec<f32>,
    pub flora: Vec<Flora>,
    pub nodes: Vec<Node>,
    pub cores: Vec<Core>,
}

fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn ang_diff(a: f32, b: f32) -> f32 {
    let mut d = a - b;
    while d > PI {
        d -= 2.0 * PI;
    }
    while d < -PI {
        d += 2.0 * PI;
    }
    d
}

pub fn mix(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

impl World {
    pub fn weights(&self, x: f32, z: f32) -> [f32; 4] {
        let r = (x * x + z * z).sqrt();
        // неровная граница отмелей
        let wob = (z.atan2(x) * 3.0 + self.ph[6]).sin() * 15.0;
        let s = 1.0 - smooth(60.0 + wob, 125.0 + wob, r);
        let ang = z.atan2(x);
        let mut w = [0.0f32; 3];
        let mut sum = 0.0;
        for (i, b) in [Biome::Forest, Biome::Crystal, Biome::Abyss].iter().enumerate() {
            let c = ang_diff(ang, b.sector().0).cos().max(0.0);
            w[i] = c.powi(4) + 0.0005;
            sum += w[i];
        }
        [s, (1.0 - s) * w[0] / sum, (1.0 - s) * w[1] / sum, (1.0 - s) * w[2] / sum]
    }

    pub fn biome_at(&self, x: f32, z: f32) -> Biome {
        let w = self.weights(x, z);
        let mut best = 0;
        for i in 1..4 {
            if w[i] > w[best] {
                best = i;
            }
        }
        BIOMES[best]
    }

    // глубина дна в метрах (положительная)
    pub fn depth(&self, x: f32, z: f32) -> f32 {
        let w = self.weights(x, z);
        let p = &self.ph;
        let mut d = 0.0;
        for (i, b) in BIOMES.iter().enumerate() {
            d += w[i] * b.sector().1;
        }
        let n = (x * 0.031 + p[0]).sin() * (z * 0.027 + p[1]).cos() * 6.0
            + (x * 0.07 + p[2]).sin() * 2.5
            + (z * 0.09 + p[3]).cos() * 2.5;
        // кристаллы: гребни; бездна: изрезанные каньоны
        let ridge = ((x * 0.05 + p[4]).sin() * (z * 0.045 + p[5]).sin()).abs() * 22.0 * w[2];
        let canyon = ((x * 0.02 + z * 0.015 + p[7]).sin()) * 30.0 * w[3];
        d += n * (1.0 + w[1]) - ridge + canyon;
        // край мира — стены
        let r = (x * x + z * z).sqrt();
        let edge = smooth(WORLD_R - 30.0, WORLD_R + 10.0, r);
        d = d * (1.0 - edge) + 4.0 * edge;
        d.max(3.0)
    }

    pub fn floor_y(&self, x: f32, z: f32) -> f32 {
        -self.depth(x, z)
    }

    pub fn generate() -> World {
        let mut w = World {
            ph: [0.0; 8],
            heights: vec![],
            base_col: vec![],
            relief: vec![],
            flora: vec![],
            nodes: vec![],
            cores: vec![],
        };
        for i in 0..8 {
            w.ph[i] = gen_range(0.0, 2.0 * PI);
        }
        let n = GRID + 1;
        for j in 0..n {
            for i in 0..n {
                let x = -SPAN / 2.0 + i as f32 * CELL;
                let z = -SPAN / 2.0 + j as f32 * CELL;
                w.heights.push(w.floor_y(x, z));
                let ws = w.weights(x, z);
                let mut c = Color::new(0.0, 0.0, 0.0, 1.0);
                for (k, b) in BIOMES.iter().enumerate() {
                    let g = b.ground();
                    c.r += g.r * ws[k];
                    c.g += g.g * ws[k];
                    c.b += g.b * ws[k];
                }
                let jitter = gen_range(0.9, 1.08);
                c.r *= jitter;
                c.g *= jitter;
                c.b *= jitter;
                w.base_col.push(c);
            }
        }
        // освещённость по наклону
        for j in 0..n {
            for i in 0..n {
                let h = |ii: usize, jj: usize| w.heights[jj.min(n - 1) * n + ii.min(n - 1)];
                let dx = h(i + 1, j) - h(i.saturating_sub(1), j);
                let dz = h(i, j + 1) - h(i, j.saturating_sub(1));
                let nrm = vec3(-dx, 2.0 * CELL, -dz).normalize();
                let l = vec3(0.3, 1.0, 0.2).normalize();
                w.relief.push(0.45 + 0.55 * nrm.dot(l).max(0.0));
            }
        }

        // флора
        let mut tries = 0;
        while w.flora.len() < 1500 && tries < 20000 {
            tries += 1;
            let a = gen_range(0.0, 2.0 * PI);
            let r = gen_range(0.0, 1.0f32).sqrt() * (WORLD_R - 20.0);
            let (x, z) = (a.cos() * r, a.sin() * r);
            if x.abs() < 12.0 && z.abs() < 12.0 {
                continue; // место старта
            }
            let y = w.floor_y(x, z);
            let b = w.biome_at(x, z);
            let roll = gen_range(0.0, 1.0);
            let (kind, color, scale) = match b {
                Biome::Shallows => {
                    if roll < 0.35 {
                        let pal = [
                            Color::new(1.0, 0.45, 0.5, 1.0),
                            Color::new(1.0, 0.7, 0.25, 1.0),
                            Color::new(0.85, 0.4, 0.95, 1.0),
                        ];
                        (FloraKind::Fan, pal[gen_range(0, 3)], gen_range(0.8, 1.6))
                    } else if roll < 0.6 {
                        (FloraKind::Tube, Color::new(0.95, 0.85, 0.4, 1.0), gen_range(0.7, 1.4))
                    } else {
                        (FloraKind::Grass, Color::new(0.35, 0.7, 0.3, 1.0), gen_range(0.8, 1.5))
                    }
                }
                Biome::Forest => {
                    if roll < 0.55 {
                        (FloraKind::Stalk, Color::new(0.35, 0.55, 0.3, 1.0), gen_range(0.7, 1.3))
                    } else {
                        (FloraKind::Grass, Color::new(0.25, 0.5, 0.35, 1.0), gen_range(1.2, 2.2))
                    }
                }
                Biome::Crystal => {
                    let pal = [Color::new(0.6, 0.4, 1.0, 1.0), Color::new(0.35, 0.85, 1.0, 1.0)];
                    (FloraKind::Spike, pal[gen_range(0, 2)], gen_range(0.6, 2.2))
                }
                Biome::Abyss => {
                    if roll < 0.25 {
                        (FloraKind::Vent, Color::new(0.15, 0.1, 0.1, 1.0), gen_range(0.8, 1.5))
                    } else {
                        (FloraKind::Spire, Color::new(0.16, 0.12, 0.13, 1.0), gen_range(0.8, 2.5))
                    }
                }
            };
            w.flora.push(Flora { pos: vec3(x, y, z), kind, scale, color, phase: gen_range(0.0, 6.28) });
        }

        // ресурсы
        let mut tries = 0;
        while w.nodes.len() < 420 && tries < 20000 {
            tries += 1;
            let a = gen_range(0.0, 2.0 * PI);
            let r = gen_range(0.05, 1.0f32).sqrt() * (WORLD_R - 25.0);
            let (x, z) = (a.cos() * r, a.sin() * r);
            let y = w.floor_y(x, z) + 0.6;
            let roll = gen_range(0.0, 1.0);
            let kind = match w.biome_at(x, z) {
                Biome::Shallows => {
                    if roll < 0.4 {
                        Res::Titanium
                    } else if roll < 0.75 {
                        Res::Copper
                    } else {
                        Res::Quartz
                    }
                }
                Biome::Forest => {
                    if roll < 0.4 {
                        Res::Lithium
                    } else if roll < 0.75 {
                        Res::Titanium
                    } else {
                        Res::Copper
                    }
                }
                Biome::Crystal => {
                    if roll < 0.45 {
                        Res::Crystal
                    } else if roll < 0.75 {
                        Res::Quartz
                    } else {
                        Res::Lithium
                    }
                }
                Biome::Abyss => {
                    if roll < 0.6 {
                        Res::Magma
                    } else {
                        Res::Crystal
                    }
                }
            };
            w.nodes.push(Node { pos: vec3(x, y, z), kind, alive: true });
        }

        // ядра: по одному в сердце каждого биома
        for b in BIOMES {
            let (ang, rad) = match b {
                Biome::Shallows => (gen_range(0.0, 2.0 * PI), 45.0),
                _ => (b.sector().0 + gen_range(-0.15, 0.15), 205.0),
            };
            let (x, z) = (ang.cos() * rad, ang.sin() * rad);
            w.cores.push(Core { pos: vec3(x, w.floor_y(x, z) + 2.0, z), biome: b, taken: false });
        }
        w
    }

    pub fn vertex(&self, i: usize, j: usize) -> (Vec3, usize) {
        let k = j * (GRID + 1) + i;
        (vec3(-SPAN / 2.0 + i as f32 * CELL, self.heights[k], -SPAN / 2.0 + j as f32 * CELL), k)
    }
}
