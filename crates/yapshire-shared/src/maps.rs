use serde::{Deserialize, Serialize};
use std::io;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Map {
    pub width: u32,
    pub height: u32,
    pub tilewidth: u32,
    pub tileheight: u32,
    pub orientation: String,
    pub infinite: bool,
    pub tilesets: Vec<TilesetRef>,
    pub layers: Vec<Layer>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct TilesetRef {
    pub firstgid: u32,
    pub source: String,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Layer {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub width: u32,
    pub height: u32,
    pub x: f32,
    pub y: f32,
    #[serde(default)]
    pub offsetx: f32,
    #[serde(default)]
    pub offsety: f32,
    pub visible: bool,
    pub opacity: f32,
    pub data: Vec<u32>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

impl Map {
    pub fn validate(&self, width: u32, tiles: u32) -> io::Result<()> {
        // shortcut: visual maps share a flat y=0 route; add collision layers before variable terrain.
        if self.width != width
            || self.height != 17
            || self.tilewidth != 16
            || self.tileheight != 16
            || self.orientation != "orthogonal"
            || self.infinite
            || self.layers.len() != 5
            || self.tilesets.len() != 1
            || self.tilesets[0].firstgid != 1
            || self.tilesets[0].source != "harbor.tsj"
        {
            return Err(invalid(
                "Expected a fixed 16px orthogonal map with five layers and harbor.tsj",
            ));
        }
        for layer in &self.layers {
            if layer.kind != "tilelayer"
                || layer.width != self.width
                || layer.height != self.height
                || layer.x != 0.0
                || layer.y != 0.0
                || layer.offsetx != 0.0
                || layer.offsety != 0.0
                || layer.opacity != 1.0
                || layer.data.len() != (self.width * self.height) as usize
                || layer.data.iter().any(|gid| {
                    let index = gid & 0x0fff_ffff;
                    index > tiles || (*gid != 0 && index == 0) || gid & 0x1000_0000 != 0
                })
            {
                return Err(invalid(&format!(
                    "Unsupported layer or invalid tile data: {}",
                    layer.name
                )));
            }
        }
        if serde_json::to_vec(self).map_err(io::Error::other)?.len() as u64 > crate::MAX_MAP_BYTES {
            return Err(invalid("Map exceeds 256 KiB"));
        }
        Ok(())
    }
}
