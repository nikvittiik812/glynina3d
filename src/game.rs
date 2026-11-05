// Состояние игры и логика: игрок, база, транспорт, существа.

use crate::world::*;
use macroquad::prelude::*;
use macroquad::rand::gen_range;
use std::f32::consts::PI;

// ---------- модули базы ----------
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ModKind {
    Foundation,
    Corridor,
    Room,
    Dome,
    Generator,
    Lamp,
    Pillar,
    Shipyard,
    Resonator,
}

pub const MODS: [ModKind; 9] = [
    ModKind::Foundation,
    ModKind::Corridor,
    ModKind::Room,
    ModKind::Dome,
    ModKind::Generator,
    ModKind::Lamp,
    ModKind::Pillar,
    ModKind::Shipyard,
    ModKind::Resonator,
];

impl ModKind {
    pub fn name(self) -> &'static str {
        match self {
            ModKind::Foundation => "Фундамент",
            ModKind::Corridor => "Коридор",
            ModKind::Room => "Отсек",
            ModKind::Dome => "Обзорный купол",
            ModKind::Generator => "Генератор",
            ModKind::Lamp => "Прожектор",
            ModKind::Pillar => "Опора",
            ModKind::Shipyard => "Верфь",
            ModKind::Resonator => "Резонатор",
        }
    }
    pub fn desc(self) -> &'static str {
        match self {
            ModKind::Foundation => "площадка под постройки",
            ModKind::Corridor => "воздух внутри (нужен генератор)",
            ModKind::Room => "большой отсек с воздухом",
            ModKind::Dome => "стеклянный купол с воздухом",
            ModKind::Generator => "питает всё в радиусе 45 м",
            ModKind::Lamp => "освещает округу",
            ModKind::Pillar => "декор и опора",
            ModKind::Shipyard => "собирает транспорт",
            ModKind::Resonator => "ЦЕЛЬ: нужны все 4 ядра",
        }
    }
    // медь, титан, кварц, литий, кристалл, магмит
    pub fn cost(self) -> [u32; 6] {
        match self {
            ModKind::Foundation => [0, 2, 0, 0, 0, 0],
            ModKind::Corridor => [0, 1, 0, 0, 0, 0],
            ModKind::Room => [0, 3, 1, 0, 0, 0],
            ModKind::Dome => [0, 2, 3, 0, 0, 0],
            ModKind::Generator => [2, 0, 1, 1, 0, 0],
            ModKind::Lamp => [1, 0, 1, 0, 0, 0],
            ModKind::Pillar => [0, 1, 0, 0, 0, 0],
            ModKind::Shipyard => [2, 4, 0, 2, 0, 0],
            ModKind::Resonator => [0, 0, 0, 0, 3, 3],
        }
    }
    pub fn size(self, rot: bool) -> Vec3 {
        let s = match self {
            ModKind::Foundation => vec3(8.0, 1.0, 8.0),
            ModKind::Corridor => vec3(8.0, 3.0, 3.0),
            ModKind::Room => vec3(8.0, 5.0, 8.0),
            ModKind::Dome => vec3(8.0, 8.0, 8.0),
            ModKind::Generator => vec3(2.0, 3.0, 2.0),
            ModKind::Lamp => vec3(0.6, 4.0, 0.6),
            ModKind::Pillar => vec3(1.0, 6.0, 1.0),
            ModKind::Shipyard => vec3(12.0, 1.0, 12.0),
            ModKind::Resonator => vec3(5.0, 10.0, 5.0),
        };
        if rot {
            vec3(s.z, s.y, s.x)
        } else {
            s
        }
    }
    pub fn breathable(self) -> bool {
        matches!(self, ModKind::Corridor | ModKind::Room | ModKind::Dome)
    }
}

pub struct Module {
    pub kind: ModKind,
    pub pos: Vec3,
    pub rot: bool,
}

impl Module {
    pub fn contains(&self, p: Vec3) -> bool {
        if self.kind == ModKind::Dome {
            return p.distance(self.pos) < 4.0 && p.y > self.pos.y - 4.0;
        }
        let h = self.kind.size(self.rot) / 2.0;
        let d = (p - self.pos).abs();
        d.x < h.x && d.y < h.y && d.z < h.z
    }
}

// ---------- транспорт ----------
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VehKind {
    Skat,
    Kit,
}

