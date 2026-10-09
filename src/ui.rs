use crate::{
    Session,
    game::{Actor, Art, font},
    network::{self, ClientMessage, Mode},
};
use bevy::{
    input::keyboard::{Key, KeyboardInput},
    prelude::*,
};
use std::sync::{Mutex, mpsc};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    Host,
    Lan,
    Cloud,
    Playing,
}
#[derive(Clone, Copy, PartialEq, Eq, Component)]
pub enum Field {
    Name,
    Address,
    Server,
    Room,
    RoomName,
    Port,
}

#[derive(Resource)]
pub struct Menu {
    pub page: Page,
    pub active: Option<Field>,
    pub name: String,
    address: String,
    server: String,
    room: String,
    pub(crate) room_name: String,
    port: String,
    cloud_host: bool,
    preedit: String,
    selected: bool,
    pub status: String,
    pub connecting: bool,
    pub request: Option<Mode>,
    pub leave: bool,
    pub dirty: bool,
    pub(crate) rooms: Vec<network::RoomEntry>,
    scan: Option<Mutex<mpsc::Receiver<Result<Vec<network::RoomEntry>, String>>>>,
    lobby_error: String,
    room_page: usize,
}

impl Default for Menu {
    fn default() -> Self {
        Self {
            page: Page::Home,
            active: None,
            name: "Wanderer".into(),
            address: "127.0.0.1:4761".into(),
            server: network::DEFAULT_SERVER.trim().into(),
            room: String::new(),
            room_name: String::new(),
            port: network::PORT.to_string(),
            cloud_host: false,
            preedit: String::new(),
            selected: false,
            status: String::new(),
            connecting: false,
            request: None,
            leave: false,
            dirty: true,
            rooms: Vec::new(),
            scan: None,
            lobby_error: String::new(),
            room_page: 0,
        }
    }
}

impl Menu {
    fn field(&self, field: Field) -> &str {
        match field {
            Field::Name => &self.name,
            Field::Address => &self.address,
            Field::Server => &self.server,
            Field::Room => &self.room,
            Field::RoomName => &self.room_name,
            Field::Port => &self.port,
        }
    }
    fn field_mut(&mut self, field: Field) -> &mut String {
        match field {
            Field::Name => &mut self.name,
            Field::Address => &mut self.address,
            Field::Server => &mut self.server,
            Field::Room => &mut self.room,
            Field::RoomName => &mut self.room_name,
            Field::Port => &mut self.port,
        }
    }
    fn fields(&self) -> Vec<Field> {
        match self.page {
            Page::Home => vec![Field::Name],
            Page::Host if self.cloud_host => vec![Field::RoomName],
            Page::Host => vec![Field::Port],
            Page::Lan => vec![Field::Address],
            Page::Cloud => vec![Field::Server, Field::Room],
            Page::Playing => vec![],
        }
    }
    fn go(&mut self, page: Page) {
        self.page = page;
        self.active = None;
        self.preedit.clear();
        self.status.clear();
        self.dirty = true;
        self.rooms.clear();
        self.scan = None;
        self.room_page = 0;
        if matches!(page, Page::Lan | Page::Cloud) {
            self.refresh_rooms();
        }
    }
    fn refresh_rooms(&mut self) {
        if self.scan.is_some() {
            return;
        }
        let cloud = self.page == Page::Cloud;
        let address = self.server.clone();
        let (send, receive) = mpsc::channel();
        std::thread::spawn(move || {
            let result = if cloud {
                network::discover_cloud(&address)
            } else {
                network::discover_lan()
            };
            let _ = send.send(result);
        });
        self.scan = Some(Mutex::new(receive));
        self.lobby_error.clear();
        self.dirty = true;
    }
    fn connect(&mut self) {
        if self.connecting {
            return;
        }
        self.name = network::clean(&self.name, 12);
        if self.name.is_empty() {
            self.status = "Please enter your nickname on the main menu.".into();
            return;
        }
        self.request = match self.page {
            Page::Host if self.cloud_host => {
                self.room_name = network::clean(&self.room_name, 24);
                if self.room_name.is_empty() {
                    self.status = "Please enter a room name.".into();
                    None
                } else {
                    Some(Mode::HostCloud {
                        server: network::DEFAULT_SERVER.trim().into(),
                        room_name: self.room_name.clone(),
                    })
                }
            }
            Page::Host => match self.port.parse::<u16>() {
                Ok(port) if port > 0 => Some(Mode::HostLan(port)),
                _ => {
                    self.status = "Port must be a number from 1 to 65535.".into();
                    None
                }
            },
            Page::Lan => Some(Mode::JoinLan(self.address.clone())),
            Page::Cloud => Some(Mode::JoinCloud {
                server: self.server.clone(),
                room: self.room.clone(),
            }),
            _ => None,
        };
        self.active = None;
    }
}

