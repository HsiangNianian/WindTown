use crate::i18n::{Message, tr};
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use serde::Deserialize;
use std::{io, path::Path};

pub(crate) use yapshire_shared::maps::Map;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Component)]
pub(crate) enum MapKind {
    Town,
    Shop,
}

impl MapKind {
    pub const ALL: [Self; 2] = [Self::Town, Self::Shop];

    pub fn index(self) -> usize {
        if self == Self::Town { 0 } else { 1 }
    }

    pub fn filename(self) -> &'static str {
        if self == Self::Town {
            "town.tmj"
        } else {
            "tackle-shop.tmj"
        }
    }

    pub fn title(self) -> Message {
        if self == Self::Town {
            tr("editor.town_title")
        } else {
            tr("editor.shop_title")
        }
    }
}

#[derive(Component)]
pub(crate) struct MapLayer;

#[derive(Deserialize)]
struct Tileset {
    image: String,
    imagewidth: u32,
    imageheight: u32,
    tilewidth: u32,
    tileheight: u32,
    tilecount: u32,
    columns: u32,
    margin: u32,
    spacing: u32,
    tiles: Vec<Animation>,
}

#[derive(Deserialize)]
struct Animation {
    id: u32,
    animation: Vec<Frame>,
}

#[derive(Deserialize)]
struct Frame {
    tileid: u32,
    duration: u32,
}

#[derive(Resource)]
pub(crate) struct Maps {
    pub town: Map,
    pub shop: Map,
    pub asset_path: std::path::PathBuf,
    pub revision: u64,
    tileset: Tileset,
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

impl Maps {
    pub fn world(&self) -> io::Result<yapshire_shared::World> {
        yapshire_shared::validate_tileset(&self.asset_path.join("maps"))?;
        yapshire_shared::World::new(self.town.clone(), self.shop.clone())
    }

    pub fn apply_world(&mut self, world: &yapshire_shared::World) {
        self.replace(MapKind::Town, world.town.clone());
        self.replace(MapKind::Shop, world.shop.clone());
    }

    pub fn get(&self, kind: MapKind) -> &Map {
        match kind {
            MapKind::Town => &self.town,
            MapKind::Shop => &self.shop,
        }
    }

    pub fn replace(&mut self, kind: MapKind, map: Map) {
        if self.get(kind) == &map {
            return;
        }
        self.revision += 1;
        match kind {
            MapKind::Town => self.town = map,
            MapKind::Shop => self.shop = map,
        }
    }

    pub fn tile_count(&self) -> u32 {
        self.tileset.tilecount
    }

    pub fn validate_map(&self, kind: MapKind, map: &Map) -> io::Result<()> {
        map.validate(
            if kind == MapKind::Town { 90 } else { 30 },
            self.tile_count(),
        )
    }

    pub fn load(assets: &Path) -> io::Result<Self> {
        fn read<T: serde::de::DeserializeOwned>(path: &Path) -> io::Result<T> {
            serde_json::from_slice(&std::fs::read(path)?)
                .map_err(|error| invalid(&format!("{}: {error}", path.display())))
        }
        let folder = assets.join("maps");
        let maps = Self {
            asset_path: assets.to_path_buf(),
            revision: 0,
            town: read(&folder.join("town.tmj"))?,
            shop: read(&folder.join("tackle-shop.tmj"))?,
            tileset: read(&folder.join("harbor.tsj"))?,
        };
        let set = &maps.tileset;
        if set.image != "harbor.png"
            || set.tilewidth != 16
            || set.tileheight != 16
            || set.columns != 16
            || set.imagewidth != 256
            || set.imageheight == 0
            || set.imageheight % 16 != 0
            || set.tilecount == 0
            || set.tilecount > (set.imageheight / 16) * set.columns
            || set.margin != 0
            || set.spacing != 0
        {
            return Err(invalid("Invalid harbor tileset dimensions or texture"));
        }
        for tile in &set.tiles {
            let frames = &tile.animation;
            if tile.id >= set.tilecount
                || frames.is_empty()
                || frames.iter().enumerate().any(|(i, frame)| {
                    frame.tileid >= set.tilecount
                        || frame.tileid != frames[0].tileid + i as u32
                        || frame.duration == 0
                        || frame.duration != frames[0].duration
                })
            {
                return Err(invalid(
                    "Water animations need consecutive tiles with equal frame durations",
                ));
            }
        }
        maps.town.validate(90, set.tilecount)?;
        maps.shop.validate(30, set.tilecount)?;
        Ok(maps)
    }

