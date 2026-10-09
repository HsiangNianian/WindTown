use crate::{
    Session,
    network::{ClientMessage, Player},
    ui::{Chat, Menu, Page},
};
use bevy::{
    camera::{RenderTarget, Viewport, visibility::RenderLayers},
    prelude::*,
    render::{
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        view::screenshot::{Screenshot, save_to_disk},
    },
    sprite::Anchor,
    text::FontSmoothing,
};

pub const WIDTH: f32 = 480.0;
pub const HEIGHT: f32 = 270.0;
pub const WINDOW_SIZE: UVec2 = UVec2::new(1440, 810);

#[derive(Resource)]
pub struct Art {
    pub font: Handle<Font>,
    pub people: Handle<Image>,
    pub atlas: Handle<TextureAtlasLayout>,
    pub shadow: Handle<Image>,
    pub items: Handle<Image>,
    pub items_atlas: Handle<TextureAtlasLayout>,
    pub panel: Handle<Image>,
    pub slot: Handle<Image>,
    pub water: Handle<Image>,
}

#[derive(Component)]
pub struct Actor {
    pub player: Player,
    pub position: Vec2,
    velocity_y: f32,
    phase: f32,
    send_time: f32,
    last_sent: (Vec2, bool, bool, bool, bool),
}

impl Actor {
    pub(crate) fn teleport(&mut self, position: Vec2) {
        self.position = position;
        self.player.x = position.x;
        self.player.y = position.y;
        self.velocity_y = 0.0;
    }
}

#[derive(Component)]
pub(crate) struct Outside;

#[derive(Component)]
pub struct Bubble {
    pub id: u32,
    timer: Timer,
    height: f32,
}
#[derive(Component)]
pub(crate) struct WorldCamera;
#[derive(Component)]
pub(crate) struct OuterCamera;
#[derive(Component)]
pub(crate) struct Backdrop {
    base: Vec2,
    parallax: f32,
}
#[derive(Component)]
pub(crate) struct Cloud {
    x: f32,
    y: f32,
    speed: f32,
}
#[derive(Component)]
pub(crate) struct Mote {
    origin: Vec2,
    phase: f32,
}

pub fn font(art: &Art, size: f32) -> TextFont {
    TextFont {
        font: art.font.clone(),
        font_size: size,
        font_smoothing: FontSmoothing::None,
        ..default()
    }
}

pub fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let art = Art {
        font: assets.load("fonts/fusion-pixel.ttf"),
        people: assets.load("people.png"),
        atlas: layouts.add(TextureAtlasLayout::from_grid(
            UVec2::new(24, 32),
            6,
            4,
            None,
            None,
        )),
        shadow: assets.load("shadow.png"),
        items: assets.load("fishing/items.png"),
        items_atlas: layouts.add(TextureAtlasLayout::from_grid(
            UVec2::splat(32),
            4,
            4,
            None,
            None,
        )),
        panel: assets.load("fishing/frame.png"),
        slot: assets.load("fishing/slot.png"),
        water: assets.load("fishing/water.png"),
    };
    let size = Extent3d {
        width: WIDTH as u32,
        height: HEIGHT as u32,
        depth_or_array_layers: 1,
    };
    let mut canvas = Image {
        texture_descriptor: TextureDescriptor {
            label: Some("pixel canvas"),
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };
    canvas.resize(size);
    let canvas = images.add(canvas);
    commands.spawn((
        Camera2d,
        Camera {
            order: -1,
            ..default()
        },
        RenderTarget::Image(canvas.clone().into()),
        Msaa::Off,
        WorldCamera,
        Transform::from_xyz(260.0, 76.0, 0.0),
    ));
    commands.spawn((Sprite::from_image(canvas), RenderLayers::layer(1)));
    commands.spawn((
        Camera2d,
        Msaa::Off,
        OuterCamera,
        RenderLayers::layer(1),
        IsDefaultUiCamera,
    ));
    commands.spawn((
        Sprite::from_image(assets.load("sky.png")),
        Outside,
        Transform::from_xyz(260.0, 76.0, -50.0),
        Backdrop {
            base: Vec2::new(260.0, 76.0),
            parallax: 1.0,
        },
    ));
    commands.spawn((
        Sprite::from_image(assets.load("hills.png")),
        Outside,
        Transform::from_xyz(720.0, 76.0, -40.0),
        Backdrop {
            base: Vec2::new(720.0, 76.0),
            parallax: 0.68,
        },
    ));
    for (x, y, speed) in [
        (77.0, 154.0, 1.5),
        (325.0, 176.0, 0.8),
        (591.0, 144.0, 1.2),
        (884.0, 163.0, 1.0),
        (1190.0, 175.0, 1.3),
    ] {
        commands.spawn((
            Sprite::from_image(assets.load("cloud.png")),
            Outside,
            Transform::from_xyz(x, y, -45.0),
            Cloud { x, y, speed },
        ));
    }
    commands.spawn((
        Sprite {
            image: assets.load("town.png"),
            rect: Some(Rect::new(0.0, 0.0, 928.0, 192.0)),
            ..default()
        },
        Outside,
        Transform::from_xyz(464.0, 80.0, -20.0),
    ));
    for i in 0..22 {
        let origin = Vec2::new(30.0 + i as f32 * 64.0, 12.0 + (i * 17 % 62) as f32);
        commands.spawn((
            Sprite::from_color(Color::srgb_u8(246, 221, 155), Vec2::new(2.0, 1.0)),
            Transform::from_xyz(origin.x, origin.y, 9.0),
            Mote {
                origin,
                phase: i as f32 * 2.3,
            },
            Outside,
        ));
    }
    commands.insert_resource(art);
}

