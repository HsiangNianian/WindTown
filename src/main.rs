#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod game;
mod network;
#[cfg(debug_assertions)]
mod smoke;
mod ui;

use bevy::prelude::*;
use network::{Event, ServerMessage};
use std::collections::VecDeque;

#[derive(Resource, Default)]
struct Session {
    link: Option<network::Link>,
    you: Option<u32>,
    label: String,
    invite: String,
    connected: bool,
    log: VecDeque<String>,
}

impl Session {
    fn log(&mut self, line: String) {
        self.log.push_back(line);
        while self.log.len() > 5 {
            self.log.pop_front();
        }
    }
}

fn main() {
    let adjacent_assets = std::env::current_exe()
        .unwrap_or_default()
        .with_file_name("assets");
    let asset_path = if adjacent_assets.is_dir() {
        adjacent_assets
    } else {
        std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/assets"))
    };
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb_u8(35, 56, 57)))
        .init_resource::<Session>()
        .init_resource::<ui::Menu>()
        .init_resource::<ui::Chat>()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(AssetPlugin {
                    file_path: asset_path.to_string_lossy().into_owned(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: std::env::var("WIND_TOWN_SMOKE")
                            .map(|mode| format!("WIND TOWN - AUTOMATED TEST - {mode}"))
                            .unwrap_or_else(|_| "WIND TOWN - A little place to be together".into()),
                        name: Some("wind-town".into()),
                        resolution: game::WINDOW_SIZE.into(),
                        resizable: false,
                        enabled_buttons: bevy::window::EnabledButtons {
                            maximize: false,
                            ..default()
                        },
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(bevy_ecs_tilemap::TilemapPlugin)
        .add_systems(Startup, game::setup)
        .add_systems(
            Update,
            (
                ui::buttons,
                ui::discover,
                ui::keyboard,
                connect_requests,
                network_events,
                game::walk,
                game::animate,
                game::follow_camera,
                game::bubbles,
                ui::render,
                ui::refresh,
                game::fit_window,
                game::capture,
            )
                .chain(),
        );
    #[cfg(debug_assertions)]
    app.init_resource::<smoke::Smoke>()
        .add_systems(Update, smoke::drive.before(ui::buttons));
    app.run();
}

fn connect_requests(
    mut commands: Commands,
    mut menu: ResMut<ui::Menu>,
    mut session: ResMut<Session>,
    mut chat: ResMut<ui::Chat>,
    actors: Query<Entity, Or<(With<game::Actor>, With<game::Bubble>)>>,
) {
    if menu.leave {
        *session = Session::default();
        *chat = ui::Chat::default();
        for entity in &actors {
            commands.entity(entity).despawn();
        }
        menu.leave = false;
    }
    if let Some(mode) = menu.request.take() {
        session.link = Some(network::start(mode, menu.name.clone()));
        menu.status = "Connecting to your town...".into();
        menu.connecting = true;
        menu.dirty = true;
    }
}

fn network_events(
    mut commands: Commands,
    mut session: ResMut<Session>,
    mut menu: ResMut<ui::Menu>,
    art: Res<game::Art>,
    mut actors: Query<(Entity, &mut game::Actor)>,
    bubbles: Query<(Entity, &game::Bubble)>,
) {
    let events: Vec<_> = session
        .link
        .as_ref()
        .map(|link| {
            let receiver = link.events.lock().unwrap();
            let mut events = Vec::new();
            loop {
                match receiver.try_recv() {
                    Ok(event) => events.push(event),
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        if !events.iter().any(|event| matches!(event, Event::Error(_))) {
                            events.push(Event::Error(
                                "Connection ended. Return to the menu to rejoin.".into(),
                            ));
                        }
                        break;
                    }
                }
            }
            events
        })
        .unwrap_or_default();
    for event in events {
        match event {
            Event::Room { label, invite } => {
                session.label = label;
                session.invite = invite;
            }
            Event::Error(error) => {
                warn!("{error}");
                session.log(error.clone());
                menu.status = error;
                menu.connecting = false;
                session.connected = false;
                session.link = None;
                menu.dirty = true;
            }
            Event::Message(message) => match message {
                ServerMessage::Welcome { you, players } => {
                    session.you = Some(you);
                    session.connected = true;
                    menu.page = ui::Page::Playing;
                    menu.connecting = false;
                    menu.status.clear();
                    menu.active = None;
                    menu.dirty = true;
                    session.log("Welcome to Wind Town. Press Enter and say hello.".into());
                    info!(
                        "Connected: {} · {} players · you={you}",
                        session.label,
                        players.len()
                    );
                    for player in players.into_iter().take(16) {
                        game::spawn_actor(&mut commands, &art, player);
                    }
                }
                ServerMessage::Joined { player } => {
                    session.log(format!("{} joined the town.", player.name));
                    game::spawn_actor(&mut commands, &art, player);
                }
                ServerMessage::Moved { player } => {
                    if player.id == session.you.unwrap_or(0) {
                        continue;
                    }
                    if !player.x.is_finite() || !player.y.is_finite() {
                        continue;
                    }
                    for (_, mut actor) in &mut actors {
                        if actor.player.id == player.id {
                            actor.player = player.clone();
                        }
                    }
                }
                ServerMessage::Chat { id, text } => {
                    let text = network::clean(&text, 80);
                    if let Some((_, actor)) = actors.iter().find(|(_, actor)| actor.player.id == id)
                    {
                        session.log(format!("{}: {}", actor.player.name, text));
                        info!("Chat · {}: {text}", actor.player.name);
                        for (entity, bubble) in &bubbles {
                            if bubble.id == id {
                                commands.entity(entity).despawn();
                            }
                        }
                        game::spawn_bubble(&mut commands, &art, id, actor.position, &text);
                    }
                }
                ServerMessage::Left { id } => {
                    for (entity, actor) in &actors {
                        if actor.player.id == id {
                            session.log(format!("{} left the town.", actor.player.name));
                            commands.entity(entity).despawn();
                        }
                    }
                    for (entity, bubble) in &bubbles {
                        if bubble.id == id {
                            commands.entity(entity).despawn();
                        }
                    }
                }
            },
        }
    }
}