#[derive(Resource, Default)]
pub struct Chat {
    pub open: bool,
    value: String,
    preedit: String,
    selected: bool,
}

#[derive(Component, Clone, Copy)]
pub enum Action {
    Go(Page),
    Focus(Field),
    Hosting(bool),
    Connect,
    Back,
    Copy,
    Refresh,
    JoinRoom(usize),
    NextRooms,
    Fishing(crate::fishing::Action),
}
#[derive(Component)]
pub struct Root;
#[derive(Component)]
pub struct FieldValue(Field);
#[derive(Component)]
pub struct Status;
#[derive(Component)]
pub struct ChatValue;
#[derive(Component)]
pub struct ChatLog;
#[derive(Component)]
pub(crate) struct PlayOverlay;
#[derive(Component)]
pub struct RoomStatus;
#[derive(Component)]
pub struct BaseColor(Color);

pub(crate) const INK: Color = Color::srgb_u8(48, 76, 67);
pub(crate) const GREEN: Color = Color::srgb_u8(64, 99, 80);
pub(crate) const CREAM: Color = Color::srgb_u8(247, 234, 206);
pub(crate) const MUTED: Color = Color::srgb_u8(123, 133, 104);

pub fn buttons(
    mut interactions: Query<
        (&Interaction, &Action, &mut BackgroundColor, &BaseColor),
        Changed<Interaction>,
    >,
    mut menu: ResMut<Menu>,
    session: Res<Session>,
    mut fishing: ResMut<crate::fishing::Fishing>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    for (interaction, action, mut color, base) in &mut interactions {
        *color = match interaction {
            Interaction::Hovered => BackgroundColor(Color::srgb_u8(156, 165, 121)),
            Interaction::Pressed => BackgroundColor(Color::srgb_u8(124, 147, 106)),
            Interaction::None => BackgroundColor(base.0),
        };
        if *interaction != Interaction::Pressed {
            continue;
        }
        if menu.connecting && !matches!(action, Action::Back) {
            continue;
        }
        match *action {
            Action::Go(page) => menu.go(page),
            Action::Focus(field) => {
                menu.active = Some(field);
                menu.selected = true;
                menu.preedit.clear();
            }
            Action::Hosting(cloud) => {
                menu.cloud_host = cloud;
                menu.active = None;
                menu.dirty = true;
            }
            Action::Connect => menu.connect(),
            Action::Back => {
                menu.leave = true;
                menu.connecting = false;
                menu.go(Page::Home);
            }
            Action::Copy => {
                menu.status = match arboard::Clipboard::new()
                    .and_then(|mut c| c.set_text(session.invite.clone()))
                {
                    Ok(_) => "Invite copied to clipboard.".into(),
                    Err(_) => format!("Invite: {}", session.invite),
                };
            }
            Action::Refresh => menu.refresh_rooms(),
            Action::JoinRoom(index) => {
                if let Some(room) = menu.rooms.get(index).cloned() {
                    if menu.page == Page::Lan {
                        menu.address = room.address;
                    } else {
                        menu.room = room.code;
                    }
                    menu.connect();
                }
            }
            Action::NextRooms => {
                menu.room_page = (menu.room_page + 1) % menu.rooms.len().div_ceil(3).max(1);
                menu.dirty = true;
            }
            Action::Fishing(action) => {
                // Rebuilt shop buttons under a held mouse must not buy repeatedly.
                if mouse.just_pressed(MouseButton::Left) {
                    fishing.command = Some(action);
                }
            }
        }
    }
}

