use crate::{
    Session,
    fishing::{Fishing, Stage},
    game::{Actor, Art, Outside, font},
    i18n::{Localized, Message, tr},
    maps::Maps,
};
use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct Interior;
#[derive(Component)]
pub(crate) struct Rod(u32);

#[derive(Clone, Copy)]
enum Part {
    Pole(u8),
    Line(u8),
    Leader,
    Float,
    Ripple,
    Fish,
    Splash,
    Bite,
    Spark,
}
#[derive(Component)]
pub(crate) struct RigPart {
    owner: u32,
    part: Part,
}

pub fn rod(commands: &mut Commands, parent: Entity, id: u32, art: &Art) {
    let root = commands
        .spawn((
            Rod(id),
            Transform::default(),
            Visibility::Hidden,
            ChildOf(parent),
        ))
        .id();
    for part in (0..6)
        .map(Part::Pole)
        .chain((0..10).map(Part::Line))
        .chain([
            Part::Leader,
            Part::Float,
            Part::Ripple,
            Part::Fish,
            Part::Splash,
            Part::Bite,
            Part::Spark,
        ])
    {
        let icon = match part {
            Part::Float => Some(8),
            Part::Ripple => Some(9),
            Part::Fish => Some(12),
            Part::Splash => Some(10),
            Part::Bite => Some(14),
            Part::Spark => Some(13),
            _ => None,
        };
        let sprite = if let Some(index) = icon {
            Sprite::from_atlas_image(
                art.items.clone(),
                TextureAtlas {
                    layout: art.items_atlas.clone(),
                    index,
                },
            )
        } else {
            Sprite::from_color(color(0xebd6a7), Vec2::ONE)
        };
        commands.spawn((
            sprite,
            RigPart { owner: id, part },
            Transform::default(),
            Visibility::Inherited,
            ChildOf(root),
        ));
    }
}

fn curve(start: Vec2, control: Vec2, end: Vec2, t: f32) -> Vec2 {
    start.lerp(control, t).lerp(control.lerp(end, t), t)
}

fn endpoints(player_x: f32, tension: f32) -> (Vec2, Vec2) {
    (
        Vec2::new(37.0, 44.0 - tension * 10.0),
        Vec2::new(1408.0 - player_x, -18.0),
    )
}

fn beam(
    sprite: &mut Sprite,
    transform: &mut Transform,
    start: Vec2,
    end: Vec2,
    width: f32,
    z: f32,
) {
    let delta = end - start;
    sprite.custom_size = Some(Vec2::new(delta.length().max(1.0), width));
    transform.translation = start.lerp(end, 0.5).extend(z);
    transform.rotation = Quat::from_rotation_z(delta.y.atan2(delta.x));
}

