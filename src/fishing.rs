use crate::{
    Session,
    game::Actor,
    ui::{Chat, Menu, Page},
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{io, path::PathBuf};

pub const SHOP_DOOR: f32 = 965.0;
pub const SHOP_EXIT: f32 = 64.0;
pub const COUNTER: f32 = 270.0;
pub const PIER_START: f32 = 1304.0;
pub const PIER_END: f32 = 1360.0;
const ROD_PRICE: u32 = 45;
const HOOK_PRICE: u32 = 15;
const BAIT_PRICE: u32 = 10;
pub const FISH: [(&str, u32); 4] = [
    ("Sardine", 8),
    ("Mackerel", 14),
    ("Sea bass", 24),
    ("Golden bream", 40),
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
// shortcut: local per-nickname progress; make transactions authoritative before adding trading.
pub struct Progress {
    version: u32,
    pub coins: u32,
    pub rod: bool,
    pub hook: bool,
    pub bait: u32,
    pub catches: [u32; 4],
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            version: 1,
            coins: 100,
            rod: false,
            hook: false,
            bait: 0,
            catches: [0; 4],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Rod,
    Hook,
    Bait,
    Sell,
    Close,
}

impl Progress {
    pub fn buy(&mut self, item: Action) -> Result<String, &'static str> {
        let (price, description) = match item {
            Action::Rod if !self.rod => (ROD_PRICE, "Bamboo rod equipped."),
            Action::Hook if !self.hook => (HOOK_PRICE, "Barbless hook equipped."),
            Action::Rod | Action::Hook => return Err("You already own this tackle."),
            Action::Bait if self.bait <= 994 => (BAIT_PRICE, "Five worms added to your bait tin."),
            Action::Bait => return Err("Your bait tin is full."),
            _ => return Err("Choose some tackle first."),
        };
        if self.coins < price {
            return Err("Not enough coins. Sell your catch at the counter.");
        }
        self.coins -= price;
        match item {
            Action::Rod => self.rod = true,
            Action::Hook => self.hook = true,
            Action::Bait => self.bait += 5,
            _ => unreachable!(),
        }
        Ok(description.into())
    }

    pub fn cast(&mut self) -> Result<(), &'static str> {
        if !self.rod || !self.hook {
            return Err("Buy a rod and a hook at Tide & Tackle first.");
        }
        if self.bait == 0 {
            return Err("Out of bait. Visit Tide & Tackle for more worms.");
        }
        if self.catches.iter().any(|&count| count >= 999) {
            return Err("Your creel is full. Sell your catch first.");
        }
        self.bait -= 1;
        Ok(())
    }

    pub fn value(&self) -> u32 {
        self.catches
            .iter()
            .zip(FISH)
            .map(|(n, (_, price))| n * price)
            .sum()
    }

    pub fn sell(&mut self) -> Result<u32, &'static str> {
        let value = self.value();
        if value == 0 {
            return Err("Your creel is empty. Try the pier to the east.");
        }
        self.coins = self
            .coins
            .checked_add(value)
            .ok_or("Your wallet is full.")?;
        self.catches = [0; 4];
        Ok(value)
    }
}

pub fn profile_path(name: &str) -> io::Result<PathBuf> {
    let base = if let Some(path) = std::env::var_os("YAPSHIRE_SAVE_DIR") {
        PathBuf::from(path)
    } else if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| io::Error::other("APPDATA is not available"))?
            .join("Yapshire")
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| io::Error::other("HOME is not available"))?
            .join("Library/Application Support/Yapshire")
    } else if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
        PathBuf::from(path).join("yapshire")
    } else {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| io::Error::other("HOME is not available"))?
            .join(".local/share/yapshire")
    };
    // Hex keeps arbitrary nicknames inside the save directory on every platform.
    let name = name
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    Ok(base.join(format!("fishing-{name}.json")))
}

fn load_progress(path: &std::path::Path) -> io::Result<Progress> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Progress::default()),
        Err(error) => return Err(error),
    };
    let progress: Progress = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
    if progress.version != 1 || progress.bait > 999 || progress.catches.iter().any(|&n| n > 999) {
        return Err(io::Error::other("Unsupported or invalid fishing save"));
    }
    Ok(progress)
}

