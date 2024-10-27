use byteorder::{LittleEndian, ReadBytesExt};
use std::{io::Read, path::Path};

pub mod palette;
pub mod tileset;

pub struct MapData {
    pub metatiles: Vec<MapMetatileData>,
    borders: Vec<MapMetatileData>,
}

pub struct MapMetatileData {
    pub metatile_id: u16,
    collision_data: u8,
    elevation: u8,
}

impl MapData {
    pub fn from_files(
        map_path: impl AsRef<Path>,
        border_path: impl AsRef<Path>,
    ) -> std::io::Result<Self> {
        let mut map_bin = std::fs::File::open(map_path)?;
        let mut border_bin = std::fs::File::open(border_path)?;
        let mut map_data = vec![];
        let mut border_data = vec![];

        map_bin.read_to_end(&mut map_data)?;
        border_bin.read_to_end(&mut border_data)?;

        if map_data.len() % 2 == 1 || border_data.len() % 2 == 1 {
            return Err(std::io::ErrorKind::InvalidData.into());
        }

        let mut map_data_cursor = std::io::Cursor::new(&map_data);
        let metatile_data = (0..map_data.len())
            .step_by(2)
            .map_while(|_| match map_data_cursor.read_u16::<LittleEndian>() {
                Ok(metatile_data) => Some(MapMetatileData::from(metatile_data)),
                Err(_) => None,
            })
            .collect();
        let mut border_data_cursor = std::io::Cursor::new(&border_data);
        let border_data = (0..border_data.len())
            .step_by(2)
            .map_while(|_| match border_data_cursor.read_u16::<LittleEndian>() {
                Ok(metatile_data) => Some(MapMetatileData::from(metatile_data)),
                Err(_) => None,
            })
            .collect();

        Ok(MapData {
            metatiles: metatile_data,
            borders: border_data,
        })
    }
}

impl From<u16> for MapMetatileData {
    fn from(value: u16) -> Self {
        MapMetatileData {
            metatile_id: value & 0x3ff,
            collision_data: ((value & 0xc00) >> 10) as u8,
            elevation: ((value & 0xf000) >> 12) as u8,
        }
    }
}