pub fn spawn_actor(commands: &mut Commands, art: &Art, player: Player) {
    let position = Vec2::new(player.x, player.y);
    let row = player.id as usize % 4;
    let name = crate::network::clean(&player.name, 12);
    let id = player.id;
    let entity = commands
        .spawn((
            Sprite::from_atlas_image(
                art.people.clone(),
                TextureAtlas {
                    layout: art.atlas.clone(),
                    index: row * 6,
                },
            ),
            Anchor::BOTTOM_CENTER,
            Transform::from_xyz(position.x.round(), position.y.round(), 5.0),
            Actor {
                player,
                position,
                velocity_y: 0.0,
                phase: 0.0,
                send_time: 0.0,
                last_sent: (position, false, false, false, false),
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Sprite::from_image(art.shadow.clone()),
                Transform::from_xyz(0.0, 1.0, -0.2),
            ));
            parent.spawn((
                Text2d::new(name),
                font(art, 12.0),
                TextColor(Color::srgb_u8(250, 235, 192)),
                TextBackgroundColor(Color::srgba_u8(42, 66, 56, 220)),
                Transform::from_xyz(0.0, 40.0, 0.5),
            ));
        })
        .id();
    crate::coast::rod(commands, entity, id, art);
}

fn step(position: &mut Vec2, velocity_y: &mut f32, direction: f32, run: bool, jump: bool, dt: f32) {
    position.x = (position.x + direction * if run { 105.0 } else { 62.0 } * dt)
        .clamp(12.0, crate::fishing::PIER_END - 12.0);
    if jump && position.y == 0.0 {
        *velocity_y = 174.0;
    }
    *velocity_y -= 460.0 * dt;
    position.y = (position.y + *velocity_y * dt).max(0.0);
    if position.y == 0.0 {
        *velocity_y = 0.0;
    }
}

