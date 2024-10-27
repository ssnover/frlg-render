use std::io;
use std::path::Path;

#[derive(Debug)]
pub struct Palette {
    inner: [(u8, u8, u8); 16],
}

impl Palette {
    pub fn get(&self, entry: usize) -> &(u8, u8, u8) {
        &self.inner[entry]
    }
}

fn is_pal_file(entry: &Path) -> bool {
    entry
        .extension()
        .unwrap_or_default()
        .to_os_string()
        .to_str()
        .unwrap_or_default()
        == "pal"
}

fn palette_number(path: &Path) -> u32 {
    path.file_stem().unwrap().to_str().unwrap().parse().unwrap()
}

pub fn parse_all_palettes(path: impl AsRef<Path>) -> io::Result<Vec<Palette>> {
    let mut palettes = std::fs::read_dir(path)?
        .into_iter()
        .filter_map(|entry| {
            if let Ok(entry) = entry {
                if is_pal_file(&entry.path()) {
                    Some(entry.path())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .map(|palette_path| {
            parse_palette(&palette_path).map(|palette| (palette, palette_number(&palette_path)))
        })
        .collect::<io::Result<Vec<(Palette, u32)>>>()?;
    palettes.sort_by(|(_, a), (_, b)| a.cmp(b));
    Ok(palettes.into_iter().map(|(palette, _)| palette).collect())
}

fn parse_palette(path: impl AsRef<Path>) -> io::Result<Palette> {
    // Parses a JASC-PAL file
    let palette_contents = std::fs::read_to_string(&path)?;
    let mut lines = palette_contents.lines();
    let mut palette_data = [(0, 0, 0); 16];
    if let (Some("JASC-PAL"), Some("0100"), Some("16")) = (lines.next(), lines.next(), lines.next())
    {
        log::debug!("Loading palette {}", path.as_ref().display());
        for palette_id in 0..16 {
            let palette_values = lines
                .next()
                .unwrap()
                .split_ascii_whitespace()
                .map(|value| value.parse::<u8>().unwrap())
                .collect::<Vec<_>>();
            assert_eq!(palette_values.len(), 3);
            log::debug!("Entry {palette_id}: {palette_values:?}");
            palette_data[palette_id] = (palette_values[0], palette_values[1], palette_values[2]);
        }
    }

    Ok(Palette {
        inner: palette_data,
    })
}
