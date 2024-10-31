use crate::palette::{parse_all_palettes, Palette};
use byteorder::{LittleEndian, ReadBytesExt};
use image::{GrayImage, ImageBuffer, Luma, RgbImage, RgbaImage};
use png::Decoder;
use std::{
    io::{self, Read},
    path::Path,
};

#[derive(Debug)]
pub struct LayoutTileset {
    primary: Tileset,
    secondary: Tileset,
}

#[derive(Debug)]
pub struct Tileset {
    metatiles: Vec<Metatile>,
    tile_image: TilesetImage,
    palettes: Vec<Palette>,
}

#[derive(Debug)]
pub struct Metatile {
    tiles: [TileData; 8],
    _attributes: MetatileAttributes,
}

#[derive(Debug)]
pub struct MetatileAttributes {
    _layer_type: LayerType,
}

#[derive(Debug)]
pub enum LayerType {
    MiddleTop,
    BottomMiddle,
    BottomTop,
}

impl From<u32> for MetatileAttributes {
    fn from(value: u32) -> Self {
        let value = (value >> 29) & 0b011;
        let layer_type = if value == 0 {
            LayerType::MiddleTop
        } else if value == 1 {
            LayerType::BottomMiddle
        } else if value == 2 {
            LayerType::BottomTop
        } else {
            LayerType::MiddleTop
        };

        MetatileAttributes {
            _layer_type: layer_type,
        }
    }
}

#[derive(Debug)]
pub struct TileData {
    tile_id: u16,
    flip_horizontal: bool,
    flip_vertical: bool,
    palette_number: u8,
}

impl LayoutTileset {
    pub fn load_from_paths(
        primary: impl AsRef<Path>,
        secondary: impl AsRef<Path>,
    ) -> io::Result<LayoutTileset> {
        let primary = Tileset::load_from_path(primary)?;
        let secondary = Tileset::load_from_path(secondary)?;
        log::info!(
            "Primary metatiles: {}, secondary: {}",
            primary.metatiles.len(),
            secondary.metatiles.len()
        );

        Ok(LayoutTileset { primary, secondary })
    }

    pub fn get_metatile_image(&self, metatile_id: u16) -> Option<RgbImage> {
        let metatile_id = metatile_id as usize;
        let end_of_primary = self.primary.metatiles.len();
        let end_of_secondary = self.secondary.metatiles.len() + end_of_primary;
        let metatile = if metatile_id < end_of_primary {
            log::info!("Used primary metatile id {metatile_id}");
            Some(self.primary.get_metatile(metatile_id))
        } else if metatile_id >= end_of_primary && metatile_id < end_of_secondary {
            Some(self.secondary.get_metatile(metatile_id - 640))
        } else {
            None
        };

        if let Some(metatile) = metatile {
            let mut metatile_image: RgbImage = ImageBuffer::new(16, 16);

            for layer in 0..2 {
                for col in 0..2 {
                    for row in 0..2 {
                        let top_layer = layer == 1;
                        let tile_idx = (layer * 4 + row * 2 + col) as usize;

                        let tileset_tile_id = metatile.tiles[tile_idx].tile_id;
                        let tile_image = if tileset_tile_id < 640 {
                            self.primary.get_tile_image(
                                metatile.tiles[tile_idx].tile_id.into(),
                                metatile.tiles[tile_idx].flip_vertical,
                                metatile.tiles[tile_idx].flip_horizontal,
                                metatile.tiles[tile_idx].palette_number.into(),
                                &self.primary.tile_image,
                            )
                        } else {
                            self.secondary.get_tile_image(
                                (metatile.tiles[tile_idx].tile_id - 640).into(),
                                metatile.tiles[tile_idx].flip_vertical,
                                metatile.tiles[tile_idx].flip_horizontal,
                                metatile.tiles[tile_idx].palette_number.into(),
                                &self.secondary.tile_image,
                            )
                        };

                        if let Some(tile_image) = tile_image {
                            for pixel_row in 0..8 {
                                for pixel_col in 0..8 {
                                    let output_row = 8 * row + pixel_row;
                                    let output_col = 8 * col + pixel_col;
                                    const ALPHA: usize = 3;
                                    if top_layer
                                        && tile_image.get_pixel(pixel_col, pixel_row).0[ALPHA] == 0
                                    {
                                        continue;
                                    }
                                    metatile_image
                                        .get_pixel_mut(output_col, output_row)
                                        .0
                                        .copy_from_slice(
                                            &tile_image.get_pixel(pixel_col, pixel_row).0[..=2],
                                        );
                                }
                            }
                        } else {
                            log::error!("Failed to get tile image for tile id {tileset_tile_id}");
                        }
                    }
                }
            }

            Some(metatile_image)
        } else {
            None
        }
    }
}

impl Tileset {
    fn load_from_path(path: impl AsRef<Path>) -> io::Result<Self> {
        let mut metatile_file = path.as_ref().to_path_buf();
        metatile_file.push("metatiles.bin");
        let mut metatile_attrs_file = path.as_ref().to_path_buf();
        metatile_attrs_file.push("metatile_attributes.bin");
        let metatiles = parse_metatile_files(metatile_file, metatile_attrs_file)?;

        let mut tileset_png_file = path.as_ref().to_path_buf();
        tileset_png_file.push("tiles.png");
        let tile_image = parse_tileset_png(tileset_png_file)?;

        let mut palettes_dir = path.as_ref().to_path_buf();
        palettes_dir.push("palettes");
        let palettes = parse_all_palettes(palettes_dir)?;

        Ok(Tileset {
            metatiles,
            tile_image,
            palettes,
        })
    }

    fn get_metatile(&self, metatile_id: usize) -> &Metatile {
        &self.metatiles[metatile_id]
    }