pub fn walk(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    menu: Res<Menu>,
    chat: Res<Chat>,
    session: Res<Session>,
    fishing: Res<crate::fishing::Fishing>,
    window: Single<&Window>,
    mut actors: Query<&mut Actor>,
) {
    let dt = time.delta_secs().min(0.05);
    for mut actor in &mut actors {
        if Some(actor.player.id) != session.you {
            let target = Vec2::new(
                actor.player.x.clamp(
                    12.0,
                    if actor.player.indoors {
                        444.0
                    } else {
                        crate::fishing::PIER_END - 12.0
                    },
                ),
                actor.player.y.clamp(0.0, 96.0),
            );
            actor.position = actor.position.lerp(target, 1.0 - (-18.0 * dt).exp());
            continue;
        }
        let enabled = menu.page == Page::Playing
            && !chat.open
            && !fishing.modal()
            && session.connected
            && (window.focused
                || (cfg!(debug_assertions) && std::env::var_os("YAPSHIRE_SMOKE").is_some()));
        let direction = if enabled {
            (keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) as i8
                - keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) as i8) as f32
        } else {
            0.0
        };
        let run = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        let jump = enabled && keys.just_pressed(KeyCode::Space);
        let Actor {
            position,
            velocity_y,
            ..
        } = &mut *actor;
        step(position, velocity_y, direction, run, jump, dt);
        if fishing.indoors {
            actor.position.x = actor.position.x.clamp(38.0, 444.0);
        }
        actor.player.x = actor.position.x;
        actor.player.y = actor.position.y;
        actor.player.moving = direction != 0.0;
        if direction != 0.0 {
            actor.player.facing = direction < 0.0;
        }
        actor.send_time += dt;
        let now = (
            actor.position,
            actor.player.moving,
            actor.player.facing,
            actor.player.indoors,
            actor.player.fishing,
        );
        if actor.send_time >= 0.05 && now != actor.last_sent {
            if let Some(link) = &session.link {
                let sent = link.send.try_send(ClientMessage::Move {
                    x: actor.player.x,
                    y: actor.player.y,
                    moving: actor.player.moving,
                    facing: actor.player.facing,
                    indoors: actor.player.indoors,
                    fishing: actor.player.fishing,
                });
                if sent.is_ok() {
                    actor.last_sent = now;
                    actor.send_time = 0.0;
                }
            }
        }
    }
}

pub fn animate(
    time: Res<Time>,
    mut actors: Query<(&mut Actor, &mut Sprite, &mut Transform)>,
    mut clouds: Query<(&Cloud, &mut Transform), Without<Actor>>,
    mut motes: Query<(&Mote, &mut Transform, &mut Sprite), (Without<Actor>, Without<Cloud>)>,
) {
    let t = time.elapsed_secs();
    for (mut actor, mut sprite, mut transform) in &mut actors {
        actor.phase += time.delta_secs() * if actor.player.moving { 9.0 } else { 1.3 };
        let frame = if actor.player.y > 1.0 {
            3
        } else if actor.player.moving {
            2 + actor.phase as usize % 4
        } else {
            actor.phase as usize % 2
        };
        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = actor.player.id as usize % 4 * 6 + frame;
        }
        sprite.flip_x = actor.player.facing;
        transform.translation.x = actor.position.x.round();
        transform.translation.y = actor.position.y.round();
    }
    for (cloud, mut transform) in &mut clouds {
        transform.translation.x = (cloud.x + t * cloud.speed).rem_euclid(1700.0).round() - 80.0;
        transform.translation.y = cloud.y;
    }
    for (mote, mut transform, mut sprite) in &mut motes {
        transform.translation.x = (mote.origin.x + (t * 0.3 + mote.phase).sin() * 9.0).round();
        transform.translation.y = (mote.origin.y + (t * 0.8 + mote.phase).sin() * 5.0).round();
        sprite
            .color
            .set_alpha(0.25 + (t * 1.7 + mote.phase).sin().max(0.0) * 0.6);
    }
}

pub fn follow_camera(
    time: Res<Time>,
    session: Res<Session>,
    actors: Query<&Actor>,
    mut camera: Single<&mut Transform, (With<WorldCamera>, Without<Backdrop>)>,
    mut backdrops: Query<(&Backdrop, &mut Transform), Without<WorldCamera>>,
    mut was_indoors: Local<bool>,
) {
    let mine = actors.iter().find(|a| Some(a.player.id) == session.you);
    let indoors = mine.is_some_and(|a| a.player.indoors);
    let target = if indoors {
        240.0
    } else {
        mine.map_or(260.0, |a| a.position.x).clamp(240.0, 1200.0)
    };
    if *was_indoors != indoors {
        camera.translation.x = target;
        *was_indoors = indoors;
    }
    let x = camera.translation.x
        + (target - camera.translation.x) * (1.0 - (-6.0 * time.delta_secs()).exp());
    camera.translation.x = if (target - x).abs() < 1.0 {
        target.round()
    } else {
        x
    };
    for (backdrop, mut transform) in &mut backdrops {
        transform.translation.x =
            (backdrop.base.x + (camera.translation.x - 260.0) * backdrop.parallax).round();
        transform.translation.y = backdrop.base.y;
    }
}

