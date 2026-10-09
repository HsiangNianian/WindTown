use crate::{
    Session,
    fishing::{self, Action, FISH, Fishing, Panel, Stage},
    game::{Actor, Art},
    ui::{self, Menu, Page},
};
use bevy::{
    prelude::*,
    sprite::{BorderRect, TextureSlicer},
    ui::widget::NodeImageMode,
};

#[derive(Component)]
pub(crate) struct Root;
#[derive(Component)]
pub(crate) struct ItemIcon;
#[derive(Component)]
pub(crate) struct Prompt;
#[derive(Component)]
pub(crate) enum Readout {
    Coins,
    Bait,
    Prompt,
    Notice,
    Title,
    Detail,
}
#[derive(Component)]
pub(crate) enum Fill {
    Landing,
    Tension,
    Bite,
}
#[derive(Component)]
pub(crate) enum Visual {
    Fish,
    Line,
    Float,
}

fn icon(commands: &mut Commands, parent: Entity, art: &Art, index: usize, size: f32) -> Entity {
    commands
        .spawn((
            ItemIcon,
            ImageNode::from_atlas_image(
                art.items.clone(),
                TextureAtlas {
                    layout: art.items_atlas.clone(),
                    index,
                },
            ),
            Node {
                width: px(size),
                height: px(size),
                flex_shrink: 0.0,
                ..default()
            },
            ChildOf(parent),
        ))
        .id()
}

fn frame(commands: &mut Commands, parent: Entity, image: Handle<Image>, node: Node) -> Entity {
    let entity = commands.spawn((node, ChildOf(parent))).id();
    commands.spawn((
        ImageNode::new(image).with_mode(NodeImageMode::Sliced(TextureSlicer {
            border: BorderRect::all(24.0),
            ..default()
        })),
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            ..default()
        },
        ZIndex(-1),
        ChildOf(entity),
    ));
    entity
}

