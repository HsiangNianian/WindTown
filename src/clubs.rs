//! Saved server subscriptions and their independent, bounded lobby refreshes.
use crate::{
    i18n::{Message, tr},
    network::{self, ClubSnapshot},
    settings::Settings,
    ui::{Field, Menu, Page},
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

pub const MAX_CLUBS: usize = 32;
const MAX_SCANS: usize = 4;
const REFRESH_SECONDS: f64 = 8.0;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedClub {
    #[serde(default = "rand::random")]
    pub id: u64,
    pub alias: String,
    pub server: String,
}

impl SavedClub {
    pub fn new(alias: &str, server: &str) -> Result<Self, Message> {
        let server = network::normalize_server(server)
            .map_err(|error| tr("menu.server_invalid").arg("error", error))?;
        let alias = network::clean(alias, 32);
        Ok(Self {
            id: rand::random(),
            alias: if alias.is_empty() {
                network::clean(&server, 32)
            } else {
                alias
            },
            server,
        })
    }

    pub fn official() -> Self {
        Self::new("Yapshire 小镇", network::DEFAULT_SERVER.trim()).unwrap()
    }
}

pub fn restore(saved: Option<Vec<SavedClub>>, previous_server: &str) -> Vec<SavedClub> {
    let mut clubs = saved.unwrap_or_else(|| {
        let mut clubs = vec![SavedClub::official()];
        if previous_server != network::DEFAULT_SERVER.trim() {
            if let Ok(club) = SavedClub::new("", previous_server) {
                clubs.push(club);
            }
        }
        clubs
    });
    let mut addresses = HashSet::new();
    let mut ids = HashSet::new();
    clubs.retain_mut(|club| {
        let Ok(mut cleaned) = SavedClub::new(&club.alias, &club.server) else {
            return false;
        };
        if network::LEGACY_SERVERS.contains(&cleaned.server.as_str()) {
            cleaned.server = network::DEFAULT_SERVER.trim().into();
        }
        if !addresses.insert(cleaned.server.clone()) {
            return false;
        }
        while !ids.insert(club.id) {
            club.id = rand::random();
        }
        club.alias = cleaned.alias;
        club.server = cleaned.server;
        true
    });
    clubs.truncate(MAX_CLUBS);
    clubs
}

pub struct Club {
    pub saved: SavedClub,
    pub snapshot: Option<ClubSnapshot>,
    pub error: Option<Message>,
    pub expanded: bool,
    pub pending: Option<u64>,
    password: String,
    generation: u64,
    next_scan: f64,
}

impl Club {
    fn new(saved: SavedClub) -> Self {
        Self {
            saved,
            snapshot: None,
            error: None,
            expanded: true,
            pending: None,
            password: String::new(),
            generation: 0,
            next_scan: 0.0,
        }
    }
    fn invalidate(&mut self) {
        self.generation += 1;
        self.pending = None;
        self.snapshot = None;
        self.error = None;
        self.next_scan = 0.0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Add,
    Edit(u64),
    Save,
    Cancel,
    Select(u64),
    Toggle(u64),
    Remove(u64),
    ConfirmRemove(u64),
    CancelRemove,
    Join(u64, [u8; 8]),
    Host(u64),
    Refresh,
    RefreshSelected,
    Official,
    ScrollUp,
    ScrollDown,
}

struct ScanResult {
    id: u64,
    generation: u64,
    ticket: u64,
    result: Result<ClubSnapshot, String>,
}

#[derive(Resource)]
pub struct Browser {
    pub clubs: Vec<Club>,
    pub selected: Option<u64>,
    pub removing: Option<u64>,
    pub scroll: f32,
    jobs: HashMap<u64, Arc<AtomicBool>>,
    sender: mpsc::Sender<ScanResult>,
    results: Mutex<mpsc::Receiver<ScanResult>>,
    next_ticket: u64,
    was_open: bool,
}

impl Default for Browser {
    fn default() -> Self {
        Self::new(vec![SavedClub::official()], network::DEFAULT_SERVER.trim())
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        for stop in self.jobs.values() {
            stop.store(true, Ordering::Relaxed);
        }
    }
}

impl Browser {
    pub fn new(clubs: Vec<SavedClub>, selected_server: &str) -> Self {
        let selected = clubs
            .iter()
            .find(|c| c.server == selected_server)
            .or_else(|| clubs.first())
            .map(|c| c.id);
        let (sender, results) = mpsc::channel();
        Self {
            clubs: clubs.into_iter().map(Club::new).collect(),
            selected,
            removing: None,
            scroll: 0.0,
            jobs: HashMap::new(),
            sender,
            results: Mutex::new(results),
            next_ticket: 0,
            was_open: false,
        }
    }

    pub fn selected(&self) -> Option<&Club> {
        self.clubs
            .iter()
            .find(|c| Some(c.saved.id) == self.selected)
    }

    fn saved(&self) -> Vec<SavedClub> {
        self.clubs.iter().map(|c| c.saved.clone()).collect()
    }

    fn stash_password(&mut self, menu: &Menu) {
        if let Some(club) = self
            .clubs
            .iter_mut()
            .find(|c| Some(c.saved.id) == self.selected && c.saved.server == menu.server)
        {
            if club.password != menu.server_password {
                club.password = menu.server_password.clone();
                club.invalidate();
            }
        }
    }

    fn sync_destination(&mut self, menu: &mut Menu) {
        let destination = self
            .clubs
            .iter()
            .find(|c| c.saved.server == menu.server)
            .map(|c| c.saved.id);
        if self.selected != destination {
            self.selected = destination;
            if let Some(club) = self.selected() {
                // Pasting an invitation clears the previous server's password.
                // Restore only this destination's own in-memory credentials.
                if menu.server_password.is_empty() {
                    menu.server_password = club.password.clone();
                }
            }
            menu.dirty = true;
        }
        self.stash_password(menu);
    }

    fn select(&mut self, id: u64, menu: &mut Menu) {
        self.stash_password(menu);
        if let Some(club) = self.clubs.iter().find(|c| c.saved.id == id) {
            self.selected = Some(id);
            self.removing = None;
            menu.select_server(&club.saved.server, &club.password);
            menu.rooms = club
                .snapshot
                .as_ref()
                .map_or_else(Vec::new, |s| s.rooms.clone());
        }
    }

    pub fn handle(&mut self, action: Action, menu: &mut Menu, settings: &mut Settings) {
        if menu.club_editor.is_some()
            && !matches!(
                action,
                Action::Save
                    | Action::Cancel
                    | Action::Toggle(_)
                    | Action::ScrollUp
                    | Action::ScrollDown
            )
        {
            return;
        }
        menu.dirty = true;
        match action {
            Action::Add => {
                menu.club_editor = Some(None);
                menu.club_alias.clear();
                menu.club_server.clear();
                menu.club_password.clear();
                menu.active = Some(Field::ClubAlias);
                menu.status.clear();
                self.removing = None;
            }
            Action::Edit(id) => {
                self.stash_password(menu);
                if let Some(club) = self.clubs.iter().find(|c| c.saved.id == id) {
                    menu.club_editor = Some(Some(id));
                    menu.club_alias = club.saved.alias.clone();
                    menu.club_server = club.saved.server.clone();
                    menu.club_password = club.password.clone();
                    menu.active = Some(Field::ClubAlias);
                    menu.status.clear();
                    self.removing = None;
                }
            }
            Action::Save => {
                if let Err(error) = self.save(menu, settings) {
                    menu.status = error;
                }
            }
            Action::Cancel => {
                menu.club_editor = None;
                menu.club_password.clear();
                menu.active = None;
                menu.status.clear();
            }
            Action::Select(id) => self.select(id, menu),
            Action::Toggle(id) => {
                if let Some(club) = self.clubs.iter_mut().find(|c| c.saved.id == id) {
                    club.expanded = !club.expanded;
                }
            }
            Action::Remove(id) => self.removing = Some(id),
            Action::CancelRemove => self.removing = None,
            Action::ConfirmRemove(id) => {
                let saved: Vec<_> = self.saved().into_iter().filter(|c| c.id != id).collect();
                match settings.save_clubs(saved) {
                    Ok(()) => {
                        if let Some(club) = self.clubs.iter().find(|c| c.saved.id == id) {
                            if let Some(stop) = club.pending.and_then(|t| self.jobs.get(&t)) {
                                stop.store(true, Ordering::Relaxed);
                            }
                        }
                        self.clubs.retain(|c| c.saved.id != id);
                        self.removing = None;
                        if self.selected == Some(id) {
                            self.selected = None;
                            menu.select_server("", "");
                            if let Some(club) = self.clubs.first() {
                                self.select(club.saved.id, menu);
                            }
                        }
                    }
                    Err(error) => menu.status = tr("clubs.save_failed").arg("error", error),
                }
            }
            Action::Join(id, code) => {
                let code = String::from_utf8_lossy(&code).into_owned();
                if self
                    .clubs
                    .iter()
                    .find(|c| c.saved.id == id)
                    .is_some_and(|c| {
                        c.error.is_none()
                            && c.snapshot.as_ref().is_some_and(|s| {
                                s.rooms.iter().any(|r| {
                                    r.code == code && (r.capacity == 0 || r.players < r.capacity)
                                })
                            })
                    })
                {
                    self.select(id, menu);
                    menu.join_room_code(&code);
                }
            }
            Action::Host(id) => {
                self.select(id, menu);
                if self.selected == Some(id) {
                    menu.host_selected_club();
                }
            }
            Action::Refresh | Action::RefreshSelected => {
                self.stash_password(menu);
                for club in &mut self.clubs {
                    if action == Action::Refresh || Some(club.saved.id) == self.selected {
                        club.next_scan = 0.0;
                    }
                }
            }
            Action::Official => {
                if let Some(club) = self
                    .clubs
                    .iter()
                    .find(|c| c.saved.server == network::DEFAULT_SERVER.trim())
                {
                    self.select(club.saved.id, menu);
                } else {
                    self.handle(Action::Add, menu, settings);
                    menu.club_alias = "Yapshire 小镇".into();
                    menu.club_server = network::DEFAULT_SERVER.trim().into();
                    self.handle(Action::Save, menu, settings);
                }
            }
            Action::ScrollUp => self.scroll = (self.scroll - 300.0).max(0.0),
            Action::ScrollDown => self.scroll += 300.0,
        }
    }

    fn save(&mut self, menu: &mut Menu, settings: &mut Settings) -> Result<(), Message> {
        let Some(editing) = menu.club_editor else {
            return Ok(());
        };
        let mut saved = SavedClub::new(&menu.club_alias, &menu.club_server)?;
        if menu.club_password.len() > 128 {
            return Err(tr("clubs.password_long"));
        }
        if self
            .clubs
            .iter()
            .any(|c| Some(c.saved.id) != editing && c.saved.server == saved.server)
        {
            return Err(tr("clubs.duplicate"));
        }
        if editing.is_none() && self.clubs.len() >= MAX_CLUBS {
            return Err(tr("clubs.limit"));
        }
        let mut all = self.saved();
        if let Some(id) = editing {
            let Some(old) = all.iter_mut().find(|c| c.id == id) else {
                return Err(tr("clubs.removed"));
            };
            saved.id = id;
            *old = saved.clone();
        } else {
            while all.iter().any(|c| c.id == saved.id) {
                saved.id = rand::random();
            }
            all.push(saved.clone());
        }
        settings
            .save_clubs(all)
            .map_err(|error| tr("clubs.save_failed").arg("error", error))?;
        let id = saved.id;
        if let Some(club) = self.clubs.iter_mut().find(|c| c.saved.id == id) {
            if club.saved.server != saved.server || club.password != menu.club_password {
                if let Some(stop) = club.pending.and_then(|t| self.jobs.get(&t)) {
                    stop.store(true, Ordering::Relaxed);
                }
                club.invalidate();
            }
            club.saved = saved;
            club.password = menu.club_password.clone();
            club.next_scan = 0.0;
        } else {
            let mut club = Club::new(saved);
            club.password = menu.club_password.clone();
            self.clubs.push(club);
        }
        // Do not let the previous selected server's transient password overwrite
        // the just-saved credentials when selecting this entry.
        self.selected = Some(id);
        let club = self.selected().unwrap();
        menu.select_server(&club.saved.server, &club.password);
        menu.club_editor = None;
        menu.club_password.clear();
        menu.active = None;
        menu.status = tr("clubs.saved");
        Ok(())
    }

    fn accept(&mut self, result: ScanResult, now: f64) -> bool {
        self.jobs.remove(&result.ticket);
        let Some(club) = self.clubs.iter_mut().find(|c| {
            c.saved.id == result.id
                && c.generation == result.generation
                && c.pending == Some(result.ticket)
        }) else {
            return false;
        };
        club.pending = None;
        club.next_scan = now + REFRESH_SECONDS;
        match result.result {
            Ok(snapshot) => {
                club.snapshot = Some(snapshot);
                club.error = None;
            }
            Err(error) => {
                club.error = Some(tr("clubs.unavailable").arg("error", network::clean(&error, 160)))
            }
        }
        true
    }

    fn start_scans(&mut self, now: f64) -> bool {
        let mut started = false;
        let mut order: Vec<_> = (0..self.clubs.len()).collect();
        order.sort_by(|&a, &b| self.clubs[a].next_scan.total_cmp(&self.clubs[b].next_scan));
        for index in order {
            let club = &mut self.clubs[index];
            if self.jobs.len() >= MAX_SCANS {
                break;
            }
            if club.pending.is_some() || club.next_scan > now {
                continue;
            }
            self.next_ticket += 1;
            let ticket = self.next_ticket;
            let stop = Arc::new(AtomicBool::new(false));
            self.jobs.insert(ticket, stop.clone());
            club.pending = Some(ticket);
            let (id, generation) = (club.saved.id, club.generation);
            let address = club.saved.server.clone();
            let password = club.password.clone();
            let sender = self.sender.clone();
            std::thread::spawn(move || {
                let result = network::inspect_club(&address, &password, &stop);
                let _ = sender.send(ScanResult {
                    id,
                    generation,
                    ticket,
                    result,
                });
            });
            started = true;
        }
        started
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("yapshire-clubs-{}", rand::random::<u64>()));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn settings(&self) -> Settings {
            Settings::from_path(Ok(self.0.join("settings.json"))).0
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn adding_editing_and_removing_clubs_survives_restart_without_saving_passwords() {
        let fixture = Fixture::new();
        let mut settings = fixture.settings();
        let mut browser = Browser::new(settings.clubs(), settings.server());
        let mut menu = Menu::default();
        browser.handle(Action::Add, &mut menu, &mut settings);
        menu.club_alias = "茶馆 {name}".into();
        menu.club_server = "https://friends.example/".into();
        menu.club_password = "friends-password".into();
        browser.handle(Action::Save, &mut menu, &mut settings);
        assert!(menu.club_editor.is_none());
        assert_eq!(browser.clubs.len(), 2);
        let id = browser.selected.unwrap();
        assert_eq!(menu.server, "wss://friends.example");
        assert_eq!(menu.server_password, "friends-password");
        let restored = fixture.settings().clubs();
        assert_eq!(restored[1].alias, "茶馆 {name}");
        assert_eq!(restored[1].id, id);
        let json = std::fs::read_to_string(fixture.0.join("settings.json")).unwrap();
        assert!(!json.contains("password"));

        browser.handle(Action::Edit(id), &mut menu, &mut settings);
        menu.club_alias = "朋友的小镇".into();
        menu.club_server = "ws://127.0.0.1:4771".into();
        menu.club_password.clear();
        browser.handle(Action::Save, &mut menu, &mut settings);
        assert_eq!(browser.selected().unwrap().saved.id, id);
        assert_eq!(fixture.settings().clubs()[1].server, "ws://127.0.0.1:4771");
        assert!(menu.server_password.is_empty());
        for id in browser.saved().iter().map(|c| c.id).collect::<Vec<_>>() {
            browser.handle(Action::Remove(id), &mut menu, &mut settings);
            browser.handle(Action::ConfirmRemove(id), &mut menu, &mut settings);
        }
        assert!(
            fixture.settings().clubs().is_empty(),
            "An intentionally empty list must stay empty"
        );
    }

    #[test]
    fn migration_preserves_custom_server_and_explicit_aliases_and_deduplicates_urls() {
        let legacy = restore(None, "ws://127.0.0.1:4777");
        assert_eq!(legacy.len(), 2);
        assert_eq!(legacy[1].server, "ws://127.0.0.1:4777");
        let mut one = SavedClub::new("Home", "wss://friends.example").unwrap();
        let mut two = SavedClub::new("Work", "wss://work.example").unwrap();
        two.id = one.id;
        let duplicate = SavedClub::new("Duplicate", "https://friends.example/").unwrap();
        one.server = "wss://user:secret@private.example".into();
        let restored = restore(Some(vec![duplicate.clone(), duplicate, two, one]), "");
        assert_eq!(restored.len(), 2);
        assert_ne!(restored[0].id, restored[1].id);
        assert_eq!(restored[0].alias, "Duplicate");
        assert!(restore(Some(vec![]), "ws://previous.example").is_empty());
    }

    #[test]
    fn failed_or_duplicate_saves_do_not_change_the_subscription() {
        let fixture = Fixture::new();
        let mut settings = fixture.settings();
        let mut browser = Browser::new(settings.clubs(), settings.server());
        let original = browser.saved();
        let mut menu = Menu::default();
        browser.handle(Action::Add, &mut menu, &mut settings);
        menu.club_server = network::DEFAULT_SERVER.trim().into();
        browser.handle(Action::Save, &mut menu, &mut settings);
        assert_eq!(menu.status, tr("clubs.duplicate"));
        assert_eq!(browser.saved(), original);
        menu.club_server = "wss://another.example".into();
        // A directory cannot atomically be replaced by the preferences file.
        let mut unavailable = Settings::from_path(Ok(fixture.0.clone())).0;
        browser.handle(Action::Save, &mut menu, &mut unavailable);
        assert_eq!(browser.saved(), original);
        assert!(menu.club_editor.is_some());
        assert!(menu.status.to_string().contains("Could not save Clubs"));
    }

    #[test]
    fn stale_refreshes_cannot_overwrite_an_edited_or_removed_club() {
        let mut browser = Browser::default();
        let id = browser.clubs[0].saved.id;
        browser.clubs[0].pending = Some(1);
        browser.clubs[0].invalidate();
        browser.clubs[0].saved.server = "wss://replacement.example".into();
        browser.clubs[0].pending = Some(2);
        let old = ScanResult {
            id,
            ticket: 1,
            generation: 0,
            result: Err("old host offline".into()),
        };
        assert!(!browser.accept(old, 2.0));
        assert_eq!(browser.clubs[0].pending, Some(2));
        assert!(browser.clubs[0].error.is_none());
        browser.clubs.clear();
        assert!(!browser.accept(
            ScanResult {
                id,
                ticket: 2,
                generation: 1,
                result: Ok(ClubSnapshot {
                    rooms: vec![],
                    latency_ms: Some(10)
                })
            },
            3.0
        ));
        assert!(browser.clubs.is_empty());
    }

    #[test]
    fn joining_uses_the_clicked_club_and_never_reuses_another_clubs_password() {
        let fixture = Fixture::new();
        let mut settings = fixture.settings();
        let a = SavedClub::new("A", "ws://127.0.0.1:4701").unwrap();
        let b = SavedClub::new("B", "ws://127.0.0.1:4702").unwrap();
        let mut menu = Menu::default();
        menu.page = Page::Cloud;
        menu.select_server(&a.server, "password-for-a");
        let mut browser = Browser::new(vec![a, b.clone()], &menu.server);
        browser.clubs[1].snapshot = Some(ClubSnapshot {
            latency_ms: Some(8),
            rooms: vec![network::RoomEntry {
                code: "MAIN0001".into(),
                name: "B room".into(),
                players: 0,
                capacity: 2,
                address: String::new(),
            }],
        });
        browser.handle(Action::Join(b.id, *b"MAIN0001"), &mut menu, &mut settings);
        assert!(
            matches!(menu.request, Some(network::Mode::JoinCloud { ref server, .. }) if *server == b.server)
        );
        assert!(menu.server_password.is_empty());
        assert_eq!(browser.clubs[0].password, "password-for-a");
        assert!(browser.clubs[1].password.is_empty());
    }

    #[test]
    fn invitations_restore_only_the_destination_clubs_own_password() {
        let a = SavedClub::new("A", "wss://a.example").unwrap();
        let b = SavedClub::new("B", "wss://b.example").unwrap();
        let mut menu = Menu::default();
        menu.select_server(&a.server, "password-for-a");
        let mut browser = Browser::new(vec![a.clone(), b.clone()], &a.server);
        browser.clubs[1].password = "password-for-b".into();
        browser.sync_destination(&mut menu);
        menu.select_server(&b.server, "");
        browser.sync_destination(&mut menu);
        assert_eq!(browser.selected, Some(b.id));
        assert_eq!(menu.server_password, "password-for-b");
        assert_eq!(browser.clubs[0].password, "password-for-a");
        menu.select_server("wss://unknown.example", "");
        browser.sync_destination(&mut menu);
        assert!(browser.selected.is_none() && menu.server_password.is_empty());
        menu.select_server(&a.server, "");
        browser.sync_destination(&mut menu);
        assert_eq!(menu.server_password, "password-for-a");
    }
}

pub fn update(
    time: Res<Time>,
    mut browser: ResMut<Browser>,
    mut menu: ResMut<Menu>,
    mut settings: ResMut<Settings>,
) {
    let now = time.elapsed_secs_f64();
    let results: Vec<_> = browser.results.lock().unwrap().try_iter().collect();
    let mut changed = false;
    for result in results {
        changed |= browser.accept(result, now);
    }
    if menu.page != Page::Cloud {
        browser.was_open = false;
        return;
    }
    if menu.club_submit {
        menu.club_submit = false;
        browser.handle(Action::Save, &mut menu, &mut settings);
    }
    if !browser.was_open {
        browser.was_open = true;
        browser.sync_destination(&mut menu);
        for club in &mut browser.clubs {
            club.next_scan = 0.0;
        }
    }
    if menu.club_editor.is_none() && menu.active != Some(Field::ServerPassword) {
        browser.sync_destination(&mut menu);
    }
    if !menu.connecting && menu.request.is_none() {
        changed |= browser.start_scans(now);
    }
    if changed {
        menu.rooms = browser
            .selected()
            .and_then(|c| c.snapshot.as_ref())
            .map_or_else(Vec::new, |s| s.rooms.clone());
        menu.dirty = true;
    }
}
