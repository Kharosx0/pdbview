use std::path::PathBuf;

fn main() {
    env_logger::init();

    let pdb_path = PathBuf::from("cache_test_pdbs/ntkrnlmp.pdb");

    println!("=== OLD PARSER ===");
    let old_data = pdb_compare::old_pdb::parse_pdb(&pdb_path).unwrap();

    for old_type in &old_data.types {
        if let Some(name) = &old_type.name {
            if name == "_RTLP_CPU_FEATURE_ENTRY" {
                println!("Type: {} (index {})", name, old_type.index);
                println!("Kind: {}", old_type.kind);
                println!("Size: {:?}", old_type.size);
                println!("Fields: {}", old_type.fields.len());
                for field in &old_type.fields {
                    println!(
                        "  - {} @ offset {:?}, type_kind={:?}, bitfield: len={:?}, pos={:?}",
                        field.name,
                        field.offset,
                        field.type_kind,
                        field.bitfield_length,
                        field.bitfield_position
                    );
                }
                println!();
            }
        }
    }

    println!("=== NEW PARSER ===");
    let new_data = pdb_compare::new_pdb::parse_pdb_new(&pdb_path).unwrap();

    for new_type in &new_data.types {
        if let Some(name) = &new_type.name {
            if name == "_RTLP_CPU_FEATURE_ENTRY" {
                println!("Type: {} (index {})", name, new_type.index);
                println!("Kind: {}", new_type.kind);
                println!("Size: {:?}", new_type.size);
                println!("Fields: {}", new_type.fields.len());
                for field in &new_type.fields {
                    println!(
                        "  - {} @ offset {:?}, type_kind={:?}, bitfield: len={:?}, pos={:?}",
                        field.name,
                        field.offset,
                        field.type_kind,
                        field.bitfield_length,
                        field.bitfield_position
                    );
                }
                println!();
            }
        }
    }
}
