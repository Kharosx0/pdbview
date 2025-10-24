use std::collections::HashMap;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdb_path = std::env::args()
        .nth(1)
        .expect("Usage: analyze_collisions_simple <pdb_path>");

    println!("Parsing PDB: {}\n", pdb_path);

    // Parse PDB using ezpdb
    let parsed_pdb = ezpdb::parse_pdb(Path::new(&pdb_path), None)?;

    // Get TPI and IPI type names
    let mut tpi_type_names = HashMap::new();
    for (idx, type_ref) in parsed_pdb.types.iter() {
        let borrowed = type_ref.borrow();
        let type_name = format!("{:?}", borrowed)
            .split('(')
            .next()
            .unwrap_or("Unknown")
            .to_string();
        tpi_type_names.insert(*idx, type_name);
    }

    let mut ipi_type_names = HashMap::new();
    for (idx, type_ref) in parsed_pdb.ipi_types.iter() {
        let borrowed = type_ref.borrow();
        let type_name = format!("{:?}", borrowed)
            .split('(')
            .next()
            .unwrap_or("Unknown")
            .to_string();
        ipi_type_names.insert(*idx, type_name);
    }

    println!("TPI types loaded: {}", parsed_pdb.types.len());
    println!("IPI types loaded: {}", parsed_pdb.ipi_types.len());
    println!();

    // Analyze collisions
    let mut collision_count = 0;
    let mut same_type_count = 0;
    let mut different_type_count = 0;
    let mut ipi_only_type_count = 0;

    // IPI-only types according to LLVM documentation
    let ipi_only_types = [
        "FuncId",
        "MFuncId",
        "BuildInfoType",
        "SubStrList",
        "StringId",
        "UdtSrcLineType",
        "UdtModSrcLineType",
    ];

    println!("Analyzing index collisions...\n");

    for (idx, _) in parsed_pdb.types.iter() {
        if parsed_pdb.ipi_types.contains_key(idx) {
            collision_count += 1;

            let tpi_type_name = tpi_type_names.get(idx).unwrap();
            let ipi_type_name = ipi_type_names.get(idx).unwrap();

            // Check if IPI side is an IPI-only type
            let is_ipi_only = ipi_only_types.iter().any(|t| ipi_type_name.contains(t));
            if is_ipi_only {
                ipi_only_type_count += 1;
            }

            if tpi_type_name == ipi_type_name {
                same_type_count += 1;
            } else {
                different_type_count += 1;

                // Print first 20 examples
                if different_type_count <= 20 {
                    println!(
                        "  Index 0x{:04x}: TPI={:20} IPI={:20} [IPI-only: {}]",
                        idx, tpi_type_name, ipi_type_name, is_ipi_only
                    );
                }
            }
        }
    }

    println!();
    println!("=== COLLISION ANALYSIS ===");
    println!("Total indices with collisions: {}", collision_count);
    println!("  Same type variant:     {}", same_type_count);
    println!("  Different type variant: {}", different_type_count);
    println!("  IPI side is IPI-only type: {}", ipi_only_type_count);
    println!();

    if different_type_count > 0 {
        println!(
            "⚠️  CRITICAL: {} collisions have DIFFERENT type variants!",
            different_type_count
        );
        println!("    This proves TPI-first lookup is WRONG when indices collide.");
        println!("    We must determine which stream based on record type or context.");
    } else if same_type_count == collision_count {
        println!("✓ All collisions have the SAME type variant.");
        println!("  This means TPI and IPI contain duplicate data at same indices.");
    }

    if ipi_only_type_count > 0 {
        println!();
        println!(
            "⚠️  {} collisions have IPI-only types (FuncId, MFuncId, etc.) on IPI side",
            ipi_only_type_count
        );
        println!("    These types should ONLY exist in IPI according to LLVM docs.");
    }

    Ok(())
}
