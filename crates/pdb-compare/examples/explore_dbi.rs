use ms_pdb::ReadAt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .expect("Usage: explore_dbi <pdb_file>");
    let path = std::path::Path::new(&path);

    // Let me check what ms-pdb exposes
    let pdb = ms_pdb::Pdb::open(path)?;

    println!("DBI Substreams:");
    let substreams = pdb.dbi_substreams();

    println!("  section_map_bytes: {:?}", substreams.section_map_bytes);
    println!(
        "  section_contributions_bytes: {:?}",
        substreams.section_contributions_bytes
    );
    println!("  source_info: {:?}", substreams.source_info);

    // Check if there are other bytes/ranges we can explore
    println!("\nLet's see what section contributions contain...");

    if !substreams.section_contributions_bytes.is_empty() {
        let reader = pdb.get_stream_reader(ms_pdb::Stream::DBI.into())?;
        let len = substreams.section_contributions_bytes.len();
        let mut data = vec![0u8; len];
        reader.read_exact_at(
            &mut data,
            substreams.section_contributions_bytes.start as u64,
        )?;

        println!(
            "Section contributions data (first 100 bytes): {:02x?}",
            &data[..data.len().min(100)]
        );
    }

    Ok(())
}