fn insert(value: &mut String, selected: &mut bool, text: &str, limit: usize) {
    if *selected {
        value.clear();
        *selected = false;
    }
    let remaining = limit.saturating_sub(value.chars().count());
    value.extend(text.chars().filter(|c| !c.is_control()).take(remaining));
}

fn edit(
    value: &mut String,
    selected: &mut bool,
    event: &KeyboardInput,
    control: bool,
    limit: usize,
) {
    match event.key_code {
        KeyCode::KeyA if control => *selected = true,
        KeyCode::KeyV if control => {
            if let Ok(text) = arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
                insert(value, selected, &text, limit);
            }
        }
        KeyCode::KeyC if control => {
            let _ = arboard::Clipboard::new().and_then(|mut c| c.set_text(value.clone()));
        }
        KeyCode::Backspace => {
            if *selected {
                value.clear();
                *selected = false;
            } else {
                value.pop();
            }
        }
        _ if !control => {
            if let Some(text) = &event.text {
                insert(value, selected, text, limit);
            }
        }
        _ => {}
    }
}

pub fn keyboard(
    mut input: MessageReader<KeyboardInput>,
    mut ime: MessageReader<Ime>,
    keys: Res<ButtonInput<KeyCode>>,
    mut menu: ResMut<Menu>,
    mut chat: ResMut<Chat>,
    mut session: ResMut<Session>,
    mut window: Single<&mut Window>,
    mut fishing: ResMut<crate::fishing::Fishing>,
) {
    let mut composing = if chat.open {
        !chat.preedit.is_empty()
    } else {
        !menu.preedit.is_empty()
    };
    let mut committed = false;
    for event in ime.read() {
        match event {
            Ime::Preedit { value, .. } => {
                composing |= !value.is_empty();
                if chat.open {
                    chat.preedit = value.clone();
                } else {
                    menu.preedit = value.clone();
                }
            }
            Ime::Commit { value, .. } => {
                committed = true;
                if chat.open {
                    let Chat {
                        value: text,
                        selected,
                        preedit,
                        ..
                    } = &mut *chat;
                    insert(text, selected, value, 80);
                    preedit.clear();
                } else if let Some(field) = menu.active {
                    let mut selected = menu.selected;
                    insert(
                        menu.field_mut(field),
                        &mut selected,
                        value,
                        field_limit(field),
                    );
                    menu.selected = selected;
                    menu.preedit.clear();
                }
            }
            Ime::Disabled { .. } => {
                chat.preedit.clear();
                menu.preedit.clear();
            }
            _ => {}
        }
    }
    let control = keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    for event in input.read().filter(|e| e.state.is_pressed()) {
        if event.logical_key == Key::Escape && !event.repeat {
            if composing {
                continue;
            }
            if chat.open {
                chat.open = false;
                chat.preedit.clear();
            } else if menu.page == Page::Playing && (fishing.modal() || fishing.indoors) {
                fishing.command = Some(crate::fishing::Action::Close);
            } else if menu.active.is_some() {
                menu.active = None;
            } else {
                menu.leave = true;
                menu.connecting = false;
                menu.go(Page::Home);
            }
            continue;
        }
        if event.logical_key == Key::Enter && !event.repeat {
            if menu.page == Page::Playing && fishing.modal() {
                continue;
            }
            if composing || committed {
                continue;
            }
            if menu.page == Page::Playing {
                if chat.open {
                    let text = network::clean(&chat.value, 80);
                    if text.is_empty() {
                        chat.open = false;
                        continue;
                    }
                    if let Some(link) = &session.link {
                        if link.send.try_send(ClientMessage::Chat { text }).is_ok() {
                            chat.value.clear();
                            chat.open = false;
                        }
                    } else {
                        session.log("Disconnected. Your draft is still here.".into());
                    }
                } else if session.connected {
                    chat.open = true;
                    chat.selected = false;
                }
            } else if menu.page == Page::Home {
                menu.active = None;
            } else {
                menu.connect();
            }
            continue;
        }
        if menu.page != Page::Playing && event.logical_key == Key::Tab {
            let fields = menu.fields();
            if !fields.is_empty() {
                let next = menu
                    .active
                    .and_then(|f| fields.iter().position(|&x| x == f))
                    .map_or(0, |i| (i + 1) % fields.len());
                menu.active = Some(fields[next]);
                menu.selected = true;
            }
            continue;
        }
        if composing || committed {
            continue;
        }
        if chat.open {
            let Chat {
                value, selected, ..
            } = &mut *chat;
            edit(value, selected, event, control, 80);
        } else if let Some(field) = menu.active {
            let mut selected = menu.selected;
            edit(
                menu.field_mut(field),
                &mut selected,
                event,
                control,
                field_limit(field),
            );
            menu.selected = selected;
        } else if menu.page == Page::Home {
            match event.key_code {
                KeyCode::Digit1 => menu.go(Page::Host),
                KeyCode::Digit2 => menu.go(Page::Lan),
                KeyCode::Digit3 => menu.go(Page::Cloud),
                _ => {}
            }
        } else if menu.page == Page::Host
            && matches!(event.key_code, KeyCode::Digit1 | KeyCode::Digit2)
        {
            menu.cloud_host = event.key_code == KeyCode::Digit2;
            menu.dirty = true;
        }
    }
    window.ime_enabled = chat.open || menu.active.is_some();
    window.ime_position = if chat.open {
        Vec2::new(100.0, window.height() - 72.0)
    } else {
        Vec2::new(window.width() * 0.2, window.height() * 0.55)
    };
}

