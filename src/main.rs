use convert_case::Casing;
use frlg_render::{tileset, MapData};
use image::{ImageBuffer, RgbImage};
use serde::Deserialize;
use std::fs::File;
use std::io;

#[derive(Debug, Clone, Deserialize)]
struct LayoutsTable {
    layouts_table_label: String,
    layouts: Vec<Layout>,
}

#[derive(Debug, Clone, Deserialize)]
struct Layout {
    id: String,
    name: String,
    width: u32,
    height: u32,
    border_width: u32,
    border_height: u32,
    primary_tileset: String,
    secondary_tileset: String,
    border_filepath: String,
    blockdata_filepath: String,
}

const LAYOUTS_FILE: &str = concat!(env!("PRET_ROOT"), "/data/layouts/layouts.json");
const BUILDINGS_METATILE_DIR: &str = concat!(env!("PRET_ROOT"), "/data/tilesets/primary/building");
const POWER_PLANT_METATILE_DIR: &str =
    concat!(env!("PRET_ROOT"), "/data/tilesets/secondary/power_plant");

fn main() -> io::Result<()> {
    let layouts = {
        let mut file = File::open(LAYOUTS_FILE)?;
        let layouts_table: LayoutsTable = serde_json::from_reader(file).unwrap();
        layouts_table.layouts
    };

    let layout = layouts
        .into_iter()
        .find(|layout| layout.id.as_str() == "LAYOUT_POWER_PLANT")
        .unwrap();
    let name = layout.name.strip_suffix("_Layout").unwrap();
    println!("{:#?}", layout);
    let primary = layout
        .primary_tileset
        .strip_prefix("gTileset_")
        .unwrap()
        .to_ascii_lowercase();
    let secondary = tileset_dir(layout.secondary_tileset.strip_prefix("gTileset_").unwrap());
    let primary_tileset_dir = format!("{}/data/tilesets/primary/{primary}", env!("PRET_ROOT"));
    let secondary_tileset_dir =
        format!("{}/data/tilesets/secondary/{secondary}", env!("PRET_ROOT"));

    let map_data = MapData::from_files(
        format!("{}/{}", env!("PRET_ROOT"), layout.blockdata_filepath),
        format!("{}/{}", env!("PRET_ROOT"), layout.border_filepath),
    )?;
    assert_eq!(
        map_data.metatiles.len(),
        (layout.width * layout.height) as usize
    );

    let tileset =
        tileset::LayoutTileset::load_from_paths(primary_tileset_dir, secondary_tileset_dir)?;

    const METATILE_DIMENSION: u32 = 16;
    let mut map_image: RgbImage = ImageBuffer::new(
        METATILE_DIMENSION * layout.width,
        METATILE_DIMENSION * layout.height,
    );

    for row in 0..layout.height {
        for col in 0..layout.width {
            let metatile_data = &map_data.metatiles[((row * layout.width) + col) as usize];
            let metatile_left_pixel = col * METATILE_DIMENSION;
            let metatile_top_pixel = row * METATILE_DIMENSION;
            if let Some(metatile_image) = tileset.get_metatile_image(metatile_data.metatile_id) {
                for pixel_row in 0..METATILE_DIMENSION {
                    for pixel_col in 0..METATILE_DIMENSION {
                        let output_row = metatile_top_pixel + pixel_row;
                        let output_col = metatile_left_pixel + pixel_col;
                        map_image.get_pixel_mut(output_col, output_row).0 =
                            metatile_image.get_pixel(pixel_col, pixel_row).0;
                    }
                }
            } else {
                println!("Failed to get metatile image at coordinate: ({col}, {row})");
            }
        }
    }

    map_image.save("/tmp/render.png");

    Ok(())
}

fn tileset_dir(tileset_name: &str) -> String {
    tileset_name.to_case(convert_case::Case::Snake)
}
