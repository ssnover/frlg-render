use byteorder::{LittleEndian, ReadBytesExt};
use std::{
    io::{self, Read},
    path::Path,
};

pub struct Layout {
    height: u32,
    width: u32,
    map_data: MapData,
}

impl Layout {
    pub fn load(
        width: u32,
        height: u32,
        map_path: impl AsRef<Path>,
        border_path: impl AsRef<Path>,
    ) -> io::Result<Self> {
        Ok(Self {
            width,
            height,
            map_data: MapData::from_files(map_path, border_path)?,
        })
    }

    fn tile_idx(&self, row: u32, col: u32) -> Option<usize> {
        if row >= self.height || col >= self.width {
            None
        } else {
            let idx = row * self.width + col;
            Some(idx as usize)
        }
    }

    pub fn get_metatile(&self, row: u32, col: u32) -> Option<MapMetatileData> {
        self.tile_idx(row, col)
            .map(|idx| self.map_data.metatiles[idx])
    }

    pub fn get_metatile_mut(&mut self, row: u32, col: u32) -> Option<&mut MapMetatileData> {
        self.tile_idx(row, col)
            .map(|idx| &mut self.map_data.metatiles[idx])
    }
}

pub struct MapData {
    pub metatiles: Vec<MapMetatileData>,
    _borders: Vec<MapMetatileData>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapMetatileData {
    pub metatile_id: u16,
    _collision_data: u8,
    _elevation: u8,
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
            _borders: border_data,
        })
    }
}

impl From<u16> for MapMetatileData {
    fn from(value: u16) -> Self {
        MapMetatileData {
            metatile_id: value & 0x03ff,
            _collision_data: ((value & 0x0c00) >> 10) as u8,
            _elevation: ((value & 0xf000) >> 12) as u8,
        }
    }
}
