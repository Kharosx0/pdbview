use std::collections::HashMap;
use std::path::PathBuf;

fn main() {
    env_logger::init();

    let pdb_path = PathBuf::from("cache_test_pdbs/ntkrnlmp.pdb");
    if !pdb_path.exists() {
        eprintln!("PDB not found: {}", pdb_path.display());
        return;
    }

    println!(
        "Analyzing offset differences in {}...\n",
        pdb_path.display()
    );

    let old_data =
        pdb_compare::old_pdb::parse_pdb(&pdb_path).expect("Failed to parse with old parser");
    let new_data =
        pdb_compare::new_pdb::parse_pdb_new(&pdb_path).expect("Failed to parse with new parser");

    // Build maps of type name -> all versions
    let mut old_versions: HashMap<String, Vec<&pdb_compare::old_pdb::TypeInfo>> = HashMap::new();
    for old_type in &old_data.types {
        if let Some(name) = &old_type.name {
            if old_type.kind == "Class" || old_type.kind == "Union" {
                old_versions
                    .entry(name.clone())
                    .or_insert_with(Vec::new)
                    .push(old_type);
            }
        }
    }

    let mut new_versions: HashMap<String, Vec<&pdb_compare::new_pdb::TypeInfo>> = HashMap::new();
    for new_type in &new_data.types {
        if let Some(name) = &new_type.name {
            if new_type.kind == "Class" || new_type.kind == "Union" {
                new_versions
                    .entry(name.clone())
                    .or_insert_with(Vec::new)
                    .push(new_type);
            }
        }
    }

    let mut types_with_multiple_versions = 0;
    let mut types_with_offset_mismatches = 0;
    let mut types_with_size_mismatches = 0;

    for (type_name, old_vers) in &old_versions {
        if old_vers.is_empty() {
            continue;
        }

        let new_vers = match new_versions.get(type_name) {
            Some(v) => v,
            None => continue,
        };

        // Check if there are multiple complete versions
        let old_complete: Vec<_> = old_vers
            .iter()
            .filter(|t| t.size.unwrap_or(0) > 0)
            .collect();
        let new_complete: Vec<_> = new_vers
            .iter()
            .filter(|t| t.size.unwrap_or(0) > 0)
            .collect();

        if old_complete.len() > 1 || new_complete.len() > 1 {
            types_with_multiple_versions += 1;

            println!("=== {} (multiple versions) ===", type_name);
            println!("Old parser found {} complete versions:", old_complete.len());
            for old_type in &old_complete {
                println!(
                    "  - Index {}: size={:?}, fields={}",
                    old_type.index,
                    old_type.size,
                    old_type.fields.len()
                );
            }
            println!("New parser found {} complete versions:", new_complete.len());
            for new_type in &new_complete {
                println!(
                    "  - Index {}: size={:?}, fields={}",
                    new_type.index,
                    new_type.size,
                    new_type.fields.len()
                );
            }

            // Try to match versions by (size, field_count)
            for old_type in &old_complete {
                for new_type in &new_complete {
                    if old_type.size == new_type.size
                        && old_type.fields.len() == new_type.fields.len()
                    {
                        // Check for offset differences
                        let mut has_offset_mismatch = false;
                        let mut has_size_mismatch = false;

                        for old_field in &old_type.fields {
                            if let Some(new_field) =
                                new_type.fields.iter().find(|f| f.name == old_field.name)
                            {
                                if old_field.offset != new_field.offset {
                                    if !has_offset_mismatch {
                                        println!("\n  OFFSET MISMATCH in matching versions (old idx {} vs new idx {}):",
                                            old_type.index, new_type.index);
                                        has_offset_mismatch = true;
                                    }
                                    println!(
                                        "    Field '{}': old offset={:?}, new offset={:?}",
                                        old_field.name, old_field.offset, new_field.offset
                                    );
                                }
                            }
                        }

                        if has_offset_mismatch {
                            types_with_offset_mismatches += 1;
                        }
                        if old_type.size != new_type.size {
                            has_size_mismatch = true;
                            types_with_size_mismatches += 1;
                        }
                    }
                }
            }
            println!();
        }
    }

    println!("\n=== SUMMARY ===");
    println!(
        "Types with multiple complete versions: {}",
        types_with_multiple_versions
    );
    println!(
        "Types with offset mismatches in matching versions: {}",
        types_with_offset_mismatches
    );
    println!(
        "Types with size mismatches in matching versions: {}",
        types_with_size_mismatches
    );

    if types_with_offset_mismatches == 0 && types_with_size_mismatches == 0 {
        println!("\n✓ No offset or size mismatches found when comparing matching versions!");
        println!("  All differences are due to comparing different legitimate type versions.");
    }
}