fn row(commands: &mut Commands, parent: Entity, gap: f32) -> Entity {
    commands
        .spawn((
            Node {
                column_gap: px(gap),
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(parent),
        ))
        .id()
}

fn slot(
    commands: &mut Commands,
    parent: Entity,
    art: &Art,
    index: usize,
    name: &str,
    amount: &str,
    owned: bool,
) {
    let cell = frame(
        commands,
        parent,
        art.slot.clone(),
        Node {
            width: px(128),
            height: px(128),
            padding: UiRect::all(px(9)),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(3),
            ..default()
        },
    );
    let item = icon(commands, cell, art, index, 64.0);
    if !owned {
        commands.entity(item).insert(
            ImageNode::from_atlas_image(
                art.items.clone(),
                TextureAtlas {
                    layout: art.items_atlas.clone(),
                    index,
                },
            )
            .with_color(Color::srgba(0.55, 0.62, 0.57, 0.4)),
        );
    }
    ui::label(commands, cell, art, name, 16.0, ui::INK);
    ui::label(
        commands,
        cell,
        art,
        amount,
        15.0,
        if owned { ui::GREEN } else { ui::MUTED },
    );
}

fn shop_item(
    commands: &mut Commands,
    parent: Entity,
    art: &Art,
    index: usize,
    title: &str,
    detail: &str,
    action: Action,
) {
    let button = ui::button(
        commands,
        parent,
        art,
        title,
        detail,
        ui::Action::Fishing(action),
        false,
    );
    commands.entity(button).insert(Node {
        width: percent(100),
        height: px(86),
        min_height: px(86),
        padding: UiRect {
            left: px(98),
            right: px(12),
            top: px(8),
            bottom: px(8),
        },
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        row_gap: px(6),
        border: UiRect::bottom(px(3)),
        ..default()
    });
    let item = icon(commands, button, art, index, 64.0);
    commands.entity(item).insert(Node {
        position_type: PositionType::Absolute,
        left: px(16),
        top: px(9),
        width: px(64),
        height: px(64),
        ..default()
    });
}

fn meter(commands: &mut Commands, parent: Entity, art: &Art, title: &str, fill: Fill) {
    ui::label(commands, parent, art, title, 15.0, ui::INK);
    let track = commands
        .spawn((
            Node {
                width: percent(100),
                height: px(15),
                border: UiRect::all(px(3)),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(74, 93, 80)),
            BorderColor::all(ui::MUTED),
            ChildOf(parent),
        ))
        .id();
    commands.spawn((
        fill,
        Node {
            width: percent(0),
            height: percent(100),
            ..default()
        },
        BackgroundColor(ui::GREEN),
        ChildOf(track),
    ));
}

pub fn render(
    mut commands: Commands,
    menu: Res<Menu>,
    art: Res<Art>,
    mut fishing: ResMut<Fishing>,
    roots: Query<Entity, With<Root>>,
) {
    if !fishing.dirty && !menu.is_changed() {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    fishing.dirty = false;
    if menu.page != Page::Playing {
        return;
    }
    let root = commands
        .spawn((
            Root,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                ..default()
            },
            GlobalZIndex(10),
        ))
        .id();
    let wallet = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(32),
                top: px(112),
                padding: UiRect::axes(px(10), px(4)),
                column_gap: px(8),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba_u8(30, 62, 62, 225)),
            ChildOf(root),
        ))
        .id();
    for (index, readout) in [(3, Readout::Coins), (2, Readout::Bait)] {
        icon(&mut commands, wallet, &art, index, 32.0);
        let text = ui::label(&mut commands, wallet, &art, "", 21.0, ui::CREAM);
        commands.entity(text).insert(readout);
    }
    icon(&mut commands, wallet, &art, 11, 32.0);
    ui::label(&mut commands, wallet, &art, "I", 18.0, ui::CREAM);
    let notice = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(166),
                left: px(32),
                max_width: px(590),
                ..default()
            },
            ChildOf(root),
        ))
        .id();
    let text = ui::label(&mut commands, notice, &art, "", 18.0, ui::CREAM);
    commands.entity(text).insert(Readout::Notice);
    let prompt = commands
        .spawn((
            Prompt,
            Node {
                position_type: PositionType::Absolute,
                top: px(208),
                left: px(32),
                padding: UiRect::all(px(10)),
                ..default()
            },
            BackgroundColor(Color::srgba_u8(30, 62, 62, 225)),
            ChildOf(root),
        ))
        .id();
    let text = ui::label(&mut commands, prompt, &art, "", 21.0, ui::CREAM);
    commands.entity(text).insert(Readout::Prompt);
    if !fishing.modal() {
        return;
    }
    let panel = frame(
        &mut commands,
        root,
        art.panel.clone(),
        Node {
            position_type: PositionType::Absolute,
            left: px(32),
            top: px(214),
            width: px(if fishing.panel == Panel::None {
                456
            } else {
                600
            }),
            padding: UiRect::axes(px(24), px(20)),
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            ..default()
        },
    );
    match fishing.panel {
        Panel::Bag => {
            ui::label(&mut commands, panel, &art, "YOUR SATCHEL", 30.0, ui::INK);
            ui::label(
                &mut commands,
                panel,
                &art,
                "TACKLE & SUPPLIES",
                15.0,
                ui::MUTED,
            );
            let equipment = row(&mut commands, panel, 8.0);
            for (index, name, count, owned) in [
                (
                    0,
                    "Bamboo rod",
                    if fishing.progress.rod {
                        "EQUIPPED".into()
                    } else {
                        "NOT OWNED".into()
                    },
                    fishing.progress.rod,
                ),
                (
                    1,
                    "Hook",
                    if fishing.progress.hook {
                        "EQUIPPED".into()
                    } else {
                        "NOT OWNED".into()
                    },
                    fishing.progress.hook,
                ),
                (
                    2,
                    "Worm tin",
                    format!("x{}", fishing.progress.bait),
                    fishing.progress.bait > 0,
                ),
                (
                    3,
                    "Coins",
                    format!("x{}", fishing.progress.coins),
                    fishing.progress.coins > 0,
                ),
            ] {
                slot(&mut commands, equipment, &art, index, name, &count, owned);
            }
            ui::label(&mut commands, panel, &art, "TODAY'S CATCH", 15.0, ui::MUTED);
            let catches = row(&mut commands, panel, 8.0);
            for (index, (name, price)) in FISH.iter().enumerate() {
                let count = fishing.progress.catches[index];
                slot(
                    &mut commands,
                    catches,
                    &art,
                    4 + index,
                    name,
                    &format!("x{count} / {price}c"),
                    count > 0,
                );
            }
            ui::label(
                &mut commands,
                panel,
                &art,
                &format!(
                    "Catch value: {} coins at Mara's counter",
                    fishing.progress.value()
                ),
                18.0,
                ui::INK,
            );
        }
        Panel::Shop => {
            ui::label(
                &mut commands,
                panel,
                &art,
                "MARA'S TACKLE COUNTER",
                27.0,
                ui::INK,
            );
            ui::label(
                &mut commands,
                panel,
                &art,
                "Pick something for an afternoon by the sea.",
                18.0,
                ui::MUTED,
            );
            shop_item(
                &mut commands,
                panel,
                &art,
                0,
                "1  BAMBOO ROD",
                if fishing.progress.rod {
                    "EQUIPPED / Yours to keep"
                } else {
                    "45 coins / Buy once, keep forever"
                },
                Action::Rod,
            );
            shop_item(
                &mut commands,
                panel,
                &art,
                1,
                "2  BARBLESS HOOK",
                if fishing.progress.hook {
                    "EQUIPPED / Fitted to your rod"
                } else {
                    "15 coins / Reusable tackle"
                },
                Action::Hook,
            );
            shop_item(
                &mut commands,
                panel,
                &art,
                2,
                "3  WORM TIN",
                "10 coins / Five worms, five casts",
                Action::Bait,
            );
            shop_item(
                &mut commands,
                panel,
                &art,
                11,
                "4  SELL YOUR CATCH",
                &format!(
                    "{} fish / {} coins",
                    fishing.progress.catches.iter().sum::<u32>(),
                    fishing.progress.value()
                ),
                Action::Sell,
            );
        }
        Panel::None => {
            let title = ui::label(&mut commands, panel, &art, "", 24.0, ui::INK);
            commands.entity(title).insert(Readout::Title);
            let view = commands
                .spawn((
                    ImageNode::new(art.water.clone()),
                    Node {
                        width: px(384),
                        height: px(168),
                        align_self: AlignSelf::Center,
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    ChildOf(panel),
                ))
                .id();
            let index = match &fishing.stage {
                Stage::Reeling(fight) => 4 + fight.fish,
                Stage::Result {
                    fish: Some(fish), ..
                } => 4 + *fish,
                _ => 12,
            };
            commands.spawn((
                Visual::Line,
                ImageNode::solid_color(ui::CREAM),
                Node {
                    position_type: PositionType::Absolute,
                    width: px(3),
                    ..default()
                },
                ChildOf(view),
            ));
            let fish = icon(&mut commands, view, &art, index, 96.0);
            commands.entity(fish).insert((
                Visual::Fish,
                Node {
                    position_type: PositionType::Absolute,
                    width: px(96),
                    height: px(96),
                    ..default()
                },
            ));
            let bob = icon(
                &mut commands,
                view,
                &art,
                if matches!(fishing.stage, Stage::Bite(_)) {
                    14
                } else {
                    8
                },
                48.0,
            );
            commands.entity(bob).insert((
                Visual::Float,
                Node {
                    position_type: PositionType::Absolute,
                    width: px(48),
                    height: px(48),
                    ..default()
                },
            ));
            if let Stage::Result { fish: Some(_), .. } = fishing.stage {
                for left in [48, 284] {
                    let sparkle = icon(&mut commands, view, &art, 13, 48.0);
                    commands.entity(sparkle).insert(Node {
                        position_type: PositionType::Absolute,
                        left: px(left),
                        top: px(40),
                        width: px(48),
                        height: px(48),
                        ..default()
                    });
                }
            }
            let detail = ui::label(&mut commands, panel, &art, "", 18.0, ui::INK);
            commands.entity(detail).insert(Readout::Detail);
            match fishing.stage {
                Stage::Reeling(_) => {
                    meter(&mut commands, panel, &art, "REELED IN", Fill::Landing);
                    meter(
                        &mut commands,
                        panel,
                        &art,
                        "LINE STRAIN / RED: RELEASE SPACE",
                        Fill::Tension,
                    );
                }
                Stage::Bite(_) => meter(
                    &mut commands,
                    panel,
                    &art,
                    "SPACE! SET THE HOOK",
                    Fill::Bite,
                ),
                _ => {}
            }
        }
    }
    let close = ui::button(
        &mut commands,
        panel,
        &art,
        if fishing.active() {
            "ESC  PUT THE ROD AWAY"
        } else {
            "ESC  BACK TO THE TOWN"
        },
        "",
        ui::Action::Fishing(Action::Close),
        false,
    );
    commands.entity(close).insert(Node {
        width: percent(100),
        height: px(40),
        min_height: px(40),
        padding: UiRect::axes(px(12), px(6)),
        align_items: AlignItems::Center,
        ..default()
    });
}

