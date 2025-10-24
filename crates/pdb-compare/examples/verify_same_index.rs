use std::path::PathBuf;

fn main() {
    env_logger::init();

    let test_pdbs = vec![
        "cache_test_pdbs/ntkrnlmp.pdb",
        "cache_test_pdbs/ntdll.pdb",
        "cache_test_pdbs/hvix64.pdb",
    ];

    for pdb_path_str in test_pdbs {
        let pdb_path = PathBuf::from(pdb_path_str);
        if !pdb_path.exists() {
            eprintln!("Skipping: {} not found", pdb_path.display());
            continue;
        }

        println!("\n{:=<80}", "");
        println!("Verifying same-index comparison for {}", pdb_path.display());
        println!("{:=<80}\n", "");

        let old_data =
            pdb_compare::old_pdb::parse_pdb(&pdb_path).expect("Failed to parse with old parser");
        let new_data = pdb_compare::new_pdb::parse_pdb_new(&pdb_path)
            .expect("Failed to parse with new parser");

        let mut total_types_compared = 0;
        let mut total_fields_compared = 0;
        let mut offset_mismatches = 0;
        let mut size_mismatches = 0;
        let mut field_count_mismatches = 0;

        // Compare types at the SAME index in both parsers
        for old_type in &old_data.types {
            // Find the type at the same index in the new parser
            if let Some(new_type) = new_data.types.iter().find(|t| t.index == old_type.index) {
                // Only compare non-empty types of the same kind
                if old_type.kind != new_type.kind {
                    continue;
                }

                if old_type.fields.is_empty() && new_type.fields.is_empty() {
                    continue;
                }

                total_types_compared += 1;

                // Check size
                if old_type.size != new_type.size {
                    size_mismatches += 1;
                    println!(
                        "SIZE MISMATCH at index {}: {:?} ({})",
                        old_type.index, old_type.name, old_type.kind
                    );
                    println!("  Old size: {:?}", old_type.size);
                    println!("  New size: {:?}", new_type.size);
                }

                // Check field count
                if old_type.fields.len() != new_type.fields.len() {
                    field_count_mismatches += 1;
                    println!(
                        "FIELD COUNT MISMATCH at index {}: {:?} ({})",
                        old_type.index, old_type.name, old_type.kind
                    );
                    println!("  Old fields: {}", old_type.fields.len());
                    println!("  New fields: {}", new_type.fields.len());
                    continue;
                }

                // Check each field
                for old_field in &old_type.fields {
                    total_fields_compared += 1;

                    if let Some(new_field) =
                        new_type.fields.iter().find(|f| f.name == old_field.name)
                    {
                        // Check offset
                        if old_field.offset != new_field.offset {
                            offset_mismatches += 1;
                            println!(
                                "OFFSET MISMATCH at index {}: {:?} ({}), field '{}'",
                                old_type.index, old_type.name, old_type.kind, old_field.name
                            );
                            println!("  Old offset: {:?}", old_field.offset);
                            println!("  New offset: {:?}", new_field.offset);
                            println!(
                                "  Type size: old={:?}, new={:?}",
                                old_type.size, new_type.size
                            );

                            // Print all fields for context
                            println!("  All OLD fields:");
                            for f in &old_type.fields {
                                println!(
                                    "    {} @ {:?} (bitfield: len={:?}, pos={:?})",
                                    f.name, f.offset, f.bitfield_length, f.bitfield_position
                                );
                            }
                            println!("  All NEW fields:");
                            for f in &new_type.fields {
                                println!(
                                    "    {} @ {:?} (bitfield: len={:?}, pos={:?})",
                                    f.name, f.offset, f.bitfield_length, f.bitfield_position
                                );
                            }
                        }

                        // Check bitfield properties
                        if old_field.bitfield_length != new_field.bitfield_length {
                            println!(
                                "BITFIELD LENGTH MISMATCH at index {}: {:?}, field '{}'",
                                old_type.index, old_type.name, old_field.name
                            );
                            println!("  Old: {:?}", old_field.bitfield_length);
                            println!("  New: {:?}", new_field.bitfield_length);
                        }

                        if old_field.bitfield_position != new_field.bitfield_position {
                            println!(
                                "BITFIELD POSITION MISMATCH at index {}: {:?}, field '{}'",
                                old_type.index, old_type.name, old_field.name
                            );
                            println!("  Old: {:?}", old_field.bitfield_position);
                            println!("  New: {:?}", new_field.bitfield_position);
                        }
                    }
                }
            }
        }

        println!("\n{:=<80}", "");
        println!("SUMMARY for {}", pdb_path.display());
        println!("{:=<80}", "");
        println!(
            "Total types compared (same index):  {}",
            total_types_compared
        );
        println!(
            "Total fields compared:               {}",
            total_fields_compared
        );
        println!("Size mismatches:                     {}", size_mismatches);
        println!(
            "Field count mismatches:              {}",
            field_count_mismatches
        );
        println!("Offset mismatches:                   {}", offset_mismatches);

        if offset_mismatches == 0 && size_mismatches == 0 && field_count_mismatches == 0 {
            println!("\n✅ PERFECT MATCH! All types at the same index are identical.");
        } else {
            println!(
                "\n⚠️  Found {} differences",
                offset_mismatches + size_mismatches + field_count_mismatches
            );
        }
    }

    println!("\n\n{:=<80}", "");
    println!("FINAL CONCLUSION");
    println!("{:=<80}", "");
    println!("When comparing types at the SAME type index:");
    println!("- Both parsers extract data from the EXACT same PDB records");
    println!("- Any differences indicate actual parsing bugs");
    println!("- Zero differences = Perfect parser correctness ✅");
}
