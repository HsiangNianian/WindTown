use crate::{
    editor::{Action, CANVAS, CANVAS_SIZE, Dialog, Editor, Tool},
    game::Art,
    i18n::{I18n, Message, tr},
    maps::{MapKind, Maps},
    ui::{self, CREAM, GREEN, INK, MUTED, Menu, Page},
};
use bevy::{a11y::AccessibilityNode, prelude::*, ui::FocusPolicy};

mod icons;
use crate::icons::Icon;

#[derive(Component)]
pub(crate) struct Root;
#[derive(Component)]
pub(crate) struct Cell {
    layer: usize,
    point: UVec2,
}
#[derive(Component)]
pub(crate) struct Face {
    rect: Rect,
}
#[derive(Component)]
pub(crate) struct ButtonIcon(Action);
#[derive(Component)]
pub(crate) struct Tooltip;
#[derive(Component)]
pub(crate) enum Readout {
    Title,
    Status,
    Cursor,
    Brush,
    DialogStatus,
    Tooltip,
}
#[derive(Component)]
pub(crate) struct Cursor;

const PAPER: Color = Color::srgb_u8(229, 220, 189);
const SEA: Color = Color::srgb_u8(35, 56, 57);
const GOLD: Color = Color::srgb_u8(235, 189, 96);

fn box_at(
    commands: &mut Commands,
    parent: Entity,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
) -> Entity {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(x),
                top: px(y),
                width: px(w),
                height: px(h),
                ..default()
            },
            BackgroundColor(color),
            ChildOf(parent),
        ))
        .id()
}

fn text_at(
    commands: &mut Commands,
    parent: Entity,
    art: &Art,
    value: impl Into<Message>,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
) -> Entity {
    let id = ui::label(commands, parent, art, value, size, color);
    commands.entity(id).insert(Node {
        position_type: PositionType::Absolute,
        left: px(x),
        top: px(y),
        ..default()
    });
    id
}

fn button(
    commands: &mut Commands,
    parent: Entity,
    art: &Art,
    label: impl Into<Message>,
    rect: [f32; 4],
    action: Action,
) -> Entity {
    let label = label.into();
    let [x, y, w, h] = rect;
    let id = box_at(commands, parent, x, y, w, h, PAPER);
    commands.entity(id).insert((
        Button,
        Face {
            rect: Rect::new(x, y, x + w, y + h),
        },
        action,
        Node {
            position_type: PositionType::Absolute,
            left: px(x),
            top: px(y),
            width: px(w),
            height: px(h),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: px(8),
            border: UiRect::all(px(2)),
            ..default()
        },
        BorderColor::all(MUTED),
    ));
    if let Some(icon) = icons::for_action(action) {
        let size = if label.is_empty() && h >= 38.0 {
            32
        } else {
            16
        };
        commands.spawn((
            ButtonIcon(action),
            icon.image(art),
            Node {
                width: px(size),
                height: px(size),
                flex_shrink: 0.0,
                ..default()
            },
            FocusPolicy::Pass,
            ChildOf(id),
        ));
    }
    if !label.is_empty() {
        ui::label(commands, id, art, label, 18.0, INK);
    }
    if let Some(key) = icons::shortcut(action) {
        let badge = text_at(commands, id, art, key, w - 17.0, h - 17.0, 12.0, INK);
        commands
            .entity(badge)
            .insert((BackgroundColor(CREAM), FocusPolicy::Pass));
    }
    id
}