fn save_progress(path: &std::path::Path, progress: &Progress) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    let result = (|| {
        use std::io::Write;
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(&serde_json::to_vec_pretty(progress).map_err(io::Error::other)?)?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

#[derive(Clone, Debug)]
pub struct Fight {
    pub elapsed: f32,
    pub tension: f32,
    pub landed: f32,
    pub fish: usize,
    pub pull: f32,
}

impl Fight {
    fn new(fish: usize) -> Self {
        Self {
            elapsed: 0.0,
            tension: 0.22,
            landed: 0.2,
            fish,
            pull: 0.0,
        }
    }

    pub fn tick(&mut self, dt: f32, reel: bool) -> Option<bool> {
        self.elapsed += dt;
        let dash = (self.elapsed + self.fish as f32 * 0.4).rem_euclid(4.2) < 1.15;
        self.pull = if dash {
            1.0
        } else {
            0.18 + (self.elapsed * 2.3).sin().abs() * 0.2
        };
        self.tension = (self.tension
            + if reel {
                (0.12 + self.pull * (0.5 + self.fish as f32 * 0.035)) * dt
            } else {
                -0.38 * dt
            })
        .clamp(0.0, 1.0);
        self.landed += if reel {
            (0.16 - self.pull * 0.065) * dt
        } else {
            -0.035 * dt
        };
        if self.tension >= 1.0 || self.landed <= 0.0 || self.elapsed >= 35.0 {
            Some(false)
        } else if self.landed >= 1.0 {
            self.landed = 1.0;
            Some(true)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Default)]
pub enum Stage {
    #[default]
    Idle,
    Waiting(f32),
    Bite(f32),
    Reeling(Fight),
    Result {
        fish: Option<usize>,
        message: String,
    },
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Panel {
    #[default]
    None,
    Shop,
    Bag,
}

#[derive(Resource)]
pub struct Fishing {
    pub progress: Progress,
    pub stage: Stage,
    pub panel: Panel,
    pub indoors: bool,
    pub command: Option<Action>,
    pub dirty: bool,
    pub notice: String,
    pub anim_time: f32,
    pub(crate) notice_time: f32,
    profile: Option<PathBuf>,
    pub(crate) save_error: String,
    loaded_for: String,
    was_playing: bool,
}

impl Default for Fishing {
    fn default() -> Self {
        Self {
            progress: Progress::default(),
            stage: Stage::Idle,
            panel: Panel::None,
            indoors: false,
            command: None,
            dirty: true,
            notice: String::new(),
            anim_time: 0.0,
            notice_time: 0.0,
            profile: None,
            save_error: String::new(),
            loaded_for: String::new(),
            was_playing: false,
        }
    }
}

impl Fishing {
    pub fn active(&self) -> bool {
        matches!(
            self.stage,
            Stage::Waiting(_) | Stage::Bite(_) | Stage::Reeling(_)
        )
    }

    pub fn modal(&self) -> bool {
        self.panel != Panel::None || !matches!(self.stage, Stage::Idle)
    }

    fn say(&mut self, message: impl Into<String>) {
        self.notice = message.into();
        self.notice_time = 6.0;
    }

    fn save(&mut self) {
        if !self.save_error.is_empty() {
            return;
        }
        if let Some(path) = &self.profile {
            if let Err(error) = save_progress(path, &self.progress) {
                warn!("Cannot save fishing progress: {error}");
                self.save_error = "Progress could not be saved. Check the save folder.".into();
            }
        }
    }

    fn cancel(&mut self) {
        if self.active() {
            self.say("Cast cancelled. The worm was already used.");
        }
        self.stage = Stage::Idle;
        self.panel = Panel::None;
        self.dirty = true;
    }
}

pub fn update(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    menu: Res<Menu>,
    chat: Res<Chat>,
    session: Res<Session>,
    window: Single<&Window>,
    mut fishing: ResMut<Fishing>,
    mut actors: Query<&mut Actor>,
) {
    let playing = menu.page == Page::Playing && session.connected;
    if playing != fishing.was_playing {
        fishing.cancel();
        fishing.indoors = false;
        fishing.was_playing = playing;
        fishing.dirty = true;
    }
    if !playing {
        fishing.command = None;
        for mut actor in &mut actors {
            if Some(actor.player.id) == session.you {
                actor.player.fishing = false;
                if actor.player.indoors {
                    actor.player.indoors = false;
                    actor.teleport(Vec2::new(SHOP_DOOR, 0.0));
                }
            }
        }
        return;
    }
    if fishing.loaded_for != menu.name {
        fishing.loaded_for = menu.name.clone();
        match profile_path(&menu.name).and_then(|path| load_progress(&path).map(|p| (path, p))) {
            Ok((path, progress)) => {
                fishing.profile = Some(path);
                fishing.progress = progress;
                fishing.save_error.clear();
                fishing.say("Welcome! Find Tide & Tackle east of the coffee shop.");
            }
            Err(error) => {
                warn!("Cannot load fishing progress: {error}");
                fishing.profile = None;
                fishing.save_error = "Save unavailable. Purchases and fishing are disabled.".into();
            }
        }
        fishing.dirty = true;
    }
    let Some(mut actor) = actors.iter_mut().find(|a| Some(a.player.id) == session.you) else {
        return;
    };
    let dt = time.delta_secs().min(0.05);
    fishing.anim_time += dt;
    fishing.notice_time = (fishing.notice_time - dt).max(0.0);
    let focused =
        window.focused || (cfg!(debug_assertions) && std::env::var_os("YAPSHIRE_SMOKE").is_some());
    let input = focused && !chat.open;
    let mut command = fishing.command.take();
    if input && fishing.panel == Panel::Shop {
        for (key, action) in [
            (KeyCode::Digit1, Action::Rod),
            (KeyCode::Digit2, Action::Hook),
            (KeyCode::Digit3, Action::Bait),
            (KeyCode::Digit4, Action::Sell),
        ] {
            if keys.just_pressed(key) {
                command = Some(action);
            }
        }
    }
    if let Some(action) = command {
        if action == Action::Close {
            if fishing.modal() {
                fishing.cancel();
            } else if fishing.indoors {
                fishing.indoors = false;
                actor.teleport(Vec2::new(SHOP_DOOR, 0.0));
                fishing.dirty = true;
            }
        } else if fishing.panel == Panel::Shop && fishing.save_error.is_empty() {
            let message = if action == Action::Sell {
                fishing
                    .progress
                    .sell()
                    .map(|n| format!("Sold your catch for {n} coins. Thank you!"))
            } else {
                fishing.progress.buy(action)
            };
            match message {
                Ok(message) => {
                    fishing.say(message);
                    fishing.save();
                }
                Err(message) => fishing.say(message),
            }
            fishing.dirty = true;
        }
    }
    if input && keys.just_pressed(KeyCode::KeyI) && !fishing.active() {
        fishing.panel = if fishing.panel == Panel::Bag {
            Panel::None
        } else {
            Panel::Bag
        };
        fishing.stage = Stage::Idle;
        fishing.dirty = true;
    }
    if input && keys.just_pressed(KeyCode::KeyE) && !fishing.modal() && actor.position.y < 1.0 {
        let x = actor.position.x;
        if !fishing.indoors && (x - SHOP_DOOR).abs() < 28.0 {
            fishing.indoors = true;
            actor.teleport(Vec2::new(SHOP_EXIT + 32.0, 0.0));
            fishing.say("Welcome to Tide & Tackle. Walk up to the counter.");
        } else if fishing.indoors && (x - SHOP_EXIT).abs() < 32.0 {
            fishing.indoors = false;
            actor.teleport(Vec2::new(SHOP_DOOR, 0.0));
        } else if fishing.indoors && (x - COUNTER).abs() < 58.0 {
            fishing.panel = Panel::Shop;
            if fishing.progress.rod
                && fishing.progress.hook
                && fishing.progress.bait == 0
                && fishing.progress.coins < BAIT_PRICE
                && fishing.progress.value() == 0
                && fishing.save_error.is_empty()
            {
                fishing.progress.bait = 1;
                fishing.say("A worm on the house. Everyone deserves another cast.");
                fishing.save();
            }
        } else if !fishing.indoors && x >= PIER_START && fishing.save_error.is_empty() {
            match fishing.progress.cast() {
                Ok(()) => {
                    fishing.stage = Stage::Waiting(1.8 + rand::random::<f32>() * 2.4);
                    fishing.anim_time = 0.0;
                    actor.player.facing = false;
                    fishing.save();
                }
                Err(error) => fishing.say(error),
            }
        }
        fishing.dirty = true;
    }
    let hooked = input && keys.just_pressed(KeyCode::Space);
    let reeling = input && keys.pressed(KeyCode::Space);
    let mut result = None;
    match &mut fishing.stage {
        Stage::Waiting(left) => {
            *left -= dt;
            if *left <= 0.0 {
                fishing.stage = Stage::Bite(1.5);
                fishing.dirty = true;
            }
        }
        Stage::Bite(left) => {
            *left -= dt;
            if hooked {
                let roll = rand::random::<u32>() % 100;
                let fish = if roll < 45 {
                    0
                } else if roll < 76 {
                    1
                } else if roll < 94 {
                    2
                } else {
                    3
                };
                fishing.stage = Stage::Reeling(Fight::new(fish));
                fishing.dirty = true;
            } else if *left <= 0.0 {
                result = Some((false, 0, "Too late! The fish took the bait."));
            }
        }
        Stage::Reeling(fight) => {
            if let Some(won) = fight.tick(dt, reeling) {
                let reason = if fight.tension >= 1.0 {
                    "Too much tension! The fish escaped."
                } else {
                    "The line went slack. The fish escaped."
                };
                result = Some((won, fight.fish, reason));
            }
        }
        _ => {}
    }
    if let Some((won, fish, reason)) = result {
        let message = if won {
            fishing.progress.catches[fish] += 1;
            fishing.save();
            format!(
                "{} landed! Worth {} coins at the shop.",
                FISH[fish].0, FISH[fish].1
            )
        } else {
            reason.into()
        };
        fishing.stage = Stage::Result {
            fish: won.then_some(fish),
            message,
        };
        fishing.anim_time = 0.0;
        fishing.dirty = true;
    }
    actor.player.indoors = fishing.indoors;
    actor.player.fishing = fishing.active();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wallet_buys_tackle_casts_and_sells_without_double_charging() {
        let mut p = Progress::default();
        assert!(p.cast().is_err());
        for action in [Action::Rod, Action::Hook, Action::Bait] {
            p.buy(action).unwrap();
        }
        assert_eq!((p.coins, p.bait), (30, 5));
        assert!(p.buy(Action::Rod).is_err());
        assert_eq!(p.coins, 30);
        for _ in 0..5 {
            p.cast().unwrap();
        }
        assert!(p.cast().is_err());
        p.catches = [2, 1, 0, 1];
        assert_eq!(p.sell(), Ok(70));
        assert_eq!(p.coins, 100);
        assert!(p.sell().is_err());
        p.coins = 9;
        let before = p.clone();
        assert!(p.buy(Action::Bait).is_err());
        assert_eq!(p, before);
    }

    #[test]
    fn holding_forever_and_never_reeling_lose_but_controlled_reeling_wins() {
        for fish in 0..4 {
            for fps in [30, 60, 144] {
                for strategy in 0..3 {
                    let mut fight = Fight::new(fish);
                    let mut outcome = None;
                    for _ in 0..fps * 36 {
                        let reel = match strategy {
                            0 => true,
                            1 => false,
                            _ => fight.tension < 0.55,
                        };
                        outcome = fight.tick(1.0 / fps as f32, reel);
                        if outcome.is_some() {
                            break;
                        }
                    }
                    assert_eq!(
                        outcome,
                        Some(strategy == 2),
                        "fish={fish} fps={fps} strategy={strategy}"
                    );
                }
            }
        }
    }

    #[test]
    fn save_roundtrip_preserves_wallet_and_rejects_corrupt_or_future_data() {
        let dir = std::env::temp_dir().join(format!("yapshire-fishing-{}", rand::random::<u64>()));
        let path = dir.join("profile.json");
        let mut p = load_progress(&path).unwrap();
        p.buy(Action::Rod).unwrap();
        save_progress(&path, &p).unwrap();
        assert_eq!(load_progress(&path).unwrap(), p);
        p.buy(Action::Hook).unwrap();
        save_progress(&path, &p).unwrap();
        assert_eq!(load_progress(&path).unwrap(), p);
        std::fs::write(&path, b"{broken").unwrap();
        assert!(load_progress(&path).is_err());
        p.version = 99;
        save_progress(&path, &p).unwrap();
        assert!(load_progress(&path).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
