use std::path::PathBuf;

fn main() {
    env_logger::init();

    let pdb_path = PathBuf::from("cache_test_pdbs/ntkrnlmp.pdb");
    if !pdb_path.exists() {
        eprintln!("PDB not found: {}", pdb_path.display());
        return;
    }

    println!("Investigating offset bug in {}...\n", pdb_path.display());

    let old_data =
        pdb_compare::old_pdb::parse_pdb(&pdb_path).expect("Failed to parse with old parser");
    let new_data =
        pdb_compare::new_pdb::parse_pdb_new(&pdb_path).expect("Failed to parse with new parser");

    // Look for the specific case mentioned in the output:
    // old idx 14074 vs new idx 48871: Field 'Long': old offset=Some(0), new offset=Some(4)

    println!("=== OLD PARSER: Index 14074 ===");
    if let Some(old_type) = old_data.types.iter().find(|t| t.index == 14074) {
        println!("Type: {:?}", old_type.name);
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
    } else {
        println!("Type 14074 not found in old parser");
    }

    println!("\n=== NEW PARSER: Index 14074 ===");
    if let Some(new_type) = new_data.types.iter().find(|t| t.index == 14074) {
        println!("Type: {:?}", new_type.name);
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
    } else {
        println!("Type 14074 not found in new parser");
    }

    println!("\n=== NEW PARSER: Index 48871 ===");
    if let Some(new_type) = new_data.types.iter().find(|t| t.index == 48871) {
        println!("Type: {:?}", new_type.name);
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
    } else {
        println!("Type 48871 not found in new parser");
    }

    println!("\n=== OLD PARSER: Index 48871 ===");
    if let Some(old_type) = old_data.types.iter().find(|t| t.index == 48871) {
        println!("Type: {:?}", old_type.name);
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
    } else {
        println!("Type 48871 not found in old parser");
    }

    // Look at a union case
    println!("\n\n=== Checking Union Cases ===");

    // Find all types where old and new have same index and name but different offsets
    for old_type in &old_data.types {
        if let Some(new_type) = new_data.types.iter().find(|t| t.index == old_type.index) {
            if old_type.name == new_type.name
                && old_type.fields.len() == new_type.fields.len()
                && old_type.fields.len() > 0
            {
                // Check for offset differences
                for old_field in &old_type.fields {
                    if let Some(new_field) =
                        new_type.fields.iter().find(|f| f.name == old_field.name)
                    {
                        if old_field.offset != new_field.offset {
                            println!("\nOFFSET MISMATCH AT INDEX {}", old_type.index);
                            println!("Type: {:?} ({})", old_type.name, old_type.kind);
                            println!("Size: old={:?}, new={:?}", old_type.size, new_type.size);
                            println!(
                                "Field '{}': old offset={:?}, new offset={:?}",
                                old_field.name, old_field.offset, new_field.offset
                            );

                            // Print all fields for context
                            println!("\nOLD fields:");
                            for f in &old_type.fields {
                                println!("  {} @ {:?}", f.name, f.offset);
                            }
                            println!("\nNEW fields:");
                            for f in &new_type.fields {
                                println!("  {} @ {:?}", f.name, f.offset);
                            }

                            // Only show first 3 cases
                            static mut SHOWN: usize = 0;
                            unsafe {
                                SHOWN += 1;
                                if SHOWN >= 3 {
                                    println!("\n... (stopping after 3 examples)");
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