pub fn discover(time: Res<Time>, mut elapsed: Local<f32>, mut menu: ResMut<Menu>) {
    if !matches!(menu.page, Page::Lan | Page::Cloud) {
        *elapsed = 0.0;
        return;
    }
    let result = menu
        .scan
        .as_ref()
        .and_then(|rx| match rx.lock().unwrap().try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("Lobby scan stopped. Please refresh.".into()))
            }
        });
    if let Some(result) = result {
        menu.scan = None;
        match result {
            Ok(rooms) => {
                menu.rooms = rooms;
                menu.room_page = menu.room_page.min(menu.rooms.len().saturating_sub(1) / 3);
            }
            Err(error) => {
                menu.lobby_error = error;
                menu.rooms.clear();
            }
        }
        menu.dirty = true;
        *elapsed = 0.0;
    }
    *elapsed += time.delta_secs();
    if *elapsed >= 8.0 && !menu.connecting {
        menu.refresh_rooms();
        *elapsed = 0.0;
    }
}

fn field_limit(field: Field) -> usize {
    match field {
        Field::Name => 12,
        Field::Room => 8,
        Field::RoomName => 24,
        Field::Port => 5,
        _ => 200,
    }
}

pub(crate) fn label(
    commands: &mut Commands,
    parent: Entity,
    art: &Art,
    text: &str,
    size: f32,
    color: Color,
) -> Entity {
    let entity = commands
        .spawn((Text::new(text), font(art, size), TextColor(color)))
        .id();
    commands.entity(parent).add_child(entity);
    entity
}