    fn get_tile_image(
        &self,
        tile_id: usize,
        flip_vertical: bool,
        flip_horizontal: bool,
        palette_number: usize,
        tileset_image: &TilesetImage,
    ) -> Option<RgbaImage> {
        let gray_tile = tileset_image.get_tile(tile_id)?;
        let mut tile_image: RgbaImage = ImageBuffer::new(8, 8);
        for row in 0..8 {
            for col in 0..8 {
                let tile_row = if !flip_vertical { row } else { 7 - row };
                let tile_col = if !flip_horizontal { col } else { 7 - col };

                let palette_value = self.palettes[palette_number]
                    .get(gray_tile.get_pixel(tile_col, tile_row).0[0] as usize);
                let transparent = gray_tile.get_pixel(tile_col, tile_row).0[0] == 0;
                let alpha = if transparent { 0 } else { 255 };
                tile_image.get_pixel_mut(col, row).0 =
                    [palette_value.0, palette_value.1, palette_value.2, alpha];
            }
        }
        Some(tile_image)
    }
}

impl From<u16> for TileData {
    fn from(value: u16) -> Self {
        TileData {
            // okay so I think what is happening here is that a metatile may reference tiles in both the primary and secondary tileset
            tile_id: (value & 0x3ff), //.saturating_sub(640), // this subtraction is sus
            flip_horizontal: (value & 0x400) != 0,
            flip_vertical: (value & 0x800) != 0,
            palette_number: ((value & 0xf000) >> 12) as u8,
        }
    }
}

fn parse_metatile_files(
    metatiles_path: impl AsRef<Path>,
    attributes_path: impl AsRef<Path>,
) -> io::Result<Vec<Metatile>> {
    let mut metatile_file = std::fs::File::open(metatiles_path)?;
    let mut metatile_raw_data = vec![];
    metatile_file.read_to_end(&mut metatile_raw_data)?;

    let mut attributes_file = std::fs::File::open(attributes_path)?;
    let mut attrs_raw_data = vec![];
    attributes_file.read_to_end(&mut attrs_raw_data)?;

    const METATILE_SIZE: usize = 8 * 2;
    if metatile_raw_data.len() % METATILE_SIZE != 0 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    const ATTR_SIZE: usize = 4;
    if attrs_raw_data.len() % ATTR_SIZE != 0 {
        return Err(io::ErrorKind::InvalidData.into());
    }

    let mut metatiles = vec![];
    let mut cursor = io::Cursor::new(&metatile_raw_data);
    let mut attr_cursor = io::Cursor::new(&attrs_raw_data);
    while cursor.position() != metatile_raw_data.len() as u64 {
        let attr_data = attr_cursor.read_u32::<LittleEndian>()?;
        let attr = MetatileAttributes::from(attr_data);

        let tile_data = (0..8)
            .map(|_| {
                let tile = cursor.read_u16::<LittleEndian>()?;
                Ok(TileData::from(tile))
            })
            .collect::<io::Result<Vec<_>>>()?;
        metatiles.push(Metatile {
            tiles: tile_data.try_into().unwrap(),
            _attributes: attr,
        });
    }

    Ok(metatiles)
}

#[derive(Debug)]
pub struct TilesetImage {
    tileset_data: Vec<u8>,
    tile_width: usize,
    tile_height: usize,
}

impl TilesetImage {
    fn get_tile(&self, tile_id: usize) -> Option<GrayImage> {
        if tile_id < self.tile_width * self.tile_height {
            let mut tile_image = ImageBuffer::new(8, 8);

            let tile_x = tile_id % self.tile_width;
            let tile_y = tile_id / self.tile_width;
            for row in 0..8 {
                for col in 0..8 {
                    const PIXELS_PER_BYTE: usize = 2;
                    const TILE_PIXEL_DIM: usize = 8;
                    let tileset_pixel_x = tile_x * TILE_PIXEL_DIM + col;
                    let tileset_pixel_y = tile_y * TILE_PIXEL_DIM + row;
                    let offset = (tileset_pixel_y * (self.tile_width * TILE_PIXEL_DIM)
                        + tileset_pixel_x)
                        / PIXELS_PER_BYTE;
                    let data = if col % 2 == 0 {
                        // Odd column
                        self.tileset_data[offset] >> 4
                    } else {
                        self.tileset_data[offset] & 0xf
                    };
                    let pixel: &mut Luma<u8> = tile_image.get_pixel_mut(col as u32, row as u32);
                    pixel.0 = [data];
                }
            }

            Some(tile_image)
        } else {
            log::error!(
                "Out of range: {tile_id}, max: {}",
                self.tile_height * self.tile_width
            );
            None
        }
    }
}

fn parse_tileset_png(path: impl AsRef<Path>) -> io::Result<TilesetImage> {
    let mut decoder = Decoder::new(std::fs::File::open(path)?);
    let info = decoder.read_header_info()?;
    assert_eq!(info.bit_depth, png::BitDepth::Four);
    assert_eq!(info.width % 8, 0);
    assert_eq!(info.height % 8, 0);
    assert_eq!(info.color_type, png::ColorType::Indexed);

    let tile_width = info.width as usize / 8;
    let tile_height = info.height as usize / 8;
    let mut reader = decoder.read_info()?;
    let mut tileset_data = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut tileset_data)?;
    tileset_data.resize(info.buffer_size(), 0);
    assert_eq!(tileset_data.len(), (info.width * info.height / 2) as usize);

    // In these tile images, each pixel is 4 bits, so each byte will contain 2 pixels of data

    Ok(TilesetImage {
        tileset_data,
        tile_width,
        tile_height,
    })
}
