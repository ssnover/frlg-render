use clap::Parser;
use convert_case::Casing;
use frlg_render::{map, tileset, METATILE_DIMENSION};
use image::{GenericImage, ImageBuffer, RgbImage};
use serde::Deserialize;
use std::fs::File;
use std::io;
use std::path::PathBuf;

const PRET_ROOT: &str = env!("PRET_ROOT");

#[derive(Parser)]
struct Args {
    #[arg(long)]
    /// The layout to render, e.g. LAYOUT_POWER_PLANT
    layout: Option<String>,

    #[arg(short, long)]
    /// The output path for the rendered png image, default is /tmp/render.png
    output: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
struct LayoutsTable {
    //layouts_table_label: String,
    layouts: Vec<Layout>,
}

#[derive(Debug, Clone, Deserialize)]
struct Layout {
    id: String,
    width: u32,
    height: u32,
    primary_tileset: String,
    secondary_tileset: String,
    border_filepath: String,
    blockdata_filepath: String,
}

const LAYOUTS_FILE: &str = concat!(env!("PRET_ROOT"), "/data/layouts/layouts.json");

fn main() -> io::Result<()> {
    env_logger::init();

    let args = Args::parse();
    let map = args.layout.unwrap_or("LAYOUT_POWER_PLANT".to_string());
    let output_file = args.output.unwrap_or(PathBuf::from("/tmp/render.png"));

    let layouts = {
        let file = File::open(LAYOUTS_FILE)?;
        let layouts_table: LayoutsTable = serde_json::from_reader(file).unwrap();
        layouts_table.layouts
    };

    let Some(layout) = layouts
        .into_iter()
        .find(|layout| layout.id.as_str() == map.as_str())
    else {
        log::error!("No layout matching name {map} found");
        std::process::exit(1);
    };
    log::info!("{:#?}", layout);
    let primary = layout
        .primary_tileset
        .strip_prefix("gTileset_")
        .unwrap()
        .to_ascii_lowercase();
    let secondary = tileset_dir(layout.secondary_tileset.strip_prefix("gTileset_").unwrap());
    let primary_tileset_dir = format!("{PRET_ROOT}/data/tilesets/primary/{primary}");
    let secondary_tileset_dir = format!("{PRET_ROOT}/data/tilesets/secondary/{secondary}");

    let map_layout = map::Layout::load(
        layout.width,
        layout.height,
        format!("{}/{}", env!("PRET_ROOT"), layout.blockdata_filepath),
        format!("{}/{}", env!("PRET_ROOT"), layout.border_filepath),
    )?;

    let tileset =
        tileset::LayoutTileset::load_from_paths(primary_tileset_dir, secondary_tileset_dir)?;

    let mut map_image: RgbImage = ImageBuffer::new(
        METATILE_DIMENSION * layout.width,
        METATILE_DIMENSION * layout.height,
    );

    for row in 0..layout.height {
        for col in 0..layout.width {
            let metatile_data = map_layout.get_metatile(row, col).unwrap();
            let metatile_left_pixel = col * METATILE_DIMENSION;
            let metatile_top_pixel = row * METATILE_DIMENSION;
            log::debug!("Metatile id: {}", metatile_data.metatile_id);
            if let Some(metatile_image) = tileset.get_metatile_image(metatile_data.metatile_id) {
                map_image
                    .sub_image(
                        metatile_left_pixel,
                        metatile_top_pixel,
                        METATILE_DIMENSION,
                        METATILE_DIMENSION,
                    )
                    .copy_from(&metatile_image, 0, 0)
                    .expect("Should be able to copy into subimage");
            } else {
                log::error!("Failed to get metatile image at coordinate: ({col}, {row})");
            }
        }
    }

    map_image.save(output_file).unwrap();

    Ok(())
}

fn tileset_dir(tileset_name: &str) -> String {
    tileset_name.to_case(convert_case::Case::Snake)
}