fn bubble_text(text: &str) -> String {
    let mut out = String::new();
    let (mut width, mut lines) = (0, 1);
    for ch in text.chars() {
        let w = if ch.is_ascii() { 1 } else { 2 };
        if width + w > 22 {
            if lines == 3 {
                out.push('…');
                break;
            }
            out.push('\n');
            width = 0;
            lines += 1;
        }
        out.push(ch);
        width += w;
    }
    out
}

pub fn spawn_bubble(commands: &mut Commands, art: &Art, id: u32, position: Vec2, text: &str) {
    let text = bubble_text(text);
    let height = text.lines().count() as f32 * 14.0 + 12.0;
    commands
        .spawn((
            Sprite::from_color(Color::srgb_u8(250, 238, 205), Vec2::new(150.0, height)),
            Transform::from_xyz(position.x, position.y + 50.0 + height / 2.0, 20.0),
            Bubble {
                id,
                height,
                timer: Timer::from_seconds(8.0, TimerMode::Once),
            },
        ))
        .with_children(|p| {
            p.spawn((
                Text2d::new(text),
                font(art, 12.0),
                TextColor(Color::srgb_u8(51, 71, 60)),
                TextLayout::new_with_justify(Justify::Center),
                Transform::from_xyz(0.0, 1.0, 1.0),
            ));
            p.spawn((
                Sprite::from_color(Color::srgb_u8(250, 238, 205), Vec2::new(5.0, 4.0)),
                Transform::from_xyz(0.0, -height / 2.0 - 2.0, 0.0),
            ));
        });
}

