use ms_pdb::{Pdb, ReadAt};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .expect("Usage: check_sections <pdb_file>");
    let path = std::path::Path::new(&path);

    let pdb = Pdb::open(path)?;

    // Read the section map
    let section_map_range = pdb.dbi_substreams().section_map_bytes.clone();
    let reader = pdb.get_stream_reader(ms_pdb::Stream::DBI.into())?;
    let len = section_map_range.len();
    let mut section_map_data = vec![0u8; len];
    reader.read_exact_at(&mut section_map_data, section_map_range.start as u64)?;

    let section_map = ms_pdb::dbi::section_map::SectionMap::parse(&section_map_data)?;

    println!("Section Map Entries:");
    println!("===================");
    for (i, entry) in section_map.entries.iter().enumerate() {
        println!(
            "Section {}: offset={:#x} ({}) length={:#x} ({})",
            i + 1, // Sections are 1-indexed in PDB
            entry.offset.get(),
            entry.offset.get(),
            entry.section_length.get(),
            entry.section_length.get()
        );
    }

    Ok(())
}