fn tile_image(gid: u32, texture: Handle<Image>) -> (ImageNode, UiTransform) {
    let index = (gid & 0x0fff_ffff).saturating_sub(1);
    let x = (index % 16 * 16) as f32;
    let y = (index / 16 * 16) as f32;
    let h = if gid & 0x8000_0000 != 0 { -1.0 } else { 1.0 };
    let v = if gid & 0x4000_0000 != 0 { -1.0 } else { 1.0 };
    let diagonal = gid & 0x2000_0000 != 0;
    (
        ImageNode {
            image: texture,
            rect: Some(Rect::new(x, y, x + 16.0, y + 16.0)),
            ..default()
        },
        UiTransform {
            scale: if diagonal {
                Vec2::new(v, -h)
            } else {
                Vec2::new(h, v)
            },
            rotation: if diagonal {
                Rot2::FRAC_PI_2
            } else {
                Rot2::IDENTITY
            },
            ..default()
        },
    )
}

pub(crate) fn render(
    i18n: Res<I18n>,
    mut commands: Commands,
    mut editor: ResMut<Editor>,
    menu: Res<Menu>,
    art: Res<Art>,
    assets: Res<AssetServer>,
    maps: Res<Maps>,
    roots: Query<Entity, With<Root>>,
) {
    if menu.page != Page::Editor {
        for root in &roots {
            commands.entity(root).despawn();
        }
        return;
    }
    if !editor.ui_dirty && !i18n.is_changed() && !roots.is_empty() {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    editor.ui_dirty = false;
    editor.canvas_dirty = true;
    let root = commands
        .spawn((
            Root,
            GlobalZIndex(20),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                ..default()
            },
            BackgroundColor(CREAM),
        ))
        .id();
    box_at(&mut commands, root, 0.0, 0.0, 1440.0, 74.0, SEA);
    let title = text_at(&mut commands, root, &art, "", 24.0, 21.0, 30.0, CREAM);
    commands.entity(title).insert(Readout::Title);
    for (label, x, width, action) in [
        (tr("common.save"), 798.0, 110.0, Action::Save),
        (tr("common.reload"), 920.0, 120.0, Action::Reload),
        (tr("common.original"), 1052.0, 140.0, Action::Reset),
        (tr("common.done"), 1260.0, 156.0, Action::Done),
    ] {
        button(
            &mut commands,
            root,
            &art,
            label,
            [x, 18.0, width, 38.0],
            action,
        );
    }

    for (label, x, width, action) in [
        (tr("editor.town"), 24.0, 110.0, Action::Map(MapKind::Town)),
        (tr("editor.shop"), 140.0, 110.0, Action::Map(MapKind::Shop)),
        (Message::default(), 282.0, 52.0, Action::Tool(Tool::Brush)),
        (Message::default(), 342.0, 52.0, Action::Tool(Tool::Eraser)),
        (Message::default(), 402.0, 52.0, Action::Tool(Tool::Pick)),
        (Message::default(), 462.0, 52.0, Action::Tool(Tool::Fill)),
        (Message::default(), 546.0, 52.0, Action::Undo),
        (Message::default(), 606.0, 52.0, Action::Redo),
    ] {
        button(
            &mut commands,
            root,
            &art,
            label,
            [x, 90.0, width, 44.0],
            action,
        );
    }
    for x in [266.0, 530.0, 678.0] {
        box_at(&mut commands, root, x, 94.0, 2.0, 36.0, PAPER);
    }
    let (tool_name, tool_help) = match editor.tool {
        Tool::Brush => (tr("editor.brush"), tr("editor.brush_help")),
        Tool::Eraser => (tr("editor.eraser"), tr("editor.eraser_help")),
        Tool::Pick => (tr("editor.pick"), tr("editor.pick_help")),
        Tool::Fill => (tr("editor.fill"), tr("editor.fill_help")),
    };
    text_at(&mut commands, root, &art, tool_name, 700.0, 94.0, 20.0, INK);
    text_at(
        &mut commands,
        root,
        &art,
        tool_help,
        700.0,
        119.0,
        12.0,
        MUTED,
    );
    text_at(
        &mut commands,
        root,
        &art,
        tr("editor.local_map"),
        24.0,
        134.0,
        14.0,
        MUTED,
    );

    let canvas = box_at(
        &mut commands,
        root,
        CANVAS.x,
        CANVAS.y,
        CANVAS_SIZE.x,
        CANVAS_SIZE.y,
        SEA,
    );
    commands.entity(canvas).insert(Node {
        position_type: PositionType::Absolute,
        left: px(CANVAS.x),
        top: px(CANVAS.y),
        width: px(CANVAS_SIZE.x),
        height: px(CANVAS_SIZE.y),
        overflow: Overflow::clip(),
        ..default()
    });
    let size = editor.cell_size();
    let map = &editor.doc().map;
    let columns = (CANVAS_SIZE.x / size).ceil() as u32;
    let rows = (CANVAS_SIZE.y / size).ceil() as u32;
    let texture = assets.load("maps/harbor.png");
    if editor.kind == MapKind::Town {
        commands.spawn((
            ImageNode::new(assets.load("sky.png")),
            Node {
                position_type: PositionType::Absolute,
                width: px(CANVAS_SIZE.x),
                height: px(272.0 * editor.zoom as f32),
                top: px(-(editor.offset.y as f32) * size),
                ..default()
            },
            ChildOf(canvas),
        ));
    }
    for layer in 0..5 {
        // The original street illustration sits between the shore and terrain layers in-game.
        if layer == 2 && editor.kind == MapKind::Town {
            commands.spawn((
                ImageNode {
                    image: assets.load("town.png"),
                    rect: Some(Rect::new(0.0, 0.0, 928.0, 192.0)),
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    left: px(-(editor.offset.x as f32) * size),
                    top: px((2.0 - editor.offset.y as f32) * size),
                    width: px(928.0 * editor.zoom as f32),
                    height: px(192.0 * editor.zoom as f32),
                    ..default()
                },
                ChildOf(canvas),
            ));
        }
        for row in 0..rows {
            for col in 0..columns {
                let point = UVec2::new(col, row) + editor.offset;
                if point.x >= map.width || point.y >= map.height {
                    continue;
                }
                let gid = map.layers[layer].data[(point.y * map.width + point.x) as usize];
                let (image, transform) = tile_image(gid, texture.clone());
                commands.spawn((
                    Cell { layer, point },
                    image,
                    transform,
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(col as f32 * size),
                        top: px(row as f32 * size),
                        width: px(size),
                        height: px(size),
                        display: if gid == 0 || !map.layers[layer].visible {
                            Display::None
                        } else {
                            Display::Flex
                        },
                        ..default()
                    },
                    ChildOf(canvas),
                ));
            }
        }
    }
    if editor.grid {
        let color = Color::srgba_u8(240, 233, 198, 42);
        for col in 0..=columns {
            box_at(
                &mut commands,
                canvas,
                col as f32 * size,
                0.0,
                1.0,
                CANVAS_SIZE.y,
                color,
            );
        }
        for row in 0..=rows {
            box_at(
                &mut commands,
                canvas,
                0.0,
                row as f32 * size,
                CANVAS_SIZE.x,
                1.0,
                color,
            );
        }
    }
    if editor.guides {
        let floor = (13.0 - editor.offset.y as f32) * size;
        box_at(&mut commands, canvas, 0.0, floor, CANVAS_SIZE.x, 2.0, GOLD);
        let anchors: &[(f32, Message)] = if editor.kind == MapKind::Town {
            &[
                (crate::fishing::SHOP_DOOR, tr("editor.guide_door")),
                (crate::fishing::PIER_START, tr("editor.guide_cast")),
                (crate::fishing::PIER_END, tr("editor.guide_pier")),
            ]
        } else {
            &[
                (crate::fishing::SHOP_EXIT, tr("editor.guide_exit")),
                (crate::fishing::COUNTER, tr("editor.guide_counter")),
            ]
        };
        for (world_x, name) in anchors {
            let x = *world_x * editor.zoom as f32 - editor.offset.x as f32 * size;
            if !(0.0..CANVAS_SIZE.x).contains(&x) {
                continue;
            }
            box_at(&mut commands, canvas, x, floor - 48.0, 2.0, 70.0, GOLD);
            let label = text_at(
                &mut commands,
                canvas,
                &art,
                name,
                x.min(CANVAS_SIZE.x - 120.0).max(4.0),
                floor - 70.0,
                16.0,
                GOLD,
            );
            commands.entity(label).insert(BackgroundColor(SEA));
        }
    }
    let cursor = box_at(&mut commands, canvas, 0.0, 0.0, size, size, Color::NONE);
    commands.entity(cursor).insert((
        Cursor,
        BorderColor::all(GOLD),
        Node {
            position_type: PositionType::Absolute,
            width: px(size),
            height: px(size),
            border: UiRect::all(px(2)),
            display: Display::None,
            ..default()
        },
    ));

    for (label, x, w, action) in [
        (
            Message::default(),
            24.0,
            40.0,
            Action::Pan(IVec2::new(-8, 0)),
        ),
        (
            Message::default(),
            70.0,
            40.0,
            Action::Pan(IVec2::new(8, 0)),
        ),
        (Message::default(), 146.0, 40.0, Action::Zoom(-1)),
        (Message::default(), 244.0, 40.0, Action::Zoom(1)),
        (tr("editor.grid"), 308.0, 90.0, Action::Grid),
        (tr("editor.guides"), 408.0, 104.0, Action::Guides),
    ] {
        button(
            &mut commands,
            root,
            &art,
            label,
            [x, 714.0, w, 34.0],
            action,
        );
    }
    text_at(
        &mut commands,
        root,
        &art,
        &format!("{}x", editor.zoom),
        200.0,
        722.0,
        18.0,
        INK,
    );
    let coords = text_at(&mut commands, root, &art, "", 540.0, 721.0, 18.0, INK);
    commands.entity(coords).insert(Readout::Cursor);
    text_at(
        &mut commands,
        root,
        &art,
        tr("editor.controls"),
        24.0,
        756.0,
        14.0,
        MUTED,
    );
    let status = text_at(&mut commands, root, &art, "", 24.0, 780.0, 16.0, INK);
    commands.entity(status).insert((
        Readout::Status,
        Node {
            position_type: PositionType::Absolute,
            left: px(24),
            top: px(778),
            width: px(1392),
            height: px(30),
            overflow: Overflow::clip(),
            ..default()
        },
    ));

    box_at(&mut commands, root, 1004.0, 90.0, 2.0, 660.0, PAPER);
    for (icon, y) in [(Icon::Layers, 94.0), (Icon::Tiles, 298.0)] {
        commands.spawn((
            icon.image(&art),
            Node {
                position_type: PositionType::Absolute,
                left: px(1020),
                top: px(y),
                width: px(16),
                height: px(16),
                ..default()
            },
            ChildOf(root),
        ));
    }
    text_at(
        &mut commands,
        root,
        &art,
        tr("editor.layers"),
        1044.0,
        92.0,
        20.0,
        INK,
    );
    text_at(
        &mut commands,
        root,
        &art,
        tr("editor.layer_order"),
        1232.0,
        96.0,
        14.0,
        MUTED,
    );
    for (row, layer) in (0..5).rev().enumerate() {
        let y = 124.0 + row as f32 * 30.0;
        button(
            &mut commands,
            root,
            &art,
            crate::maps::layer_title(&map.layers[layer].name),
            [1020.0, y, 302.0, 26.0],
            Action::Layer(layer),
        );
        button(
            &mut commands,
            root,
            &art,
            "",
            [1330.0, y, 74.0, 26.0],
            Action::Visibility(layer),
        );
    }
    text_at(
        &mut commands,
        root,
        &art,
        tr("editor.tiles"),
        1044.0,
        296.0,
        20.0,
        INK,
    );
    let pages = maps.tile_count().div_ceil(128);
    text_at(
        &mut commands,
        root,
        &art,
        &format!("{} / {pages}", editor.palette + 1),
        1240.0,
        300.0,
        16.0,
        MUTED,
    );
    button(
        &mut commands,
        root,
        &art,
        "",
        [1326.0, 288.0, 36.0, 30.0],
        Action::Palette(-1),
    );
    button(
        &mut commands,
        root,
        &art,
        "",
        [1368.0, 288.0, 36.0, 30.0],
        Action::Palette(1),
    );
    box_at(&mut commands, root, 1018.0, 336.0, 388.0, 196.0, SEA);
    for slot in 0..128_u32 {
        let gid = editor.palette as u32 * 128 + slot + 1;
        if gid > maps.tile_count() {
            break;
        }
        let id = button(
            &mut commands,
            root,
            &art,
            "",
            [
                1020.0 + (slot % 16) as f32 * 24.0,
                338.0 + (slot / 16) as f32 * 24.0,
                24.0,
                24.0,
            ],
            Action::Tile(gid),
        );
        let (image, transform) = tile_image(gid, texture.clone());
        commands.spawn((
            image,
            transform,
            Node {
                width: px(16),
                height: px(16),
                flex_shrink: 0.0,
                ..default()
            },
            ChildOf(id),
        ));
    }
    let preview = box_at(&mut commands, root, 1020.0, 556.0, 64.0, 64.0, SEA);
    let (image, transform) = tile_image(editor.tile, texture);
    commands.spawn((
        image,
        transform,
        Node {
            width: px(64),
            height: px(64),
            ..default()
        },
        ChildOf(preview),
    ));
    let brush = text_at(&mut commands, root, &art, "", 1100.0, 558.0, 18.0, INK);
    commands.entity(brush).insert(Readout::Brush);
    button(
        &mut commands,
        root,
        &art,
        "H",
        [1100.0, 588.0, 94.0, 30.0],
        Action::Flip(0x8000_0000),
    );
    button(
        &mut commands,
        root,
        &art,
        "V",
        [1202.0, 588.0, 94.0, 30.0],
        Action::Flip(0x4000_0000),
    );
    button(
        &mut commands,
        root,
        &art,
        tr("editor.swap"),
        [1304.0, 588.0, 100.0, 30.0],
        Action::Flip(0x2000_0000),
    );
    text_at(
        &mut commands,
        root,
        &art,
        tr("editor.make_place"),
        1020.0,
        644.0,
        14.0,
        GREEN,
    );
    text_at(
        &mut commands,
        root,
        &art,
        tr("editor.guide_help"),
        1020.0,
        674.0,
        18.0,
        MUTED,
    );

    button(
        &mut commands,
        root,
        &art,
        tr("editor.files"),
        [1020.0, 738.0, 180.0, 30.0],
        Action::Folder,
    );
    if let Some(dialog) = editor.dialog {
        let shade = box_at(
            &mut commands,
            root,
            0.0,
            0.0,
            1440.0,
            810.0,
            Color::srgba_u8(24, 42, 40, 210),
        );
        let panel = box_at(&mut commands, shade, 410.0, 248.0, 620.0, 314.0, CREAM);
        text_at(
            &mut commands,
            panel,
            &art,
            tr("editor.unsaved_title"),
            28.0,
            26.0,
            30.0,
            INK,
        );
        let description = match dialog {
            Dialog::Leave(_) => tr("editor.unsaved_leave"),
            Dialog::Reload => tr("editor.unsaved_reload"),
        };
        text_at(
            &mut commands,
            panel,
            &art,
            description,
            28.0,
            86.0,
            18.0,
            INK,
        );
        if matches!(dialog, Dialog::Leave(_)) {
            button(
                &mut commands,
                panel,
                &art,
                tr("editor.save_leave"),
                [28.0, 160.0, 176.0, 42.0],
                Action::SaveLeave,
            );
        }
        button(
            &mut commands,
            panel,
            &art,
            if matches!(dialog, Dialog::Reload) {
                tr("common.reload")
            } else {
                tr("editor.discard")
            },
            [216.0, 160.0, 160.0, 42.0],
            Action::Discard,
        );
        button(
            &mut commands,
            panel,
            &art,
            tr("editor.keep_editing"),
            [388.0, 160.0, 204.0, 42.0],
            Action::Cancel,
        );
        let status = text_at(&mut commands, panel, &art, "", 28.0, 222.0, 16.0, GREEN);
        commands.entity(status).insert((
            Readout::DialogStatus,
            Node {
                position_type: PositionType::Absolute,
                left: px(28),
                top: px(222),
                width: px(564),
                ..default()
            },
        ));
    }
    let tooltip = box_at(&mut commands, root, 24.0, 142.0, 472.0, 40.0, SEA);
    commands.entity(tooltip).insert((
        Tooltip,
        GlobalZIndex(30),
        FocusPolicy::Pass,
        BorderColor::all(GOLD),
        Node {
            position_type: PositionType::Absolute,
            width: px(472),
            height: px(40),
            padding: UiRect::axes(px(12), px(8)),
            align_items: AlignItems::Center,
            border: UiRect::all(px(2)),
            display: Display::None,
            ..default()
        },
    ));
    let hint = ui::label(&mut commands, tooltip, &art, "", 16.0, CREAM);
    commands
        .entity(hint)
        .insert((Readout::Tooltip, FocusPolicy::Pass));
}