impl VehKind {
    pub fn name(self) -> &'static str {
        match self {
            VehKind::Skat => "Скат",
            VehKind::Kit => "Кит",
        }
    }
    pub fn max_hull(self) -> f32 {
        match self {
            VehKind::Skat => 100.0,
            VehKind::Kit => 400.0,
        }
    }
    pub fn limit(self) -> f32 {
        match self {
            VehKind::Skat => 200.0,
            VehKind::Kit => 420.0,
        }
    }
    pub fn reach(self) -> f32 {
        match self {
            VehKind::Skat => 5.0,
            VehKind::Kit => 13.0,
        }
    }
    pub fn half_h(self) -> f32 {
        match self {
            VehKind::Skat => 0.8,
            VehKind::Kit => 3.5,
        }
    }
}

pub struct Vehicle {
    pub kind: VehKind,
    pub pos: Vec3,
    pub vel: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub hull: f32,
    pub prop: f32,
}

impl Vehicle {
    pub fn fwd(&self) -> Vec3 {
        dir(self.yaw, self.pitch)
    }
}

pub fn dir(yaw: f32, pitch: f32) -> Vec3 {
    vec3(yaw.cos() * pitch.cos(), pitch.sin(), yaw.sin() * pitch.cos())
}

// ---------- рецепты ----------
pub struct Recipe {
    pub name: &'static str,
    pub desc: &'static str,
    pub cost: [u32; 6],
    pub vehicle: bool,
}

pub const RECIPES: [Recipe; 7] = [
    Recipe { name: "Большой баллон", desc: "кислород 45 → 90 с", cost: [2, 0, 1, 0, 0, 0], vehicle: false },
    Recipe { name: "Ласты", desc: "плаваешь на 40% быстрее", cost: [1, 2, 0, 0, 0, 0], vehicle: false },
    Recipe { name: "Костюм давления", desc: "предел 60 → 150 м", cost: [0, 3, 0, 2, 0, 0], vehicle: false },
    Recipe { name: "Мощный фонарь", desc: "фонарь бьёт вдвое дальше", cost: [0, 0, 2, 0, 1, 0], vehicle: false },
    Recipe { name: "Ребризер", desc: "кислород → 160 с", cost: [0, 0, 0, 3, 2, 0], vehicle: false },
    Recipe { name: "Скат", desc: "быстрая подлодка, до 200 м", cost: [2, 5, 2, 2, 0, 0], vehicle: true },
    Recipe { name: "Кит", desc: "подлодка-база, до 420 м", cost: [6, 12, 0, 6, 4, 0], vehicle: true },
];

// ---------- существа ----------
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CrKind {
    Fish,     // мелочь на отмелях и в лесу
    Drifter,  // парусник — мирный гигант стеблевого леса
    Clawback, // клешнеспин — хищник кристаллов
    Gloomfang, // мракозуб — ужас бездны
}

pub struct Creature {
    pub kind: CrKind,
    pub pos: Vec3,
    pub vel: Vec3,
    pub home: Vec3,
    pub segs: Vec<Vec3>,
    pub cooldown: f32,
    pub retreat: f32,
    pub t: f32,
    pub color: Color,
    pub size: f32,
}

