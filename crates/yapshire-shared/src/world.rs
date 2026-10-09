use crate::maps::Map;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{self, Read},
    path::Path,
};

pub const PROTOCOL_VERSION: u32 = 2;
pub const WORLD_FORMAT: u32 = 1;
pub const TILE_COUNT: u32 = 359;
pub const MAX_MAP_BYTES: u64 = 256 * 1024;
pub const MAX_WORLD_BYTES: usize = 512 * 1024;
pub const TOWN: &str = include_str!("../../../assets/maps/town.tmj");
pub const SHOP: &str = include_str!("../../../assets/maps/tackle-shop.tmj");
pub const TILESET: &[u8] = include_bytes!("../../../assets/maps/harbor.tsj");
pub const TEXTURE: &[u8] = include_bytes!("../../../assets/maps/harbor.png");

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct World {
    pub format: u32,
    pub tileset: String,
    pub revision: String,
    pub town: Map,
    pub shop: Map,
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

pub fn tileset_revision() -> String {
    let mut hash = Sha256::new();
    hash.update(TILESET);
    hash.update(TEXTURE);
    format!("harbor:{:x}", hash.finalize())
}

/// Map layouts can change, but every player must render the same tile palette.
pub fn validate_tileset(folder: &Path) -> io::Result<()> {
    for (name, expected) in [("harbor.tsj", TILESET), ("harbor.png", TEXTURE)] {
        let mut bytes = Vec::new();
        std::fs::File::open(folder.join(name))?
            .take(expected.len() as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes != expected {
            return Err(invalid(
                "Custom tilesets are not supported in multiplayer; restore the bundled harbor.tsj and harbor.png",
            ));
        }
    }
    Ok(())
}

impl World {
    pub fn new(town: Map, shop: Map) -> io::Result<Self> {
        let mut world = Self {
            format: WORLD_FORMAT,
            tileset: tileset_revision(),
            revision: String::new(),
            town,
            shop,
        };
        world.revision = world.content_hash()?;
        world.validate()?;
        Ok(world)
    }

    pub fn bundled() -> Self {
        Self::new(
            serde_json::from_str(TOWN).expect("Bundled town"),
            serde_json::from_str(SHOP).expect("Bundled shop"),
        )
        .expect("Valid bundled world")
    }

    pub fn load(folder: &Path) -> io::Result<Self> {
        if folder.join("harbor.tsj").exists() || folder.join("harbor.png").exists() {
            validate_tileset(folder)?;
        }
        fn map(path: &Path) -> io::Result<Map> {
            let mut bytes = Vec::new();
            std::fs::File::open(path)?
                .take(MAX_MAP_BYTES + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() as u64 > MAX_MAP_BYTES {
                return Err(invalid("Map exceeds 256 KiB"));
            }
            serde_json::from_slice(&bytes).map_err(io::Error::other)
        }
        Self::new(
            map(&folder.join("town.tmj"))?,
            map(&folder.join("tackle-shop.tmj"))?,
        )
    }

    fn content_hash(&self) -> io::Result<String> {
        let bytes = serde_json::to_vec(&(self.format, &self.tileset, &self.town, &self.shop))
            .map_err(io::Error::other)?;
        if bytes.len() > MAX_WORLD_BYTES - 1024 {
            return Err(invalid("World exceeds the transfer limit"));
        }
        Ok(format!("{:x}", Sha256::digest(&bytes)))
    }

    pub fn validate(&self) -> io::Result<()> {
        if self.format != WORLD_FORMAT || self.tileset != tileset_revision() {
            return Err(invalid(
                "Unsupported map format or tileset; update the game and server together",
            ));
        }
        self.town.validate(90, TILE_COUNT)?;
        self.shop.validate(30, TILE_COUNT)?;
        if self.revision != self.content_hash()? {
            return Err(invalid("World revision does not match its maps"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn edited_tiled_maps_round_trip_and_tampering_is_rejected() {
        let mut original = World::bundled();
        original.town.layers[4].data[500] = 0x8000_005d;
        assert!(original.validate().is_err());
        let world = World::new(original.town, original.shop).unwrap();
        let decoded: World = serde_json::from_slice(&serde_json::to_vec(&world).unwrap()).unwrap();
        decoded.validate().unwrap();
        assert_eq!(decoded.town.layers[4].data[500], 0x8000_005d);
        let mut bad = decoded.clone();
        bad.town.tilesets[0].source = "../../outside.tsj".into();
        assert!(World::new(bad.town, bad.shop).is_err());
        let mut bad = decoded;
        bad.tileset = "other".into();
        assert!(bad.validate().is_err());
    }
}
