use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

use icns::{IconFamily, IconType, Image};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let input_dir = args
        .next()
        .map(PathBuf::from)
        .ok_or("usage: make_icns <icon-dir> <output.icns>")?;
    let output_path = args
        .next()
        .map(PathBuf::from)
        .ok_or("usage: make_icns <icon-dir> <output.icns>")?;

    let mut family = IconFamily::new();

    for (file_name, icon_type) in icon_sources() {
        let image = read_png(&input_dir.join(file_name))?;
        family.add_icon_with_type(&image, icon_type)?;
    }

    let writer = BufWriter::new(File::create(output_path)?);
    family.write(writer)?;

    Ok(())
}

fn read_png(path: &Path) -> Result<Image, Box<dyn std::error::Error>> {
    let reader = BufReader::new(File::open(path)?);
    Ok(Image::read_png(reader)?)
}

fn icon_sources() -> [(&'static str, IconType); 7] {
    [
        ("icon_16x16.png", IconType::RGB24_16x16),
        ("icon_32x32.png", IconType::RGB24_32x32),
        ("icon_48x48.png", IconType::RGB24_48x48),
        ("icon_128x128.png", IconType::RGB24_128x128),
        ("icon_256x256.png", IconType::RGBA32_256x256),
        ("icon_512x512.png", IconType::RGBA32_512x512),
        ("icon_1024x1024.png", IconType::RGBA32_512x512_2x),
    ]
}