pub fn animate_rig(
    time: Res<Time>,
    session: Res<Session>,
    fishing: Res<Fishing>,
    actors: Query<&Actor>,
    mut parts: Query<(&RigPart, &mut Sprite, &mut Transform, &mut Visibility)>,
) {
    let remote = Stage::Waiting(0.0);
    for (rig, mut sprite, mut transform, mut visibility) in &mut parts {
        let Some(actor) = actors.iter().find(|a| a.player.id == rig.owner) else {
            continue;
        };
        let mine = Some(rig.owner) == session.you;
        let stage = if mine { &fishing.stage } else { &remote };
        let t = time.elapsed_secs() + rig.owner as f32 % 7.0;
        let age = if mine { fishing.anim_time } else { 10.0 };
        let (tension, landed, pull) = if let Stage::Reeling(fight) = stage {
            (fight.tension, fight.landed, fight.pull)
        } else {
            (0.0, 0.0, 0.0)
        };
        let (tip, target) = endpoints(actor.position.x, tension);
        let hand = Vec2::new(7.0, 17.0);
        let cast = (age / 0.65).clamp(0.0, 1.0);
        let mut float =
            hand.lerp(target, cast) + Vec2::Y * (cast * std::f32::consts::PI).sin() * 45.0;
        float.x -= landed * 12.0;
        float.y += (t * if matches!(stage, Stage::Bite(_)) {
            14.0
        } else {
            2.6
        })
        .sin()
            * 1.5;
        let caught = matches!(stage, Stage::Result { fish: Some(_), .. });
        let fish = if caught {
            let travel = (age / 0.6).clamp(0.0, 1.0);
            Vec2::new(target.x, -40.0).lerp(Vec2::new(20.0, 47.0), travel)
                + Vec2::Y * (travel * std::f32::consts::PI).sin() * 30.0
        } else {
            Vec2::new(
                float.x + (t * 4.0).sin() * (3.0 + pull * 4.0),
                -39.0 + (t * 3.0).sin() * 4.0,
            )
        };
        let control = tip.lerp(float, 0.5) - Vec2::Y * (1.0 - tension) * 9.0;
        *visibility = Visibility::Inherited;
        sprite.color = Color::WHITE;
        transform.rotation = Quat::IDENTITY;
        match rig.part {
            Part::Pole(i) => {
                beam(
                    &mut sprite,
                    &mut transform,
                    curve(hand, Vec2::new(14.0, 48.0), tip, i as f32 / 6.0),
                    curve(hand, Vec2::new(14.0, 48.0), tip, (i + 1) as f32 / 6.0),
                    1.5,
                    0.6,
                );
                sprite.color = color(if i == 0 { 0x8e6950 } else { 0xd8bc82 });
            }
            Part::Line(i) => {
                beam(
                    &mut sprite,
                    &mut transform,
                    curve(
                        tip,
                        control,
                        if caught { fish } else { float },
                        i as f32 / 10.0,
                    ),
                    curve(
                        tip,
                        control,
                        if caught { fish } else { float },
                        (i + 1) as f32 / 10.0,
                    ),
                    0.8,
                    0.7,
                );
                sprite.color = color(if tension > 0.75 { 0xeaa582 } else { 0xe8dfb4 });
            }
            Part::Leader => {
                beam(
                    &mut sprite,
                    &mut transform,
                    float,
                    fish + Vec2::new(-9.0, 0.0),
                    0.6,
                    0.3,
                );
                sprite.color = Color::srgba_u8(198, 217, 192, 160);
                if !matches!(stage, Stage::Reeling(_)) {
                    *visibility = Visibility::Hidden;
                }
            }
            Part::Float => {
                transform.translation = float.round().extend(0.9);
                sprite.custom_size = Some(Vec2::splat(24.0));
                if caught {
                    *visibility = Visibility::Hidden;
                }
            }
            Part::Ripple => {
                transform.translation = (float + Vec2::new(0.0, -2.0)).round().extend(0.1);
                sprite.custom_size = Some(Vec2::new(30.0 + (t * 2.6).sin() * 4.0, 22.0));
                if cast < 1.0 || caught {
                    *visibility = Visibility::Hidden;
                }
            }
            Part::Fish => {
                transform.translation = fish.round().extend(0.4);
                sprite.custom_size = Some(Vec2::splat(if caught { 36.0 } else { 28.0 }));
                let species = match stage {
                    Stage::Reeling(fight) => Some(fight.fish),
                    Stage::Result { fish, .. } => *fish,
                    _ => None,
                };
                if let Some(atlas) = &mut sprite.texture_atlas {
                    atlas.index = species.map_or(12, |i| 4 + i);
                }
                if !caught {
                    sprite.color = Color::srgba(0.8, 0.95, 0.9, 0.7);
                }
                if cast < 1.0 && !caught {
                    *visibility = Visibility::Hidden;
                }
            }
            Part::Splash => {
                transform.translation = (float + Vec2::Y * 5.0).round().extend(0.8);
                sprite.custom_size = Some(Vec2::splat(27.0 + (t * 12.0).sin() * 3.0));
                if !(matches!(stage, Stage::Bite(_)) || (pull > 0.8 && (t * 5.0).sin() > 0.0)) {
                    *visibility = Visibility::Hidden;
                }
            }
            Part::Bite => {
                transform.translation = (float + Vec2::Y * 24.0).round().extend(1.0);
                sprite.custom_size = Some(Vec2::splat(24.0));
                if !matches!(stage, Stage::Bite(_)) {
                    *visibility = Visibility::Hidden;
                }
            }
            Part::Spark => {
                transform.translation = (fish + Vec2::new(18.0, (t * 4.0).sin() * 5.0))
                    .round()
                    .extend(0.8);
                sprite.custom_size = Some(Vec2::splat(14.0));
                if !caught {
                    *visibility = Visibility::Hidden;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fishing_line_clears_the_deck_and_float_stays_in_open_water() {
        for x in [crate::fishing::PIER_START, crate::fishing::PIER_END - 12.0] {
            for tension in [0.0, 0.5, 1.0] {
                let (tip, float) = endpoints(x, tension);
                let control = tip.lerp(float, 0.5) - Vec2::Y * (1.0 - tension) * 9.0;
                assert!(x + float.x > crate::fishing::PIER_END);
                assert!(x + float.x < 1440.0);
                for i in 0..=100 {
                    let p = curve(tip, control, float, i as f32 / 100.0);
                    if p.y <= 0.0 {
                        assert!(x + p.x > crate::fishing::PIER_END);
                    }
                }
            }
        }
    }
}

fn color(hex: u32) -> Color {
    Color::srgb_u8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

fn text(
    commands: &mut Commands,
    parent: Entity,
    art: &Art,
    value: impl Into<Message>,
    x: f32,
    y: f32,
    size: f32,
    hex: u32,
) {
    let value = value.into();
    commands.spawn((
        Text2d::new(value.to_string()),
        Localized(value),
        font(art, size),
        TextColor(color(hex)),
        Transform::from_xyz(x, y, 3.5),
        ChildOf(parent),
    ));
}

pub fn setup(mut commands: Commands, art: Res<Art>, assets: Res<AssetServer>, maps: Res<Maps>) {
    let outside = commands
        .spawn((
            Outside,
            crate::maps::MapKind::Town,
            Transform::default(),
            Visibility::Inherited,
        ))
        .id();
    maps.spawn(&mut commands, &assets, outside, &maps.town);
    let shop = commands
        .spawn((
            Interior,
            crate::maps::MapKind::Shop,
            Transform::default(),
            Visibility::Hidden,
        ))
        .id();
    maps.spawn(&mut commands, &assets, shop, &maps.shop);
    text(
        &mut commands,
        shop,
        &art,
        tr("world.shop"),
        240.,
        180.,
        12.,
        0xf0ddaf,
    );
    text(
        &mut commands,
        shop,
        &art,
        tr("world.shop_hint"),
        240.,
        160.,
        12.,
        0xc7c9a1,
    );
    commands.spawn((
        Sprite::from_atlas_image(
            art.people.clone(),
            TextureAtlas {
                layout: art.atlas.clone(),
                index: 6,
            },
        ),
        Transform::from_xyz(292., 20., -7.0),
        ChildOf(shop),
    ));
    text(
        &mut commands,
        shop,
        &art,
        tr("world.mara"),
        292.,
        49.,
        12.,
        0x365e59,
    );
}

pub(crate) fn reload_maps(
    mut commands: Commands,
    maps: Res<Maps>,
    assets: Res<AssetServer>,
    roots: Query<(Entity, &crate::maps::MapKind)>,
    layers: Query<Entity, With<crate::maps::MapLayer>>,
    mut revision: Local<u64>,
) {
    if *revision == maps.revision {
        return;
    }
    *revision = maps.revision;
    for layer in &layers {
        commands.entity(layer).despawn();
    }
    for (root, kind) in &roots {
        maps.spawn(&mut commands, &assets, root, maps.get(*kind));
    }
}

pub fn animate(
    fishing: Res<Fishing>,
    session: Res<Session>,
    actors: Query<&Actor>,
    mut visible: Query<(
        &mut Visibility,
        Option<&Outside>,
        Option<&Interior>,
        Option<&Actor>,
        Option<&Rod>,
    )>,
) {
    for (mut visibility, outside, interior, actor, rod) in &mut visible {
        let show = if outside.is_some() {
            Some(!fishing.indoors)
        } else if interior.is_some() {
            Some(fishing.indoors)
        } else if let Some(actor) = actor {
            Some(actor.player.indoors == fishing.indoors)
        } else if let Some(Rod(id)) = rod {
            Some(actors.iter().any(|a| {
                a.player.id == *id
                    && !a.player.indoors
                    && (a.player.fishing
                        || (Some(*id) == session.you
                            && matches!(fishing.stage, Stage::Result { fish: Some(_), .. })))
            }))
        } else {
            None
        };
        if let Some(show) = show {
            *visibility = if show {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}
