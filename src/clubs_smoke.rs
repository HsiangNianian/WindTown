//! Opt-in native acceptance against two real, isolated local servers.
use crate::{
    Session,
    clubs::{Action, Browser, SavedClub},
    i18n::{I18n, Language},
    network::{self, Link, Mode},
    settings::Settings,
    ui::{self, Field, Menu, Page},
};
use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
    window::WindowCloseRequested,
};
use std::{
    net::TcpListener,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

struct Town {
    address: String,
    server: yapshire_server::Server,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Town {
    fn new(name: &str, code: &str, capacity: usize, password: &str) -> Self {
        let server = yapshire_server::Server::new(
            yapshire_server::Config {
                name: name.into(),
                room_code: code.into(),
                max_players: capacity,
                max_rooms: 8,
                ..default()
            },
            yapshire_shared::World::bundled(),
            password,
        )
        .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("ws://{}", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let thread = server.clone().spawn(listener, stop.clone()).unwrap();
        Self {
            address,
            server,
            stop,
            thread: Some(thread),
        }
    }
}
impl Drop for Town {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap();
    }
}

#[derive(Resource, Default)]
pub(crate) struct Check {
    stage: u8,
    since: f32,
    captured: bool,
    towns: Vec<Town>,
    visitors: Vec<Link>,
    harbor: u64,
    offline: u64,
    tea: u64,
}
impl Drop for Check {
    fn drop(&mut self) {
        self.visitors.clear();
        self.towns.clear();
    }
}

#[derive(SystemParam)]
pub(crate) struct Controls<'w, 's> {
    buttons: Query<'w, 's, (&'static ui::Action, &'static mut Interaction)>,
    mouse: ResMut<'w, ButtonInput<MouseButton>>,
    ime: MessageWriter<'w, Ime>,
    window: Single<'w, 's, (Entity, &'static mut Window)>,
    close: MessageWriter<'w, WindowCloseRequested>,
}
impl Controls<'_, '_> {
    fn press(&mut self, wanted: Action) {
        let (_, mut interaction) = self
            .buttons
            .iter_mut()
            .find(|(action, _)| matches!(action, ui::Action::Club(value) if *value == wanted))
            .unwrap_or_else(|| panic!("Missing Club action {wanted:?}"));
        *interaction = Interaction::Pressed;
        self.mouse.press(MouseButton::Left);
    }
    fn type_field(&mut self, field: Field, value: &str) {
        let (_, mut interaction) = self
            .buttons
            .iter_mut()
            .find(|(action, _)| matches!(action, ui::Action::Focus(value) if *value == field))
            .expect("Missing Club field");
        *interaction = Interaction::Pressed;
        self.mouse.press(MouseButton::Left);
        self.ime.write(Ime::Commit {
            window: self.window.0,
            value: value.into(),
        });
    }
}

pub(crate) fn drive(
    mut commands: Commands,
    time: Res<Time>,
    mut check: ResMut<Check>,
    mut menu: ResMut<Menu>,
    mut browser: ResMut<Browser>,
    mut settings: ResMut<Settings>,
    mut i18n: ResMut<I18n>,
    session: Res<Session>,
    mut controls: Controls,
    texts: Query<&Text>,
) {
    if std::env::var("YAPSHIRE_SMOKE").as_deref() != Ok("clubs") {
        return;
    }
    for name in [
        "YAPSHIRE_SETTINGS_DIR",
        "YAPSHIRE_MAP_DIR",
        "YAPSHIRE_SAVE_DIR",
    ] {
        assert!(
            std::env::var_os(name).is_some(),
            "Set {name} for isolated acceptance"
        );
    }
    controls.mouse.release(MouseButton::Left);
    controls.window.1.set_physical_cursor_position(None);
    let now = time.elapsed_secs();
    assert!(
        now < 120.0,
        "Club acceptance timed out at stage {}: {}",
        check.stage,
        menu.status
    );
    if now - check.since < if check.stage == 0 { 3.0 } else { 0.8 } {
        return;
    }
    let capture = |commands: &mut Commands, check: &mut Check, name: &str| {
        if check.captured {
            return true;
        }
        std::fs::create_dir_all("artifacts/clubs").unwrap();
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(format!("artifacts/clubs/{name}.png")));
        check.captured = true;
        check.since = now;
        false
    };
    let ready = |id| {
        browser
            .clubs
            .iter()
            .find(|club| club.saved.id == id)
            .is_some_and(|club| club.snapshot.is_some())
    };
    let advance = match check.stage {
        0 => {
            check
                .towns
                .push(Town::new("Harbor Square", "HARBOR01", 4, ""));
            check
                .towns
                .push(Town::new("茶馆", "TEAHOUSE", 8, "teahouse-password"));
            let harbor = SavedClub::new("Harbor Club", &check.towns[0].address).unwrap();
            let offline = SavedClub::new("远方的朋友 · Offline", "ws://127.0.0.1:9").unwrap();
            check.harbor = harbor.id;
            check.offline = offline.id;
            let saved = vec![harbor.clone(), offline];
            settings.save_clubs(saved.clone()).unwrap();
            *browser = Browser::new(saved, &harbor.server);
            menu.select_server(&harbor.server, "");
            menu.name = "Club QA".into();
            i18n.language = Language::English;
            menu.go(Page::Cloud);
            true
        }
        1 if ready(check.harbor) && browser.clubs.iter().any(|c| c.error.is_some()) => {
            if !capture(&mut commands, &mut check, "lobby-en") {
                return;
            }
            controls.press(Action::Add);
            true
        }
        2 => {
            controls.type_field(Field::ClubAlias, "朋友的茶馆");
            true
        }
        3 => {
            controls.type_field(Field::ClubServer, &check.towns[1].address);
            true
        }
        4 => {
            controls.type_field(Field::ClubPassword, "teahouse-password");
            true
        }
        5 => {
            assert!(!texts.iter().any(|text| text.contains("teahouse-password")));
            if !capture(&mut commands, &mut check, "add-club-en") {
                return;
            }
            controls.press(Action::Save);
            true
        }
        6 if browser.clubs.len() == 3 && menu.club_editor.is_none() => {
            check.tea = browser.selected.unwrap();
            if !ready(check.tea) {
                return;
            }
            assert_eq!(browser.selected().unwrap().saved.alias, "朋友的茶馆");
            assert!(
                browser
                    .selected()
                    .unwrap()
                    .snapshot
                    .as_ref()
                    .unwrap()
                    .latency_ms
                    .is_some()
            );
            if !capture(&mut commands, &mut check, "multiple-clubs-en") {
                return;
            }
            i18n.language = Language::Chinese;
            true
        }
        7 => {
            if !capture(&mut commands, &mut check, "lobby-zh") {
                return;
            }
            controls.press(Action::Edit(check.tea));
            true
        }
        8 => {
            controls.type_field(Field::ClubAlias, "周末茶会 {name}");
            true
        }
        9 => {
            if !capture(&mut commands, &mut check, "edit-club-zh") {
                return;
            }
            controls.press(Action::Save);
            true
        }
        10 => {
            assert_eq!(browser.selected().unwrap().saved.alias, "周末茶会 {name}");
            assert_eq!(browser.selected.unwrap(), check.tea);
            controls.press(Action::Select(check.offline));
            true
        }
        11 => {
            controls.press(Action::Remove(check.offline));
            true
        }
        12 => {
            if !capture(&mut commands, &mut check, "remove-club-zh") {
                return;
            }
            controls.press(Action::ConfirmRemove(check.offline));
            true
        }
        13 => {
            assert_eq!(browser.clubs.len(), 2);
            let address = check.towns[0].address.clone();
            for title in [
                "Dockside bench",
                "Garden friends",
                "Evening walk",
                "Fishing together",
            ] {
                check.visitors.push(network::start_with_options(
                    Mode::HostCloud {
                        server: address.clone(),
                        room_name: title.into(),
                    },
                    "Fixture guest".into(),
                    yapshire_shared::World::bundled(),
                    String::new(),
                ));
            }
            true
        }
        14 if browser
            .clubs
            .iter()
            .find(|c| c.saved.id == check.harbor)
            .and_then(|c| c.snapshot.as_ref())
            .is_some_and(|s| s.rooms.len() == 5) =>
        {
            // No refresh click: this observes the eight-second automatic refresh.
            let snapshot = browser
                .clubs
                .iter()
                .find(|c| c.saved.id == check.harbor)
                .unwrap()
                .snapshot
                .as_ref()
                .unwrap();
            assert_eq!(snapshot.rooms.iter().map(|r| r.players).sum::<usize>(), 4);
            if !capture(&mut commands, &mut check, "rooms-auto-refreshed-zh") {
                return;
            }
            controls.press(Action::ScrollDown);
            true
        }
        15 => {
            assert!(browser.scroll > 0.0);
            if !capture(&mut commands, &mut check, "rooms-scrolled-zh") {
                return;
            }
            controls.press(Action::Select(check.tea));
            true
        }
        16 => {
            controls.press(Action::ScrollUp);
            true
        }
        17 => {
            controls.press(Action::Join(check.harbor, *b"HARBOR01"));
            true
        }
        18 if session.connected => {
            assert_eq!(menu.server, check.towns[0].address);
            assert!(menu.server_password.is_empty());
            assert_eq!(check.towns[0].server.player_count("HARBOR01"), 1);
            assert_eq!(check.towns[1].server.player_count("TEAHOUSE"), 0);
            if !capture(&mut commands, &mut check, "joined-correct-club") {
                return;
            }
            menu.leave = true;
            menu.go(Page::Cloud);
            true
        }
        19 if !session.connected && check.towns[0].server.player_count("HARBOR01") == 0 => {
            check.visitors.clear();
            controls.press(Action::Refresh);
            true
        }
        20 if browser.clubs.iter().all(|c| {
            c.snapshot
                .as_ref()
                .is_some_and(|s| s.rooms.len() == 1 && s.rooms[0].players == 0)
        }) =>
        {
            let path = std::path::PathBuf::from(std::env::var_os("YAPSHIRE_SETTINGS_DIR").unwrap())
                .join("settings.json");
            let json = std::fs::read_to_string(&path).unwrap();
            assert!(!json.contains("password"));
            let loaded = Settings::from_path(Ok(path)).0;
            assert_eq!(loaded.clubs().len(), 2);
            assert_eq!(loaded.clubs()[1].alias, "周末茶会 {name}");
            std::fs::write("artifacts/clubs/native-acceptance.json", serde_json::to_vec_pretty(&serde_json::json!({
                "passed": true, "source": "native Bevy UI with two real Rust servers",
                "checks": ["add via IME", "edit alias", "remove confirmation", "password masking and isolation",
                    "English and Chinese", "automatic refresh and actual player counts", "real ping", "scrolling",
                    "join the clicked Club", "leave and temporary-room cleanup", "persistence without passwords"],
                "clubs": loaded.clubs(),
            })).unwrap()).unwrap();
            info!(
                "CLUBS NATIVE PASS: CRUD, persistence, i18n, scroll, real ping, automatic room updates and correct server joining"
            );
            let window = controls.window.0;
            controls.close.write(WindowCloseRequested { window });
            true
        }
        _ => false,
    };
    if advance {
        check.captured = false;
        check.stage += 1;
        check.since = now;
    }
}