pub fn bubbles(
    mut commands: Commands,
    time: Res<Time>,
    actors: Query<&Actor>,
    fishing: Res<crate::fishing::Fishing>,
    mut bubbles: Query<(Entity, &mut Bubble, &mut Transform, &mut Visibility)>,
) {
    for (entity, mut bubble, mut transform, mut visibility) in &mut bubbles {
        if bubble.timer.tick(time.delta()).is_finished() {
            commands.entity(entity).despawn();
            continue;
        }
        if let Some(actor) = actors.iter().find(|a| a.player.id == bubble.id) {
            *visibility = if actor.player.indoors == fishing.indoors {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            transform.translation.x = actor.position.x.round();
            transform.translation.y = actor.position.y.round() + 50.0 + bubble.height / 2.0;
        }
    }
}

pub fn fit_window(
    window: Single<&Window>,
    mut camera: Single<(&mut Camera, &mut Projection), With<OuterCamera>>,
    mut ui_scale: ResMut<UiScale>,
) {
    let available = window.physical_size();
    if available.min_element() == 0 {
        return;
    }
    let fit = (available.as_vec2() / Vec2::new(WIDTH, HEIGHT)).min_element();
    let scale = fit.floor().max(1.0).min(fit);
    let size = (Vec2::new(WIDTH, HEIGHT) * scale).as_uvec2();
    // The UI and pixel canvas share one centered viewport, including on HiDPI displays.
    camera.0.viewport = Some(Viewport {
        physical_position: (available - size) / 2,
        physical_size: size,
        ..default()
    });
    if let Projection::Orthographic(p) = &mut *camera.1 {
        p.scale = window.scale_factor() / scale;
    }
    ui_scale.0 = size.x as f32 / WINDOW_SIZE.x as f32 / window.scale_factor();
}

pub fn capture(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut window: Single<&mut Window>,
) {
    if keys.just_pressed(KeyCode::F11) {
        use bevy::window::{MonitorSelection, WindowMode};
        window.mode = if window.mode == WindowMode::Windowed {
            WindowMode::BorderlessFullscreen(MonitorSelection::Current)
        } else {
            window
                .resolution
                .set(WINDOW_SIZE.x as f32, WINDOW_SIZE.y as f32);
            WindowMode::Windowed
        };
    }
    if keys.just_pressed(KeyCode::F12) {
        if let Err(e) = std::fs::create_dir_all("artifacts") {
            error!("Cannot save screenshot: {e}");
            return;
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(format!("artifacts/yapshire-{stamp}.png")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scene_and_ui_share_the_same_viewport_across_display_sizes() {
        let mut app = App::new();
        app.init_resource::<UiScale>()
            .add_systems(Update, fit_window);
        let window = app.world_mut().spawn(Window::default()).id();
        let camera = app
            .world_mut()
            .spawn((
                Camera::default(),
                Projection::Orthographic(OrthographicProjection::default_2d()),
                OuterCamera,
            ))
            .id();
        for (width, height, dpi, size, position) in [
            (1440, 810, 1.0, (1440, 810), (0, 0)),
            (2560, 1440, 1.0, (2400, 1350), (80, 45)),
            (3440, 1440, 1.0, (2400, 1350), (520, 45)),
            (1080, 2560, 1.0, (960, 540), (60, 1010)),
            (3840, 2160, 2.0, (3840, 2160), (0, 0)),
            (2160, 1215, 1.5, (1920, 1080), (120, 67)),
            (320, 180, 1.0, (320, 180), (0, 0)),
        ] {
            let mut w = app.world_mut().get_mut::<Window>(window).unwrap();
            w.resolution.set_scale_factor(dpi);
            w.resolution.set_physical_resolution(width, height);
            app.update();
            let viewport = app
                .world()
                .get::<Camera>(camera)
                .unwrap()
                .viewport
                .as_ref()
                .unwrap();
            assert_eq!(viewport.physical_size, UVec2::from(size));
            assert_eq!(viewport.physical_position, UVec2::from(position));
            let ui_size =
                viewport.physical_size.as_vec2() / (dpi * app.world().resource::<UiScale>().0);
            assert!((ui_size - WINDOW_SIZE.as_vec2()).length() < 0.01);
            let Projection::Orthographic(p) = app.world().get::<Projection>(camera).unwrap() else {
                panic!("Expected orthographic camera");
            };
            let scene_size = viewport.physical_size.as_vec2() / dpi * p.scale;
            assert!((scene_size - Vec2::new(WIDTH, HEIGHT)).length() < 0.01);
        }
    }

    #[test]
    fn keyboard_events_reach_game_controls() {
        use bevy::input::{
            ButtonState, InputPlugin,
            keyboard::{Key, KeyboardInput},
        };
        let mut app = App::new();
        app.add_plugins(InputPlugin);
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::KeyD,
            logical_key: Key::Character("d".into()),
            state: ButtonState::Pressed,
            text: Some("d".into()),
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        app.update();
        assert!(
            app.world()
                .resource::<ButtonInput<KeyCode>>()
                .pressed(KeyCode::KeyD)
        );
    }
    #[test]
    fn walking_jumping_and_bounds() {
        let mut pos = Vec2::new(244.0, 0.0);
        let mut vy = 0.0;
        for _ in 0..60 {
            step(&mut pos, &mut vy, 1.0, false, false, 1.0 / 60.0);
        }
        assert!((pos.x - 306.0).abs() < 0.1);
        step(&mut pos, &mut vy, 0.0, false, true, 1.0 / 60.0);
        assert!(pos.y > 0.0);
        for _ in 0..180 {
            step(&mut pos, &mut vy, -1.0, true, false, 1.0 / 60.0);
        }
        assert_eq!(pos, Vec2::new(12.0, 0.0));
        assert_eq!(vy, 0.0);
        for _ in 0..1000 {
            step(&mut pos, &mut vy, 1.0, true, false, 1.0 / 60.0);
        }
        assert_eq!(pos.x, crate::fishing::PIER_END - 12.0);
    }
    #[test]
    fn bubble_is_readable_and_bounded() {
        let text = bubble_text(&"你好呀".repeat(40));
        assert_eq!(text.lines().count(), 3);
        assert!(text.ends_with('…'));
    }
}
