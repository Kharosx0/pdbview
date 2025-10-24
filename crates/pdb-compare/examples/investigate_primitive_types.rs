use std::path::PathBuf;

fn main() {
    env_logger::init();

    let pdb_path = PathBuf::from("cache_test_pdbs/ntkrnlmp.pdb");
    if !pdb_path.exists() {
        eprintln!("PDB not found: {}", pdb_path.display());
        return;
    }

    println!("Investigating primitive type differences...\n");

    let old_data =
        pdb_compare::old_pdb::parse_pdb(&pdb_path).expect("Failed to parse with old parser");
    let new_data =
        pdb_compare::new_pdb::parse_pdb_new(&pdb_path).expect("Failed to parse with new parser");

    // Check the specific indices mentioned in the test failure
    let problematic_indices = vec![35, 1571, 117, 1568, 1553, 1652, 1554, 34, 32, 1555];

    for index in problematic_indices {
        println!("=== Type Index {} ===", index);

        let old_type = old_data.types.iter().find(|t| t.index == index);
        let new_type = new_data.types.iter().find(|t| t.index == index);

        match (old_type, new_type) {
            (Some(old), Some(new)) => {
                println!("OLD: kind={}, name={:?}", old.kind, old.name);
                println!("NEW: kind={}, name={:?}", new.kind, new.name);

                if old.kind != new.kind {
                    println!("⚠️  KIND MISMATCH!");
                    println!("  Old: {}", old.kind);
                    println!("  New: {}", new.kind);
                }

                // Check if these are primitive types
                if old.kind.starts_with("Primitive(") && new.kind.starts_with("Primitive(") {
                    println!("  Both are primitive types but with different representations");
                    println!(
                        "  This might be a difference in how primitive type indices are mapped"
                    );
                }
            }
            (Some(old), None) => {
                println!("OLD: kind={}, name={:?}", old.kind, old.name);
                println!("NEW: NOT FOUND");
            }
            (None, Some(new)) => {
                println!("OLD: NOT FOUND");
                println!("NEW: kind={}, name={:?}", new.kind, new.name);
            }
            (None, None) => {
                println!("NOT FOUND in either parser");
            }
        }
        println!();
    }

    // Summary: Check all primitive type mismatches
    println!("\n=== COMPLETE PRIMITIVE TYPE COMPARISON ===\n");

    let mut mismatch_count = 0;
    let mut total_primitives = 0;

    for old_type in &old_data.types {
        if !old_type.kind.starts_with("Primitive(") {
            continue;
        }

        total_primitives += 1;

        if let Some(new_type) = new_data.types.iter().find(|t| t.index == old_type.index) {
            if old_type.kind != new_type.kind {
                mismatch_count += 1;
                if mismatch_count <= 20 {
                    println!(
                        "Index {}: Old='{}', New='{}'",
                        old_type.index, old_type.kind, new_type.kind
                    );
                }
            }
        }
    }

    if mismatch_count > 20 {
        println!("... and {} more mismatches", mismatch_count - 20);
    }

    println!("\n=== SUMMARY ===");
    println!("Total primitive types: {}", total_primitives);
    println!("Primitive type mismatches: {}", mismatch_count);

    if mismatch_count > 0 {
        println!("\n⚠️  ISSUE FOUND:");
        println!("Old and new parsers are interpreting primitive type indices differently.");
        println!("This could be due to:");
        println!("  1. Different primitive type mapping tables");
        println!("  2. Different interpretation of CodeView primitive type indices");
        println!("  3. A bug in one of the parsers");
        println!("\nHowever, this might NOT affect correctness if:");
        println!("  - These primitive indices are for basic types (builtin compiler types)");
        println!("  - They're not actually used in struct field definitions");
        println!("  - The actual data layout is still correct");
    } else {
        println!("\n✅ All primitive types match perfectly!");
    }
}
