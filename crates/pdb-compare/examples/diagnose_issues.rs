/// Diagnostic tool to investigate BuildInfo parsing and TPI parsing issues
use anyhow::Result;
use ms_pdb::Pdb;
use std::fs::File;
use std::path::Path;

fn main() -> Result<()> {
    let pdb_path = std::env::args().nth(1).unwrap_or_else(|| {
        "./crates/ms-pdb/target/debug/build/anyhow-11a718d5af00b923/build_script_build.pdb"
            .to_string()
    });

    println!("═══════════════════════════════════════════════════════════════");
    println!("           BUILDINFO & TPI PARSING DIAGNOSTICS");
    println!("═══════════════════════════════════════════════════════════════\n");
    println!("PDB File: {}\n", pdb_path);

    // Open with ms-pdb to get raw data
    let file = File::open(&pdb_path)?;
    let pdb = Pdb::open_from_file(file)?;
    let tpi_stream = pdb.read_type_stream()?;
    let ipi_stream = pdb.read_ipi_stream()?;

    // Parse with ezpdb
    let parsed_pdb = ezpdb::parse_pdb(Path::new(&pdb_path), None)?;

    println!("🔍 ISSUE 1: BUILDINFO TYPE INDEX COLLISIONS\n");
    println!("Checking if BuildInfo args reference indices that exist in both TPI and IPI...\n");

    let mut collision_count = 0;
    let mut buildinfo_count = 0;

    for (idx, type_ref) in parsed_pdb.ipi_types.iter() {
        let ty = type_ref.borrow();

        if let ezpdb::type_info::Type::BuildInfoType(build_info) = &*ty {
            buildinfo_count += 1;

            if buildinfo_count <= 3 {
                println!("BuildInfo at IPI index 0x{:08x}:", idx);

                // Get the raw BuildInfo from IPI stream
                let type_index = ms_pdb::codeview::types::TypeIndex(*idx);
                if let Ok(record) = ipi_stream.record(type_index) {
                    if let Ok(ms_pdb::codeview::types::TypeData::BuildInfo(raw_build_info)) =
                        record.parse()
                    {
                        println!("  Raw args from IPI stream:");
                        for (i, arg_idx) in raw_build_info.args.iter().enumerate() {
                            let arg_type_idx = arg_idx.get();
                            if arg_type_idx == 0 {
                                continue;
                            }

                            // Check what this index resolves to in IPI
                            let ipi_type_idx = ms_pdb::codeview::types::TypeIndex(arg_type_idx);
                            let ipi_result = if let Ok(rec) = ipi_stream.record(ipi_type_idx) {
                                match rec.parse() {
                                    Ok(ms_pdb::codeview::types::TypeData::StringId(s)) => {
                                        format!("StringId(\"{}\")", s.name)
                                    }
                                    Ok(other) => format!("{:?}", other)
                                        .split('(')
                                        .next()
                                        .unwrap()
                                        .to_string(),
                                    Err(e) => format!("Error: {}", e),
                                }
                            } else {
                                "Not in IPI".to_string()
                            };

                            // Check what this index resolves to in TPI
                            let tpi_type_idx = ms_pdb::codeview::types::TypeIndex(arg_type_idx);
                            let tpi_result = if let Ok(rec) = tpi_stream.record(tpi_type_idx) {
                                match rec.parse() {
                                    Ok(data) => {
                                        let type_str = format!("{:?}", data);
                                        type_str.split('(').next().unwrap().to_string()
                                    }
                                    Err(e) => format!("Error: {}", e),
                                }
                            } else {
                                "Not in TPI".to_string()
                            };

                            let has_collision =
                                tpi_result != "Not in TPI" && ipi_result != "Not in IPI";
                            if has_collision {
                                collision_count += 1;
                            }

                            println!("    [{}] Index 0x{:08x}:", i, arg_type_idx);
                            println!("        IPI: {}", ipi_result);
                            println!("        TPI: {}", tpi_result);
                            if has_collision {
                                println!("        ⚠️  COLLISION! Index exists in both streams");
                            }

                            // Show what ezpdb resolved it to
                            if let Some(item) = build_info.items.get(i) {
                                let item_ty = item.borrow();
                                let item_str = format!("{:?}", item_ty);
                                let variant = item_str.split('(').next().unwrap_or("Unknown");
                                println!("        ezpdb resolved to: {}", variant);
                            }

                            println!();
                        }
                    }
                }
                println!();
            }
        }
    }

    println!("Summary:");
    println!("  Total BuildInfo types: {}", buildinfo_count);
    println!("  Index collisions detected: {}", collision_count);
    println!();

    if collision_count > 0 {
        println!("❌ BUG CONFIRMED: BuildInfo args have index collisions!");
        println!("   The code is checking TPI HashMap first, causing it to resolve");
        println!("   to TPI types instead of IPI StringIds.\n");
    } else {
        println!("✅ No collisions detected in BuildInfo args.\n");
    }

    println!("═══════════════════════════════════════════════════════════════\n");
    println!("🔍 ISSUE 2: TPI PARSING FAILURES\n");
    println!("Checking why TPI isn't parsing at 100%...\n");

    let tpi_begin = tpi_stream.type_index_begin().0;
    let tpi_end = tpi_stream.type_index_end().0;
    let total_tpi = (tpi_end - tpi_begin) as usize;
    let parsed_tpi = parsed_pdb.types.len();
    let missing_tpi = total_tpi - parsed_tpi;

    println!("TPI Stream Info:");
    println!("  Range: 0x{:08x} to 0x{:08x}", tpi_begin, tpi_end);
    println!("  Total types in stream: {}", total_tpi);
    println!("  Parsed by ezpdb: {}", parsed_tpi);
    println!(
        "  Missing: {} ({:.2}%)",
        missing_tpi,
        (missing_tpi as f64 / total_tpi as f64) * 100.0
    );
    println!();

    // Find the missing indices
    println!("Looking for missing type indices...\n");
    let mut missing_indices = Vec::new();

    for idx in tpi_begin..tpi_end {
        if !parsed_pdb.types.contains_key(&idx) {
            missing_indices.push(idx);
        }
    }

    println!("Missing {} type indices:", missing_indices.len());
    for (i, idx) in missing_indices.iter().enumerate().take(10) {
        print!("  0x{:08x}", idx);

        // Try to parse this record directly
        let type_idx = ms_pdb::codeview::types::TypeIndex(*idx);
        match tpi_stream.record(type_idx) {
            Ok(record) => {
                let leaf_kind = record.kind;
                match record.parse() {
                    Ok(data) => {
                        let type_str = format!("{:?}", data);
                        let variant = type_str.split('(').next().unwrap_or("Unknown");
                        if variant == "Unknown" {
                            println!(" - Unknown type, Leaf kind: {:?}", leaf_kind);
                        } else {
                            println!(" - Can parse as: {}", variant);
                        }
                    }
                    Err(e) => {
                        println!(" - Parse error: {} (Leaf: {:?})", e, leaf_kind);
                    }
                }
            }
            Err(e) => {
                println!(" - Record error: {}", e);
            }
        }

        if i >= 9 && missing_indices.len() > 10 {
            println!("  ... and {} more", missing_indices.len() - 10);
            break;
        }
    }
    println!();

    // Analyze types of missing records
    println!("Analyzing what types are missing...\n");
    let mut missing_type_counts = std::collections::HashMap::new();

    for idx in missing_indices.iter() {
        let type_idx = ms_pdb::codeview::types::TypeIndex(*idx);
        if let Ok(record) = tpi_stream.record(type_idx) {
            if let Ok(data) = record.parse() {
                let type_str = format!("{:?}", data);
                let variant = type_str.split('(').next().unwrap_or("Unknown").to_string();
                *missing_type_counts.entry(variant).or_insert(0) += 1;
            }
        }
    }

    if !missing_type_counts.is_empty() {
        println!("Missing type distribution:");
        let mut counts_vec: Vec<_> = missing_type_counts.iter().collect();
        counts_vec.sort_by_key(|(_, count)| std::cmp::Reverse(**count));

        for (type_name, count) in counts_vec.iter() {
            println!("  {}: {}", type_name, count);
        }
        println!();
    }

    // Check for patterns
    if missing_indices.len() == 2 {
        println!("💡 Observation: Only 2 types missing");
        println!("   This suggests specific type records that aren't being handled.\n");
    } else if missing_indices.len() > 10 {
        println!("⚠️  Warning: Many types missing");
        println!("   This suggests a systematic issue with type parsing.\n");
    } else {
        println!("✅ Only a few types missing - likely edge cases or unsupported types.\n");
    }

    println!("═══════════════════════════════════════════════════════════════\n");
    println!("                        RECOMMENDATIONS");
    println!("═══════════════════════════════════════════════════════════════\n");

    if collision_count > 0 {
        println!("1. FIX BUILDINFO PARSING:");
        println!("   In type_info.rs BuildInfoTypeData::try_from(), change:");
        println!("   ");
        println!("   FROM:");
        println!("     if let Some(typ) = output_pdb.types.get(&type_idx.0) {{");
        println!("         Ok(Rc::clone(typ))");
        println!("     }} else if let Some(typ) = output_pdb.ipi_types.get(&type_idx.0) {{");
        println!("   ");
        println!("   TO:");
        println!("     // BuildInfo ONLY references IPI types (StringIds), check IPI first");
        println!("     if let Some(typ) = output_pdb.ipi_types.get(&type_idx.0) {{");
        println!("         Ok(Rc::clone(typ))");
        println!("     }} else if let Some(typ) = output_pdb.types.get(&type_idx.0) {{");
        println!();
    }

    if !missing_type_counts.is_empty() {
        println!("2. INVESTIGATE MISSING TPI TYPES:");
        for (type_name, count) in missing_type_counts.iter() {
            println!("   - Add support for {} ({} instances)", type_name, count);
        }
        println!();
    }

    println!("═══════════════════════════════════════════════════════════════\n");

    Ok(())
}