pub(crate) fn button(
    commands: &mut Commands,
    parent: Entity,
    art: &Art,
    title: &str,
    subtitle: &str,
    action: Action,
    primary: bool,
) -> Entity {
    let color = if primary {
        GREEN
    } else {
        Color::srgb_u8(228, 218, 186)
    };
    let entity = commands
        .spawn((
            Button,
            action,
            BaseColor(color),
            Node {
                width: percent(100),
                min_height: px(if subtitle.is_empty() { 48.0 } else { 74.0 }),
                padding: UiRect::axes(px(18), px(10)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                row_gap: px(6),
                border: UiRect::bottom(px(3)),
                ..default()
            },
            BackgroundColor(color),
            BorderColor::all(if primary {
                Color::srgb_u8(42, 73, 58)
            } else {
                Color::srgb_u8(200, 192, 159)
            }),
        ))
        .id();
    commands.entity(parent).add_child(entity);
    label(
        commands,
        entity,
        art,
        title,
        24.0,
        if primary { CREAM } else { INK },
    );
    if !subtitle.is_empty() {
        label(
            commands,
            entity,
            art,
            subtitle,
            16.0,
            if primary {
                Color::srgb_u8(206, 215, 169)
            } else {
                MUTED
            },
        );
    }
    entity
}

fn field(commands: &mut Commands, parent: Entity, art: &Art, title: &str, which: Field) {
    label(commands, parent, art, title, 18.0, MUTED);
    let entity = commands
        .spawn((
            Button,
            Action::Focus(which),
            BaseColor(Color::srgb_u8(255, 246, 220)),
            Node {
                width: percent(100),
                min_height: px(48),
                padding: UiRect::all(px(12)),
                border: UiRect::all(px(2)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(255, 246, 220)),
            BorderColor::all(Color::srgb_u8(195, 193, 158)),
        ))
        .id();
    commands.entity(parent).add_child(entity);
    let text = label(
        commands,
        entity,
        art,
        "",
        if which == Field::Server { 16.0 } else { 24.0 },
        INK,
    );
    commands
        .entity(text)
        .insert((FieldValue(which), TextLayout::new_with_no_wrap()));
}

pub fn render(
    mut commands: Commands,
    mut menu: ResMut<Menu>,
    art: Res<Art>,
    roots: Query<Entity, With<Root>>,
) {
    if !menu.dirty {
        return;
    }
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    menu.dirty = false;
    let root = commands
        .spawn((
            Root,
            Node {
                width: percent(100),
                height: percent(100),
                position_type: PositionType::Absolute,
                ..default()
            },
        ))
        .id();
    if menu.page == Page::Playing {
        let header = commands
            .spawn(Node {
                position_type: PositionType::Absolute,
                left: px(32),
                top: px(26),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                ..default()
            })
            .id();
        commands.entity(root).add_child(header);
        label(&mut commands, header, &art, "Yapshire", 36.0, CREAM);
        label(
            &mut commands,
            header,
            &art,
            "A LITTLE PLACE TO BE TOGETHER",
            16.0,
            CREAM,
        );
        let room = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: px(32),
                    top: px(26),
                    width: px(410),
                    padding: UiRect::all(px(14)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                    ..default()
                },
                BackgroundColor(Color::srgba_u8(43, 67, 58, 225)),
            ))
            .id();
        commands.entity(root).add_child(room);
        let status = label(&mut commands, room, &art, "", 18.0, CREAM);
        commands.entity(status).insert(RoomStatus);
        button(
            &mut commands,
            room,
            &art,
            "COPY INVITE",
            "",
            Action::Copy,
            true,
        );
        let log = commands
            .spawn((
                PlayOverlay,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(32),
                    bottom: px(76),
                    width: px(600),
                    max_height: px(165),
                    overflow: Overflow::clip(),
                    padding: UiRect::all(px(14)),
                    ..default()
                },
                BackgroundColor(Color::srgba_u8(34, 53, 47, 170)),
            ))
            .id();
        commands.entity(root).add_child(log);
        let text = label(&mut commands, log, &art, "", 18.0, CREAM);
        commands.entity(text).insert(ChatLog);
        let bar = commands
            .spawn((
                PlayOverlay,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(32),
                    right: px(32),
                    bottom: px(16),
                    min_height: px(38),
                    padding: UiRect::axes(px(14), px(6)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(2),
                    ..default()
                },
                BackgroundColor(Color::srgba_u8(34, 53, 47, 220)),
                BorderColor::all(Color::srgb_u8(189, 187, 150)),
            ))
            .id();
        commands.entity(root).add_child(bar);
        let input = label(&mut commands, bar, &art, "", 18.0, CREAM);
        commands.entity(input).insert(ChatValue);
        let status = label(&mut commands, bar, &art, "", 14.0, MUTED);
        commands.entity(status).insert(Status);
        return;
    }
    commands
        .entity(root)
        .insert(BackgroundColor(Color::srgba_u8(25, 51, 48, 32)));
    let panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: percent(6),
                top: percent(6),
                width: px(456),
                padding: UiRect::all(px(28)),
                flex_direction: FlexDirection::Column,
                row_gap: px(13),
                border: UiRect::all(px(3)),
                ..default()
            },
            BackgroundColor(CREAM),
            BorderColor::all(Color::srgb_u8(204, 194, 155)),
        ))
        .id();
    commands.entity(root).add_child(panel);
    label(&mut commands, panel, &art, "Y A P S H I R E", 18.0, MUTED);
    label(&mut commands, panel, &art, "Yapshire", 48.0, INK);
    label(
        &mut commands,
        panel,
        &art,
        "A quiet street. A few good friends.",
        18.0,
        MUTED,
    );
    match menu.page {
        Page::Home => {
            field(&mut commands, panel, &art, "YOUR NICKNAME", Field::Name);
            button(
                &mut commands,
                panel,
                &art,
                "01  HOST A ROOM",
                "Leave a light on for your friends.",
                Action::Go(Page::Host),
                true,
            );
            button(
                &mut commands,
                panel,
                &art,
                "02  JOIN LAN",
                "A little closer. On the same network.",
                Action::Go(Page::Lan),
                false,
            );
            button(
                &mut commands,
                panel,
                &art,
                "03  JOIN SERVER",
                "Far apart. Still walking together.",
                Action::Go(Page::Cloud),
                false,
            );
            label(
                &mut commands,
                panel,
                &art,
                "CLICK TO CHOOSE / KEYS 1, 2, 3",
                14.0,
                MUTED,
            );
        }
        Page::Host => {
            label(
                &mut commands,
                panel,
                &art,
                "WHERE SHALL WE MEET?",
                24.0,
                INK,
            );
            button(
                &mut commands,
                panel,
                &art,
                if menu.cloud_host {
                    "[1] LOCAL NETWORK"
                } else {
                    "[1] LOCAL NETWORK"
                },
                "Share your IP address and port.",
                Action::Hosting(false),
                !menu.cloud_host,
            );
            button(
                &mut commands,
                panel,
                &art,
                if menu.cloud_host {
                    "[2] ONLINE SERVER"
                } else {
                    "[2] ONLINE SERVER"
                },
                "Share a room code. Meet anywhere.",
                Action::Hosting(true),
                menu.cloud_host,
            );
            if menu.cloud_host {
                field(&mut commands, panel, &art, "ROOM NAME", Field::RoomName);
            } else {
                field(&mut commands, panel, &art, "ROOM PORT", Field::Port);
            }
            button(
                &mut commands,
                panel,
                &art,
                if menu.connecting {
                    "OPENING..."
                } else {
                    "OPEN THE DOOR  >"
                },
                "",
                Action::Connect,
                true,
            );
            button(
                &mut commands,
                panel,
                &art,
                "<  BACK",
                "",
                Action::Back,
                false,
            );
        }
        Page::Lan => {
            label(&mut commands, panel, &art, "VISIT A FRIEND", 24.0, INK);
            label(
                &mut commands,
                panel,
                &art,
                "Ask your friend for their invite.",
                18.0,
                MUTED,
            );
            field(&mut commands, panel, &art, "HOST IP : PORT", Field::Address);
            label(
                &mut commands,
                panel,
                &art,
                "Example: 192.168.1.10:4761\nSame computer: 127.0.0.1:4761",
                16.0,
                MUTED,
            );
            button(
                &mut commands,
                panel,
                &art,
                if menu.connecting {
                    "CONNECTING..."
                } else {
                    "COME ON IN  >"
                },
                "",
                Action::Connect,
                true,
            );
            button(
                &mut commands,
                panel,
                &art,
                "<  BACK",
                "",
                Action::Back,
                false,
            );
        }
        Page::Cloud => {
            label(&mut commands, panel, &art, "A PLACE TO MEET", 24.0, INK);
            field(&mut commands, panel, &art, "SERVER ADDRESS", Field::Server);
            field(
                &mut commands,
                panel,
                &art,
                "8-CHARACTER ROOM CODE",
                Field::Room,
            );
            button(
                &mut commands,
                panel,
                &art,
                if menu.connecting {
                    "CONNECTING..."
                } else {
                    "COME ON IN  >"
                },
                "",
                Action::Connect,
                true,
            );
            button(
                &mut commands,
                panel,
                &art,
                "<  BACK",
                "",
                Action::Back,
                false,
            );
        }
        Page::Playing => {}
    }
    let status = label(
        &mut commands,
        panel,
        &art,
        "",
        16.0,
        Color::srgb_u8(164, 88, 62),
    );
    commands.entity(status).insert(Status);
    let caption = commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: percent(59),
            top: percent(15),
            flex_direction: FlexDirection::Column,
            row_gap: px(16),
            ..default()
        })
        .id();
    commands.entity(root).add_child(caption);
    label(
        &mut commands,
        caption,
        &art,
        "A little town.\nA little company.",
        36.0,
        CREAM,
    );
    if matches!(menu.page, Page::Lan | Page::Cloud) {
        let lobby = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(59),
                    top: percent(34),
                    width: px(500),
                    padding: UiRect::all(px(22)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(12),
                    border: UiRect::all(px(2)),
                    ..default()
                },
                BackgroundColor(CREAM),
                BorderColor::all(Color::srgb_u8(204, 194, 155)),
            ))
            .id();
        commands.entity(root).add_child(lobby);
        label(
            &mut commands,
            lobby,
            &art,
            if menu.page == Page::Lan {
                "NEARBY ROOMS"
            } else {
                "ONLINE LOBBY"
            },
            24.0,
            INK,
        );
        let detail = if menu.scan.is_some() {
            "Looking for company...".into()
        } else {
            format!("{} ROOM(S) / AUTO REFRESH 8s", menu.rooms.len())
        };
        label(&mut commands, lobby, &art, &detail, 16.0, MUTED);
        if menu.rooms.is_empty() && menu.scan.is_none() {
            label(
                &mut commands,
                lobby,
                &art,
                if menu.lobby_error.is_empty() {
                    "No rooms yet. Be the first to host!\nYou can also enter an invite on the left."
                } else {
                    &menu.lobby_error
                },
                18.0,
                INK,
            );
        }
        for (index, room) in menu
            .rooms
            .iter()
            .enumerate()
            .skip(menu.room_page * 3)
            .take(3)
        {
            let subtitle = format!(
                "{}/16 HERE  /  {}",
                room.players,
                if menu.page == Page::Lan {
                    &room.address
                } else {
                    &room.code
                }
            );
            button(
                &mut commands,
                lobby,
                &art,
                &format!(
                    "{}  >",
                    if menu.page == Page::Lan {
                        format!("{}'s town", room.name)
                    } else {
                        room.name.clone()
                    }
                ),
                &subtitle,
                Action::JoinRoom(index),
                false,
            );
        }
        if menu.rooms.len() > 3 {
            button(
                &mut commands,
                lobby,
                &art,
                &format!(
                    "NEXT PAGE  {}/{}  >",
                    menu.room_page + 1,
                    menu.rooms.len().div_ceil(3)
                ),
                "",
                Action::NextRooms,
                false,
            );
        }
        button(
            &mut commands,
            lobby,
            &art,
            "REFRESH ROOMS",
            "",
            Action::Refresh,
            true,
        );
    }
    label(
        &mut commands,
        caption,
        &art,
        "SLOW DOWN. SAY HELLO.",
        16.0,
        CREAM,
    );
    let footer = commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            right: px(36),
            bottom: px(24),
            ..default()
        })
        .id();
    commands.entity(root).add_child(footer);
    label(
        &mut commands,
        footer,
        &art,
        "TAB NEXT FIELD  /  CTRL+V PASTE  /  F11 WINDOW / FULLSCREEN",
        16.0,
        CREAM,
    );
}