pub fn refresh(
    time: Res<Time>,
    fishing: Res<Fishing>,
    session: Res<Session>,
    actors: Query<&Actor>,
    mut texts: Query<(&Readout, &mut Text)>,
    mut fills: Query<(&Fill, &mut Node, &mut BackgroundColor), (Without<Visual>, Without<Prompt>)>,
    mut visuals: Query<(&Visual, &mut Node), (Without<Fill>, Without<Prompt>)>,
    mut prompts: Query<&mut Node, (With<Prompt>, Without<Fill>, Without<Visual>)>,
) {
    let x = actors
        .iter()
        .find(|a| Some(a.player.id) == session.you)
        .map_or(0.0, |a| a.position.x);
    let prompt = if fishing.modal() {
        ""
    } else if fishing.indoors && (x - fishing::SHOP_EXIT).abs() < 32.0 {
        "E  LEAVE SHOP"
    } else if fishing.indoors && (x - fishing::COUNTER).abs() < 58.0 {
        "E  SHOP / SELL CATCH"
    } else if !fishing.indoors && (x - fishing::SHOP_DOOR).abs() < 28.0 {
        "E  ENTER TIDE & TACKLE"
    } else if !fishing.indoors && x >= fishing::PIER_START {
        "E  CAST INTO OPEN WATER"
    } else {
        ""
    };
    for mut node in &mut prompts {
        node.display = if prompt.is_empty() {
            Display::None
        } else {
            Display::Flex
        };
    }
    for (part, mut text) in &mut texts {
        let value = match part {
            Readout::Coins => fishing.progress.coins.to_string(),
            Readout::Bait => fishing.progress.bait.to_string(),
            Readout::Prompt => prompt.into(),
            Readout::Notice => {
                if !fishing.save_error.is_empty() {
                    fishing.save_error.clone()
                } else if fishing.notice_time > 0.0 {
                    fishing.notice.clone()
                } else {
                    String::new()
                }
            }
            Readout::Title => match &fishing.stage {
                Stage::Waiting(_) => "WATCH THE FLOAT".into(),
                Stage::Bite(_) => "BITE! PRESS SPACE!".into(),
                Stage::Reeling(fight) => if fight.pull > 0.8 {
                    "FISH DASHING / EASE OFF"
                } else {
                    "BRING IT HOME"
                }
                .into(),
                Stage::Result { fish, .. } => if fish.is_some() {
                    "A FINE CATCH!"
                } else {
                    "IT SLIPPED AWAY"
                }
                .into(),
                Stage::Idle => String::new(),
            },
            Readout::Detail => match &fishing.stage {
                Stage::Waiting(_) => "A quiet moment. Wait for the splash.".into(),
                Stage::Bite(_) => "Tap SPACE before it steals the worm!".into(),
                Stage::Reeling(_) => "Hold SPACE to reel. Release to ease the line.".into(),
                Stage::Result { message, .. } => message.clone(),
                Stage::Idle => String::new(),
            },
        };
        if **text != value {
            **text = value;
        }
    }
    for (fill, mut node, mut color) in &mut fills {
        let value = match (&fishing.stage, fill) {
            (Stage::Reeling(fight), Fill::Landing) => {
                color.0 = Color::srgb_u8(92, 153, 115);
                fight.landed
            }
            (Stage::Reeling(fight), Fill::Tension) => {
                color.0 = if fight.tension > 0.75 {
                    Color::srgb_u8(199, 73, 65)
                } else {
                    Color::srgb_u8(205, 159, 76)
                };
                fight.tension
            }
            (Stage::Bite(left), Fill::Bite) => left / 1.5,
            _ => 0.0,
        };
        node.width = percent(value.clamp(0.0, 1.0) * 100.0);
    }
    let t = time.elapsed_secs();
    let (fish_x, fish_y) = match &fishing.stage {
        Stage::Reeling(fight) => (
            42.0 + (1.0 - fight.landed) * 220.0 + (t * 9.0).sin() * fight.pull * 9.0,
            43.0 + (t * 4.0).sin() * (7.0 + fight.pull * 14.0),
        ),
        Stage::Result { fish: Some(_), .. } => (144.0, 28.0 + (t * 3.0).sin() * 5.0),
        Stage::Result { .. } => (290.0, 55.0),
        _ => (214.0 + (t * 1.5).sin() * 27.0, 58.0 + (t * 2.0).sin() * 8.0),
    };
    for (visual, mut node) in &mut visuals {
        match visual {
            Visual::Fish => {
                node.left = px(fish_x.round());
                node.top = px(fish_y.round());
            }
            Visual::Line => {
                node.left = px((fish_x + 10.0).round());
                node.top = px(5);
                node.height = px((fish_y + 49.0).round());
                node.display = if matches!(fishing.stage, Stage::Reeling(_)) {
                    Display::Flex
                } else {
                    Display::None
                };
            }
            Visual::Float => {
                node.left = px(76);
                node.top = px(2.0
                    + (t * if matches!(fishing.stage, Stage::Bite(_)) {
                        15.0
                    } else {
                        2.0
                    })
                    .sin()
                        * 3.0);
                node.display = if matches!(fishing.stage, Stage::Waiting(_) | Stage::Bite(_)) {
                    Display::Flex
                } else {
                    Display::None
                };
            }
        }
    }
}
