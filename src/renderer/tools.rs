use std::{error::Error, fs::File, io::Read, path::Path};

pub fn read_shader(path: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    let spv = File::open(path)?;
    Ok(spv.bytes().filter_map(|b| b.ok()).collect::<Vec<u8>>())
}