pub fn refresh(
    menu: Res<Menu>,
    chat: Res<Chat>,
    session: Res<Session>,
    time: Res<Time>,
    fishing: Res<crate::fishing::Fishing>,
    actors: Query<&Actor>,
    mut text: Query<(
        &mut Text,
        Option<&FieldValue>,
        Option<&Status>,
        Option<&ChatValue>,
        Option<&ChatLog>,
        Option<&RoomStatus>,
    )>,
    mut borders: Query<(&Action, &mut BorderColor)>,
    mut overlays: Query<&mut Node, With<PlayOverlay>>,
) {
    for mut node in &mut overlays {
        node.display = if fishing.modal() && !chat.open {
            Display::None
        } else {
            Display::Flex
        };
    }
    let cursor = if (time.elapsed_secs() * 2.0) as u32 % 2 == 0 {
        "_"
    } else {
        " "
    };
    for (mut text, field, status, input, log, room) in &mut text {
        let value = if let Some(FieldValue(field)) = field {
            let value = menu.field(*field);
            if menu.active == Some(*field) {
                format!("{}{}{}", value, menu.preedit, cursor)
            } else if value.is_empty() {
                "Click to type...".into()
            } else {
                value.into()
            }
        } else if status.is_some() {
            if menu.page == Page::Playing && menu.status.is_empty() && chat.open {
                "ENTER CHAT / SEND   /   ESC CLOSE CHAT / LEAVE   /   80 CHARACTERS".into()
            } else {
                menu.status.clone()
            }
        } else if input.is_some() {
            if chat.open {
                format!("SAY HELLO  >  {}{}{}", chat.value, chat.preedit, cursor)
            } else {
                "A D WALK  /  SHIFT RUN  /  SPACE JUMP  /  E INTERACT  /  I SATCHEL  /  ENTER CHAT"
                    .into()
            }
        } else if log.is_some() {
            session.log.iter().cloned().collect::<Vec<_>>().join("\n")
        } else if room.is_some() {
            format!(
                "{}\n{} / {} IN TOWN",
                session.label,
                if session.connected {
                    "CONNECTED"
                } else {
                    "DISCONNECTED"
                },
                actors.iter().count()
            )
        } else {
            continue;
        };
        if **text != value {
            **text = value;
        }
    }
    for (action, mut border) in &mut borders {
        if let Action::Focus(field) = action {
            *border = BorderColor::all(if menu.active == Some(*field) {
                GREEN
            } else {
                Color::srgb_u8(195, 193, 158)
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_held_mouse_does_not_repeat_purchases_when_shop_buttons_are_rebuilt() {
        let mut app = App::new();
        app.init_resource::<Menu>()
            .init_resource::<Session>()
            .init_resource::<crate::fishing::Fishing>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_systems(Update, buttons);
        let spawn = |world: &mut World| {
            world.spawn((
                Interaction::Pressed,
                Action::Fishing(crate::fishing::Action::Bait),
                BackgroundColor(GREEN),
                BaseColor(GREEN),
            ));
        };
        spawn(app.world_mut());
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert_eq!(
            app.world_mut()
                .resource_mut::<crate::fishing::Fishing>()
                .command
                .take(),
            Some(crate::fishing::Action::Bait)
        );
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        spawn(app.world_mut());
        app.update();
        assert!(
            app.world()
                .resource::<crate::fishing::Fishing>()
                .command
                .is_none()
        );
    }

    #[test]
    fn public_host_uses_a_room_name_and_the_default_server() {
        let mut menu = Menu {
            page: Page::Host,
            cloud_host: true,
            server: "ws://other.example".into(),
            ..default()
        };
        assert!(menu.fields() == vec![Field::RoomName]);
        menu.connect();
        assert!(menu.request.is_none());
        assert_eq!(menu.status, "Please enter a room name.");
        menu.room_name = " Sunset & Friends\n ".into();
        menu.connect();
        assert!(
            matches!(menu.request, Some(Mode::HostCloud { server, room_name }) if server == network::DEFAULT_SERVER.trim() && room_name == "Sunset & Friends")
        );
    }
    #[test]
    fn text_input_preserves_unicode_and_replaces_selection() {
        let mut value = "你好".to_owned();
        let mut selected = false;
        insert(&mut value, &mut selected, "呀\n朋友", 4);
        assert_eq!(value, "你好呀朋");
        value.pop();
        assert_eq!(value, "你好呀");
        selected = true;
        insert(&mut value, &mut selected, "小风", 12);
        assert_eq!(value, "小风");
        assert!(!selected);
    }
}