pub struct Particle {
    pub pos: Vec3,
    pub vel: Vec3,
    pub life: f32,
    pub size: f32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Menu {
    None,
    Craft,
    Build,
}

#[derive(Clone, Copy, PartialEq)]
pub enum State {
    Play,
    Dead(&'static str),
    Won,
}

pub enum Target {
    Core(usize),
    Vehicle(usize),
    Node(usize),
}

pub struct Game {
    pub world: World,
    pub pos: Vec3,
    pub vel: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub o2: f32,
    pub max_o2: f32,
    pub hp: f32,
    pub swim_mul: f32,
    pub lamp_mul: f32,
    pub suit_limit: f32,
    pub inv: [u32; 6],
    pub cores: u32,
    pub upgrades: [bool; 5],
    pub modules: Vec<Module>,
    pub vehicles: Vec<Vehicle>,
    pub creatures: Vec<Creature>,
    pub parts: Vec<Particle>,
    pub pilot: Option<usize>,
    pub menu: Menu,
    pub placing: Option<ModKind>,
    pub build_rot: bool,
    pub msg: String,
    pub msg_t: f32,
    pub time: f32,
    pub state: State,
    pub hurt: f32,
    pub sonar_t: f32,
    pub sonar_cd: f32,
    pub raft: Vec3,
    pub help: bool,
    bubble_t: f32,
}

pub fn wrap_angle(a: f32) -> f32 {
    let mut a = a;
    while a > PI {
        a -= 2.0 * PI;
    }
    while a < -PI {
        a += 2.0 * PI;
    }
    a
}

impl Game {
    pub fn new() -> Game {
        let world = World::generate();
        let mut creatures = Vec::new();

        let fish_pal = [
            Color::new(1.0, 0.75, 0.2, 1.0),
            Color::new(1.0, 0.45, 0.55, 1.0),
            Color::new(0.4, 0.85, 1.0, 1.0),
            Color::new(0.75, 1.0, 0.4, 1.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
        ];
        let spawn_in = |b: Biome, world: &World| -> Vec3 {
            for _ in 0..500 {
                let a = gen_range(0.0, 2.0 * PI);
                let r = gen_range(20.0, WORLD_R - 30.0);
                let (x, z) = (a.cos() * r, a.sin() * r);
                if world.biome_at(x, z) == b {
                    let f = world.floor_y(x, z);
                    return vec3(x, f + gen_range(3.0f32, 12.0).min(-f - 2.0), z);
                }
            }
            vec3(0.0, -10.0, 0.0)
        };
        for i in 0..36 {
            let b = if i % 3 == 0 { Biome::Forest } else { Biome::Shallows };
            let home = spawn_in(b, &world);
            let color = fish_pal[gen_range(0, fish_pal.len())];
            for _ in 0..5 {
                let p = home + vec3(gen_range(-3.0, 3.0), gen_range(-1.0, 1.0), gen_range(-3.0, 3.0));
                creatures.push(Creature {
                    kind: CrKind::Fish,
                    pos: p,
                    vel: vec3(gen_range(-2.0, 2.0), 0.0, gen_range(-2.0, 2.0)),
                    home,
                    segs: vec![],
                    cooldown: 0.0,
                    retreat: 0.0,
                    t: gen_range(0.0, 6.0),
                    color,
                    size: gen_range(0.25, 0.45),
                });
            }
        }
        let add = |kind: CrKind, b: Biome, n: usize, color: Color, size: f32, segs: usize, creatures: &mut Vec<Creature>| {
            for _ in 0..n {
                let mut home = spawn_in(b, &world);
                if kind == CrKind::Drifter {
                    home.y = (home.y + 20.0).min(-12.0);
                }
                creatures.push(Creature {
                    kind,
                    pos: home,
                    vel: Vec3::ZERO,
                    home,
                    segs: vec![home; segs],
                    cooldown: 0.0,
                    retreat: 0.0,
                    t: gen_range(0.0, 20.0),
                    color,
                    size,
                });
            }
        };
        add(CrKind::Drifter, Biome::Forest, 6, Color::new(0.7, 0.75, 0.95, 1.0), 1.0, 0, &mut creatures);
        add(CrKind::Clawback, Biome::Crystal, 7, Color::new(0.25, 0.65, 0.7, 1.0), 1.0, 0, &mut creatures);
        add(CrKind::Gloomfang, Biome::Abyss, 4, Color::new(0.2, 0.1, 0.24, 1.0), 1.0, 18, &mut creatures);

        Game {
            world,
            pos: vec3(0.0, -1.0, 6.0),
            vel: Vec3::ZERO,
            yaw: -PI / 2.0,
            pitch: -0.25,
            o2: 45.0,
            max_o2: 45.0,
            hp: 100.0,
            swim_mul: 1.0,
            lamp_mul: 1.0,
            suit_limit: 60.0,
            inv: [0; 6],
            cores: 0,
            upgrades: [false; 5],
            modules: vec![],
            vehicles: vec![],
            creatures,
            parts: vec![],
            pilot: None,
            menu: Menu::None,
            placing: None,
            build_rot: false,
            msg: "Найди 4 ядра — по одному в каждом биоме — и построй Резонатор.".into(),
            msg_t: 7.0,
            time: 0.0,
            state: State::Play,
            hurt: 0.0,
            sonar_t: 0.0,
            sonar_cd: 0.0,
            raft: vec3(0.0, 0.0, 0.0),
            help: true,
            bubble_t: 0.0,
        }
    }

    pub fn say(&mut self, s: &str) {
        self.msg = s.to_string();
        self.msg_t = 3.5;
    }

    pub fn look(&self) -> Vec3 {
        dir(self.yaw, self.pitch)
    }

    // точка, откуда смотрит камера
    pub fn eye(&self) -> Vec3 {
        if let Some(i) = self.pilot {
            let v = &self.vehicles[i];
            let back = match v.kind {
                VehKind::Skat => 9.0,
                VehKind::Kit => 30.0,
            };
            let up = match v.kind {
                VehKind::Skat => 2.5,
                VehKind::Kit => 9.0,
            };
            let mut e = v.pos - self.look() * back + vec3(0.0, up, 0.0);
            let fy = self.world.floor_y(e.x, e.z) + 1.0;
            e.y = e.y.max(fy);
            e
        } else {
            self.pos
        }
    }

    pub fn cam_target(&self) -> Vec3 {
        if let Some(i) = self.pilot {
            self.vehicles[i].pos + vec3(0.0, 1.0, 0.0) + self.look() * 6.0
        } else {
            self.pos + self.look()
        }
    }

    // точка, где сейчас «тело» — игрок или его транспорт
    pub fn body(&self) -> Vec3 {
        match self.pilot {
            Some(i) => self.vehicles[i].pos,
            None => self.pos,
        }
    }

    pub fn depth_m(&self) -> f32 {
        (-self.body().y).max(0.0)
    }

    pub fn affordable(&self, cost: &[u32; 6]) -> bool {
        (0..6).all(|i| self.inv[i] >= cost[i])
    }

    fn pay(&mut self, cost: &[u32; 6]) {
        for i in 0..6 {
            self.inv[i] -= cost[i];
        }
    }

    pub fn powered(&self, p: Vec3) -> bool {
        self.modules
            .iter()
            .any(|m| m.kind == ModKind::Generator && m.pos.distance(p) < 45.0)
    }

    pub fn in_base(&self, p: Vec3) -> bool {
        self.modules.iter().any(|m| m.kind.breathable() && m.contains(p) && self.powered(m.pos))
    }

    pub fn near_shipyard(&self) -> Option<Vec3> {
        let b = self.body();
        self.modules
            .iter()
            .filter(|m| m.kind == ModKind::Shipyard && m.pos.distance(b) < 30.0)
            .map(|m| m.pos)
            .next()
    }

    pub fn craft(&mut self, i: usize) {
        let r = &RECIPES[i];
        if !r.vehicle && self.upgrades[i] {
            self.say("Уже сделано.");
            return;
        }
        let yard = self.near_shipyard();
        if r.vehicle && yard.is_none() {
            self.say("Транспорт собирается только у Верфи.");
            return;
        }
        if !self.affordable(&r.cost) {
            self.say("Не хватает ресурсов.");
            return;
        }
        let cost = r.cost;
        self.pay(&cost);
        match i {
            0 => self.max_o2 = self.max_o2.max(90.0),
            1 => self.swim_mul = 1.4,
            2 => self.suit_limit = 150.0,
            3 => self.lamp_mul = 2.0,
            4 => self.max_o2 = self.max_o2.max(160.0),
            _ => {}
        }
        if i < 5 {
            self.upgrades[i] = true;
        } else {
            let kind = if i == 5 { VehKind::Skat } else { VehKind::Kit };
            let p = yard.unwrap() + vec3(0.0, kind.half_h() + 2.0, 0.0);
            self.vehicles.push(Vehicle {
                kind,
                pos: p,
                vel: Vec3::ZERO,
                yaw: self.yaw,
                pitch: 0.0,
                hull: kind.max_hull(),
                prop: 0.0,
            });
            self.menu = Menu::None;
        }
        let s = format!("Готово: {}", r.name);
        self.say(&s);
    }

    // призрак постройки
    pub fn ghost(&self) -> Option<(ModKind, Vec3, Vec3, bool)> {
        let kind = self.placing?;
        let size = kind.size(self.build_rot);
        let p = self.pos + self.look() * (6.0 + size.x.max(size.z) * 0.7);
        let snap = |v: f32| (v / 2.0).round() * 2.0;
        let (x, z) = (snap(p.x), snap(p.z));
        let floor = self.world.floor_y(x, z);
        let y = p.y.round().max(floor + size.y / 2.0 - 0.3).min(-size.y / 2.0);
        let pos = vec3(x, y, z);
        let mut ok = self.affordable(&kind.cost());
        if kind == ModKind::Resonator && self.cores < 4 {
            ok = false;
        }
        if y > -size.y / 2.0 + 0.01 || floor > -size.y {
            ok = false; // слишком мелко
        }
        Some((kind, pos, size, ok))
    }

    pub fn place(&mut self) {
        let Some((kind, pos, _, ok)) = self.ghost() else { return };
        if !ok {
            if kind == ModKind::Resonator && self.cores < 4 {
                let s = format!("Резонатору нужны все ядра: {}/4", self.cores);
                self.say(&s);
            } else {
                self.say("Нельзя: не хватает ресурсов или слишком мелко.");
            }
            return;
        }
        let c = kind.cost();
        self.pay(&c);
        self.modules.push(Module { kind, pos, rot: self.build_rot });
        if kind == ModKind::Resonator {
            self.state = State::Won;
            return;
        }
        if !self.affordable(&kind.cost()) {
            self.placing = None;
        }
    }

    // модуль под прицелом (для разборки)
    pub fn aimed_module(&self) -> Option<usize> {
        let o = self.pos;
        let d = self.look();
        let mut best = None;
        let mut bt: f32 = 14.0;
        for (i, m) in self.modules.iter().enumerate() {
            let h = m.kind.size(m.rot) / 2.0;
            let (mn, mx) = (m.pos - h, m.pos + h);
            let mut t0 = 0.0f32;
            let mut t1: f32 = bt;
            let mut hit = true;
            for a in 0..3 {
                let (oa, da, lo, hi) = (o[a], d[a], mn[a], mx[a]);
                if da.abs() < 1e-5 {
                    if oa < lo || oa > hi {
                        hit = false;
                        break;
                    }
                } else {
                    let (mut ta, mut tb) = ((lo - oa) / da, (hi - oa) / da);
                    if ta > tb {
                        std::mem::swap(&mut ta, &mut tb);
                    }
                    t0 = t0.max(ta);
                    t1 = t1.min(tb);
                    if t0 > t1 {
                        hit = false;
                        break;
                    }
                }
            }
            if hit && t0 < bt {
                bt = t0;
                best = Some(i);
            }
        }
        best
    }

    pub fn deconstruct(&mut self) {
        if let Some(i) = self.aimed_module() {
            if self.modules[i].kind == ModKind::Resonator {
                return;
            }
            let k = self.modules.remove(i).kind;
            for r in 0..6 {
                self.inv[r] += k.cost()[r];
            }
            let s = format!("Разобрано: {}", k.name());
            self.say(&s);
        }
    }

    pub fn target(&self) -> Option<Target> {
        if self.pilot.is_some() {
            return None;
        }
        let p = self.pos;
        let f = self.look();
        for (i, c) in self.world.cores.iter().enumerate() {
            if !c.taken && c.pos.distance(p) < 5.0 {
                return Some(Target::Core(i));
            }
        }
        for (i, v) in self.vehicles.iter().enumerate() {
            if v.pos.distance(p) < v.kind.reach() {
                return Some(Target::Vehicle(i));
            }
        }
        let mut best = None;
        let mut bs = f32::MAX;
        for (i, n) in self.world.nodes.iter().enumerate() {
            if !n.alive {
                continue;
            }
            let d = n.pos - p;
            let dist = d.length();
            if dist > 5.5 {
                continue;
            }
            let facing = d.normalize_or_zero().dot(f);
            if facing < 0.5 && dist > 2.5 {
                continue;
            }
            let score = dist * (2.0 - facing);
            if score < bs {
                bs = score;
                best = Some(i);
            }
        }
        best.map(Target::Node)
    }

    pub fn interact(&mut self) {
        if let Some(i) = self.pilot {
            let v = &self.vehicles[i];
            let mut out = v.pos + vec3(0.0, v.kind.half_h() + 1.5, 0.0);
            out.y = out.y.min(-0.5);
            self.pos = out;
            self.vel = v.vel;
            self.pilot = None;
            self.say("Вышел наружу.");
            return;
        }
        match self.target() {
            Some(Target::Core(i)) => {
                self.world.cores[i].taken = true;
                self.cores += 1;
                let s = format!("Ядро: {} ({}/4)", self.world.cores[i].biome.name(), self.cores);
                self.say(&s);
            }
            Some(Target::Vehicle(i)) => {
                self.pilot = Some(i);
                let v = &self.vehicles[i];
                self.yaw = v.yaw;
                self.pitch = -0.1;
                self.placing = None;
                let s = format!("За штурвалом: {}", v.kind.name());
                self.say(&s);
            }
            Some(Target::Node(i)) => {
                let n = &mut self.world.nodes[i];
                n.alive = false;
                let k = n.kind;
                let at = n.pos;
                self.inv[k.idx()] += 1;
                let s = format!("+1 {}", k.name());
                self.say(&s);
                for _ in 0..8 {
                    self.parts.push(Particle {
                        pos: at,
                        vel: vec3(gen_range(-2.0, 2.0), gen_range(0.5, 3.0), gen_range(-2.0, 2.0)),
                        life: 1.2,
                        size: 0.12,
                    });
                }
            }
            None => {}
        }
    }

    pub fn sonar(&mut self) {
        if self.sonar_cd > 0.0 {
            return;
        }
        self.sonar_t = 7.0;
        self.sonar_cd = 9.0;
    }

    pub fn respawn(&mut self) {
        // возрождаемся в запитанном отсеке, если он есть, иначе на плоту
        let spot = self
            .modules
            .iter()
            .find(|m| m.kind == ModKind::Room && self.powered(m.pos))
            .map(|m| m.pos)
            .unwrap_or(self.raft + vec3(0.0, -1.0, 4.0));
        self.pos = spot;
        self.vel = Vec3::ZERO;
        self.pilot = None;
        self.hp = 100.0;
        self.o2 = self.max_o2;
        for v in self.inv.iter_mut() {
            *v /= 2;
        }
        self.state = State::Play;
        self.say("Ты очнулся. Половина груза потеряна.");
    }

    fn die(&mut self, why: &'static str) {
        if self.state == State::Play {
            self.state = State::Dead(why);
        }
    }

    pub fn update(&mut self, dt: f32, mv: Vec3, sprint: bool) {
        self.time += dt;
        self.msg_t -= dt;
        self.hurt = (self.hurt - dt).max(0.0);
        self.sonar_t -= dt;
        self.sonar_cd -= dt;

        match self.pilot {
            None => self.update_swim(dt, mv, sprint),
            Some(i) => self.update_vehicle(i, dt, mv, sprint),
        }

        // транспорт без пилота: тормозит и держится выше дна
        for (i, v) in self.vehicles.iter_mut().enumerate() {
            if Some(i) != self.pilot {
                v.vel *= (1.0 - 2.0 * dt).max(0.0);
                v.pos += v.vel * dt;
            }
            let fy = self.world.floor_y(v.pos.x, v.pos.z) + v.kind.half_h();
            if v.pos.y < fy {
                v.pos.y = fy;
                v.vel.y = v.vel.y.max(0.0);
            }
            if v.pos.y > -v.kind.half_h() * 0.5 {
                v.vel.y -= 12.0 * dt;
            }
            let r = vec2(v.pos.x, v.pos.z).length();
            if r > WORLD_R - 15.0 {
                let k = (WORLD_R - 15.0) / r;
                v.pos.x *= k;
                v.pos.z *= k;
            }
            v.prop += v.vel.length() * dt * 0.8;
        }

        // давление на корпуса
        let mut destroyed = None;
        for (i, v) in self.vehicles.iter_mut().enumerate() {
            if -v.pos.y > v.kind.limit() {
                v.hull -= 12.0 * dt;
            }
            if v.hull <= 0.0 {
                destroyed = Some(i);
            }
        }
        if let Some(i) = destroyed {
            let v = self.vehicles.remove(i);
            match self.pilot {
                Some(p) if p == i => {
                    self.pilot = None;
                    self.pos = v.pos;
                    self.say("Корпус разрушен! Плыви к поверхности!");
                }
                Some(p) if p > i => self.pilot = Some(p - 1),
                _ => {}
            }
        }

        self.update_creatures(dt);
        self.update_particles(dt);

        if self.hp <= 0.0 {
            self.die("Ты погиб.");
        }
    }

    fn update_swim(&mut self, dt: f32, mv: Vec3, sprint: bool) {
        let f = self.look();
        let right = f.cross(Vec3::Y).normalize_or_zero();
        let wish = (f * mv.z + right * mv.x + Vec3::Y * mv.y).normalize_or_zero();
        let max = 7.0 * self.swim_mul * if sprint { 1.5 } else { 1.0 };
        self.vel += wish * 35.0 * self.swim_mul * dt;
        self.vel *= (1.0 - 2.5 * dt).max(0.0);
        if self.vel.length() > max {
            self.vel = self.vel.normalize() * max;
        }
        if self.pos.y > -0.3 {
            self.vel.y -= 14.0 * dt; // над водой тянет вниз
        }
        self.pos += self.vel * dt;
        self.pos.y = self.pos.y.min(1.2);
        let fy = self.world.floor_y(self.pos.x, self.pos.z) + 1.0;
        if self.pos.y < fy {
            self.pos.y = fy;
            self.vel.y = self.vel.y.max(0.0);
        }
        let r = vec2(self.pos.x, self.pos.z).length();
        if r > WORLD_R - 12.0 {
            let k = (WORLD_R - 12.0) / r;
            self.pos.x *= k;
            self.pos.z *= k;
        }

        let surface = self.pos.y > -0.6;
        if surface || self.in_base(self.pos) {
            self.o2 = (self.o2 + 40.0 * dt).min(self.max_o2);
            self.hp = (self.hp + 4.0 * dt).min(100.0);
        } else {
            self.o2 -= dt * if sprint && wish.length() > 0.1 { 1.6 } else { 1.0 };
        }
        if self.o2 <= 0.0 {
            self.o2 = 0.0;
            self.hp -= 15.0 * dt;
            if self.hp <= 0.0 {
                self.die("Кислород кончился.");
            }
        }
        if self.depth_m() > self.suit_limit {
            self.hp -= 9.0 * dt;
            if self.hp <= 0.0 {
                self.die("Раздавило давлением. Костюм не для такой глубины.");
            }
        }

        self.bubble_t -= dt;
        if self.bubble_t <= 0.0 && !surface {
            self.bubble_t = if wish.length() > 0.1 { 0.12 } else { 0.6 };
            self.parts.push(Particle {
                pos: self.pos - f * 0.4 + vec3(0.0, 0.5, 0.0),
                vel: vec3(gen_range(-0.3, 0.3), gen_range(1.5, 2.5), gen_range(-0.3, 0.3)),
                life: 3.0,
                size: gen_range(0.03, 0.08),
            });
        }
    }

    fn update_vehicle(&mut self, i: usize, dt: f32, mv: Vec3, sprint: bool) {
        let (yaw, pitch) = (self.yaw, self.pitch);
        let v = &mut self.vehicles[i];
        match v.kind {
            VehKind::Skat => {
                // нос следует за взглядом
                v.yaw += wrap_angle(yaw - v.yaw) * (dt * 4.0).min(1.0);
                v.pitch += (pitch.clamp(-1.2, 1.2) - v.pitch) * (dt * 4.0).min(1.0);
                let f = v.fwd();
                let right = f.cross(Vec3::Y).normalize_or_zero();
                let wish = f * mv.z + right * mv.x * 0.6 + Vec3::Y * mv.y * 0.7;
                let max = if sprint { 30.0 } else { 22.0 };
                v.vel += wish * 28.0 * dt;
                v.vel *= (1.0 - 1.2 * dt).max(0.0);
                if v.vel.length() > max {
                    v.vel = v.vel.normalize() * max;
                }
            }
            VehKind::Kit => {
                // руль: A/D, тяга: W/S, балласт: Space/Ctrl
                v.yaw += mv.x * 0.55 * dt;
                v.pitch *= 1.0 - dt;
                let f = vec3(v.yaw.cos(), 0.0, v.yaw.sin());
                let wish = f * mv.z + Vec3::Y * mv.y * 0.5;
                let max = if sprint { 13.0 } else { 9.0 };
                v.vel += wish * 6.0 * dt;
                v.vel *= (1.0 - 0.6 * dt).max(0.0);
                if v.vel.length() > max {
                    v.vel = v.vel.normalize() * max;
                }
            }
        }
        v.pos += v.vel * dt;
        self.pos = v.pos;
        self.o2 = (self.o2 + 30.0 * dt).min(self.max_o2);
    }

    fn update_creatures(&mut self, dt: f32) {
        let body = self.body();
        let (bite_r, in_veh) = match self.pilot {
            Some(i) => (match self.vehicles[i].kind {
                VehKind::Skat => 4.0,
                VehKind::Kit => 10.0,
            }, true),
            None => (2.5, false),
        };
        let mut dmg = 0.0;
        let mut knock = Vec3::ZERO;
        let mut bitten_by = "";
        let world = &self.world;
        for c in &mut self.creatures {
            c.t += dt;
            c.cooldown -= dt;
            c.retreat -= dt;
            let to_p = body - c.pos;
            let d = to_p.length();
            let (target, speed, turn) = match c.kind {
                CrKind::Fish => {
                    let w = c.home
                        + vec3((c.t * 0.4).cos() * 6.0, (c.t * 0.7).sin() * 1.5, (c.t * 0.5).sin() * 6.0);
                    if d < 6.0 {
                        (c.pos - to_p * 3.0, 7.0, 5.0)
                    } else {
                        (w, 3.0, 2.0)
                    }
                }
                CrKind::Drifter => {
                    let w = c.home
                        + vec3((c.t * 0.05).cos() * 40.0, (c.t * 0.1).sin() * 4.0, (c.t * 0.05).sin() * 40.0);
                    (w, 2.2, 0.5)
                }
                CrKind::Clawback => {
                    if c.retreat > 0.0 {
                        (c.pos - to_p.normalize_or_zero() * 10.0, 10.0, 3.0)
                    } else if d < 35.0 {
                        (body, 12.5, 3.0)
                    } else {
                        let w = c.home + vec3((c.t * 0.3).cos() * 25.0, 0.0, (c.t * 0.25).sin() * 25.0);
                        (w, 4.0, 1.5)
                    }
                }
                CrKind::Gloomfang => {
                    if c.retreat > 0.0 {
                        (c.pos - to_p.normalize_or_zero() * 20.0, 14.0, 2.0)
                    } else if d < 75.0 && body.y < -150.0 {
                        (body, 18.5, 2.2)
                    } else {
                        let w = c.home
                            + vec3((c.t * 0.12).cos() * 60.0, (c.t * 0.2).sin() * 15.0, (c.t * 0.1).sin() * 60.0);
                        (w, 7.0, 1.0)
                    }
                }
            };
            let desired = (target - c.pos).normalize_or_zero() * speed;
            c.vel = c.vel.lerp(desired, (dt * turn).min(1.0));
            c.pos += c.vel * dt;
            let fy = world.floor_y(c.pos.x, c.pos.z) + 1.5;
            c.pos.y = c.pos.y.max(fy).min(-1.0);
            let r = vec2(c.pos.x, c.pos.z).length();
            if r > WORLD_R - 15.0 {
                let k = (WORLD_R - 15.0) / r;
                c.pos.x *= k;
                c.pos.z *= k;
            }
            if !c.segs.is_empty() {
                c.segs[0] = c.pos;
                for s in 1..c.segs.len() {
                    let dv = c.segs[s] - c.segs[s - 1];
                    if dv.length() > 1.6 {
                        c.segs[s] = c.segs[s - 1] + dv.normalize() * 1.6;
                    }
                }
            }
            let biter = matches!(c.kind, CrKind::Clawback | CrKind::Gloomfang);
            if biter && d < bite_r + 1.0 && c.cooldown <= 0.0 && c.retreat <= 0.0 {
                let amount = if c.kind == CrKind::Gloomfang { 30.0 } else { 18.0 };
                dmg += amount;
                knock += to_p.normalize_or_zero() * if in_veh { 6.0 } else { 12.0 };
                c.cooldown = 1.5;
                c.retreat = if c.kind == CrKind::Gloomfang { 4.0 } else { 2.5 };
                bitten_by = if c.kind == CrKind::Gloomfang { "Мракозуб" } else { "Клешнеспин" };
            }
        }
        if dmg > 0.0 {
            self.hurt = 0.5;
            match self.pilot {
                Some(i) => {
                    let v = &mut self.vehicles[i];
                    v.hull -= dmg;
                    v.vel += knock;
                    let s = format!("{} атакует {}! Корпус {:.0}%", bitten_by, v.kind.name(), v.hull / v.kind.max_hull() * 100.0);
                    self.say(&s);
                }
                None => {
                    self.hp -= dmg;
                    self.vel += knock;
                    let s = format!("{} кусается!", bitten_by);
                    self.say(&s);
                    if self.hp <= 0.0 {
                        self.die(if bitten_by == "Мракозуб" { "Тебя сожрал мракозуб." } else { "Клешнеспин разделал тебя." });
                    }
                }
            }
        }
    }

    fn update_particles(&mut self, dt: f32) {
        let cam = self.eye();
        // пузыри из жерл поблизости
        for f in &self.world.flora {
            if f.kind == FloraKind::Vent && f.pos.distance(cam) < 70.0 && gen_range(0.0, 1.0) < dt * 6.0 {
                self.parts.push(Particle {
                    pos: f.pos + vec3(gen_range(-0.5, 0.5), 2.6 * f.scale, gen_range(-0.5, 0.5)),
                    vel: vec3(0.0, gen_range(2.0, 4.0), 0.0),
                    life: 5.0,
                    size: gen_range(0.08, 0.25),
                });
            }
        }
        let t = self.time;
        for p in &mut self.parts {
            p.pos += p.vel * dt;
            p.pos.x += (t * 2.0 + p.pos.y).sin() * 0.3 * dt;
            p.life -= dt;
        }
        self.parts.retain(|p| p.life > 0.0 && p.pos.y < 0.0);
        if self.parts.len() > 600 {
            let n = self.parts.len() - 600;
            self.parts.drain(0..n);
        }
    }
}
