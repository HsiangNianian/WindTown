use crate::game::Art;
use bevy::prelude::*;

// Atlas order is shared with tools/draw_editor_icons.py.
#[derive(Clone, Copy)]
pub(crate) enum Icon {
    Brush,
    Eraser,
    Pick,
    Fill,
    Undo,
    Redo,
    Eye,
    EyeClosed,
    ZoomIn,
    ZoomOut,
    Grid,
    Guides,
    FlipH,
    FlipV,
    Swap,
    Save,
    Reload,
    Original,
    Done,
    Folder,
    Town,
    Shop,
    Left,
    Right,
    Layers,
    Tiles,
    Discard,
    KeepEditing,
    Settings,
    Language,
}

impl Icon {
    pub(crate) fn image(self, art: &Art) -> ImageNode {
        let index = self as u32;
        let x = (index % 8 * 16) as f32;
        let y = (index / 8 * 16) as f32;
        ImageNode {
            image: art.editor_icons.clone(),
            rect: Some(Rect::new(x, y, x + 16.0, y + 16.0)),
            ..default()
        }
    }
}