pub(crate) fn refresh(
    i18n: Res<I18n>,
    mut editor: ResMut<Editor>,
    menu: Res<Menu>,
    art: Res<Art>,
    mut cells: Query<
        (&Cell, &mut ImageNode, &mut UiTransform, &mut Node),
        (Without<Cursor>, Without<Tooltip>),
    >,
    mut cursor: Query<&mut Node, (With<Cursor>, Without<Cell>, Without<Tooltip>)>,
    mut tooltips: Query<&mut Node, (With<Tooltip>, Without<Cell>, Without<Cursor>)>,
    mut images: Query<(&ButtonIcon, &mut ImageNode), Without<Cell>>,
    mut buttons: Query<
        (
            &Interaction,
            &Action,
            &Face,
            &mut BackgroundColor,
            &mut BorderColor,
            Option<&mut AccessibilityNode>,
        ),
        With<Face>,
    >,
    mut texts: Query<(&Readout, &mut Text)>,
) {
    if menu.page != Page::Editor {
        return;
    }
    if editor.canvas_dirty {
        let map = &editor.doc().map;
        for (cell, mut image, mut transform, mut node) in &mut cells {
            let layer = &map.layers[cell.layer];
            let gid = layer.data[(cell.point.y * map.width + cell.point.x) as usize];
            let (new_image, new_transform) = tile_image(gid, image.image.clone());
            *image = new_image;
            *transform = new_transform;
            node.display = if gid == 0 || !layer.visible {
                Display::None
            } else {
                Display::Flex
            };
        }
        editor.canvas_dirty = false;
    }
    for mut node in &mut cursor {
        node.display = if editor.hover.is_some() && editor.dialog.is_none() {
            Display::Flex
        } else {
            Display::None
        };
        if let Some(cell) = editor.hover {
            node.left = px(cell.x.saturating_sub(editor.offset.x) as f32 * editor.cell_size());
            node.top = px(cell.y.saturating_sub(editor.offset.y) as f32 * editor.cell_size());
        }
    }
    let enabled = |action: Action| match action {
        Action::Undo => editor.doc().can_undo(),
        Action::Redo => editor.doc().can_redo(),
        Action::Zoom(n) => {
            if n > 0 {
                editor.zoom < 3
            } else {
                editor.zoom > 1
            }
        }
        _ => true,
    };
    let mut hovered = None;
    for (interaction, action, face, mut color, mut border, accessible) in &mut buttons {
        let selected = match *action {
            Action::Map(kind) => kind == editor.kind,
            Action::Layer(layer) => layer == editor.layer,
            Action::Visibility(layer) => editor.doc().map.layers[layer].visible,
            Action::Tile(gid) => gid == editor.tile & 0x0fff_ffff,
            Action::Tool(tool) => tool == editor.tool,
            Action::Flip(flag) => editor.tile & flag != 0,
            Action::Grid => editor.grid,
            Action::Guides => editor.guides,
            Action::Save | Action::SaveLeave => true,
            _ => false,
        };
        let available = enabled(*action);
        if let Some(mut accessible) = accessible {
            let hint = icons::hint(*action, &editor).render(&i18n);
            if accessible.label() != Some(&hint) {
                accessible.set_label(hint);
            }
            if accessible.is_disabled() == available {
                if available {
                    accessible.clear_disabled();
                } else {
                    accessible.set_disabled();
                }
            }
        }
        if *interaction == Interaction::Hovered && editor.dialog.is_none() {
            hovered = Some((*action, face.rect));
        }
        *color = BackgroundColor(if !available {
            Color::srgb_u8(231, 226, 203)
        } else if *interaction != Interaction::None {
            Color::srgb_u8(194, 199, 146)
        } else if selected {
            Color::srgb_u8(173, 192, 149)
        } else {
            PAPER
        });
        *border = BorderColor::all(if selected {
            GREEN
        } else {
            Color::srgb_u8(201, 193, 161)
        });
    }
    for (icon, mut image) in &mut images {
        let tint = Color::WHITE.with_alpha(if enabled(icon.0) { 1.0 } else { 0.35 });
        if image.color != tint {
            image.color = tint;
        }
        if let Action::Visibility(layer) = icon.0 {
            let rect = if editor.doc().map.layers[layer].visible {
                Icon::Eye
            } else {
                Icon::EyeClosed
            }
            .image(&art)
            .rect;
            if image.rect != rect {
                image.rect = rect;
            }
        }
    }
    for mut node in &mut tooltips {
        node.display = if hovered.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if let Some((_, rect)) = hovered {
            node.left = px(rect.min.x.clamp(24.0, 944.0));
            node.top = px(if rect.max.y < 700.0 {
                rect.max.y + 8.0
            } else {
                rect.min.y - 48.0
            });
        }
    }
    for (readout, mut text) in &mut texts {
        let value: Message = match readout {
            Readout::Title => tr("editor.title")
                .arg("map", editor.kind.title())
                .arg("dirty", if editor.dirty() { " *" } else { "" }),
            Readout::Status | Readout::DialogStatus => editor.status.clone(),
            Readout::Tooltip => hovered
                .map(|(action, _)| icons::hint(action, &editor))
                .unwrap_or_default(),
            Readout::Cursor => editor.hover.map_or_else(
                || {
                    tr("editor.size")
                        .arg("width", editor.doc().map.width.to_string())
                        .arg("height", editor.doc().map.height.to_string())
                },
                |p| {
                    tr("editor.cursor")
                        .arg("col", format!("{:02}", p.x + 1))
                        .arg("row", format!("{:02}", p.y + 1))
                        .arg(
                            "layer",
                            crate::maps::layer_title(&editor.doc().map.layers[editor.layer].name),
                        )
                },
            ),
            Readout::Brush => tr("editor.brush_readout")
                .arg("tile", format!("{:03}", editor.tile & 0x0fff_ffff))
                .arg(
                    "tool",
                    match editor.tool {
                        Tool::Brush => tr("editor.brush"),
                        Tool::Eraser => tr("editor.eraser"),
                        Tool::Pick => tr("editor.pick"),
                        Tool::Fill => tr("editor.fill"),
                    },
                ),
        };
        let value = value.render(&i18n);
        if **text != value {
            **text = value;
        }
    }
}