    pub fn spawn(&self, commands: &mut Commands, assets: &AssetServer, parent: Entity, map: &Map) {
        let texture = assets.load("maps/harbor.png");
        let size = TilemapSize {
            x: map.width,
            y: map.height,
        };
        let tile_size = TilemapTileSize { x: 16.0, y: 16.0 };
        for (depth, layer) in map.layers.iter().enumerate() {
            let entity = commands
                .spawn((MapLayer, Name::new(layer.name.clone()), ChildOf(parent)))
                .id();
            let mut storage = TileStorage::empty(size);
            for (i, &gid) in layer.data.iter().enumerate().filter(|(_, gid)| **gid != 0) {
                let pos = TilePos {
                    x: i as u32 % map.width,
                    y: map.height - 1 - i as u32 / map.width,
                };
                let index = (gid & 0x0fff_ffff) - 1;
                let mut tile = commands.spawn(TileBundle {
                    position: pos,
                    tilemap_id: TilemapId(entity),
                    texture_index: TileTextureIndex(index),
                    flip: TileFlip {
                        x: gid & 0x8000_0000 != 0,
                        y: gid & 0x4000_0000 != 0,
                        d: gid & 0x2000_0000 != 0,
                    },
                    ..default()
                });
                tile.insert(ChildOf(entity));
                if let Some(animation) = self.tileset.tiles.iter().find(|tile| tile.id == index) {
                    let first = &animation.animation[0];
                    tile.insert(AnimatedTile {
                        start: first.tileid,
                        end: first.tileid + animation.animation.len() as u32,
                        // Bevy counts whole loops per second; Tiled stores milliseconds per frame.
                        speed: 1000.0 / (first.duration as f32 * animation.animation.len() as f32),
                    });
                }
                storage.set(&pos, tile.id());
            }
            commands.entity(entity).insert(TilemapBundle {
                grid_size: tile_size.into(),
                map_type: TilemapType::Square,
                size,
                storage,
                texture: TilemapTexture::Single(texture.clone()),
                tile_size,
                transform: Transform::from_xyz(8.0, -56.0, -30.0 + depth as f32 * 6.0),
                visibility: if layer.visible {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                },
                ..default()
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_maps_have_a_continuous_route_and_aligned_shop() {
        let maps = Maps::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")).unwrap();
        for map in [&maps.town, &maps.shop] {
            let floor = &map.layers[2].data;
            let end = if map.width == 90 {
                crate::fishing::PIER_END as u32 / 16
            } else {
                map.width - 1
            };
            for x in 1..end {
                assert_ne!(
                    floor[(13 * map.width + x) as usize],
                    0,
                    "Missing floor at {x}"
                );
            }
        }
        for x in crate::fishing::PIER_END as usize / 16..90 {
            assert_eq!(
                maps.town.layers[2].data[13 * 90 + x],
                0,
                "Open water beyond the pier"
            );
        }
        let door_column = crate::fishing::SHOP_DOOR as u32 / 16;
        assert_ne!(
            maps.town.layers[3].data[(12 * 90 + door_column) as usize],
            0
        );
        assert_ne!(
            maps.shop.layers[3].data[(12 * 30 + crate::fishing::SHOP_EXIT as u32 / 16) as usize],
            0
        );
        assert!(
            maps.tileset.tiles.len() > 1,
            "Water must use animated tiles"
        );
    }

    #[test]
    fn invalid_maps_are_rejected_before_spawning() {
        let maps = Maps::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")).unwrap();
        let mut map = maps.town;
        map.layers[0].data[0] = 0x8000_0001;
        assert!(map.validate(90, maps.tileset.tilecount).is_ok());
        map.layers[0].data[0] = maps.tileset.tilecount + 1;
        assert!(map.validate(90, maps.tileset.tilecount).is_err());
        map.layers[0].data[0] = 0;
        map.layers[0].data.pop();
        assert!(map.validate(90, maps.tileset.tilecount).is_err());
        map.layers[0].data.push(0);
        map.layers[0].offsetx = 1.0;
        assert!(map.validate(90, maps.tileset.tilecount).is_err());
    }
}

// Keep Tiled's stored names intact; only known bundled layer names are localized.
pub(crate) fn layer_title(name: &str) -> Message {
    match name {
        "Water" => tr("editor.layer.water"),
        "Shore and pilings" => tr("editor.layer.shore"),
        "Terrain" => tr("editor.layer.terrain"),
        "Buildings" => tr("editor.layer.buildings"),
        "Props" => tr("editor.layer.props"),
        "Backdrop" => tr("editor.layer.backdrop"),
        "Walls" => tr("editor.layer.walls"),
        "Floor" => tr("editor.layer.floor"),
        "Furniture" => tr("editor.layer.furniture"),
        "Counter" => tr("editor.layer.counter"),
        _ => name.into(),
    }
}
