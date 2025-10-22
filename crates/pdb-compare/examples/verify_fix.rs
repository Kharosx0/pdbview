/// Comprehensive verification that TPI and IPI types are being parsed correctly
/// This ensures the fix didn't break anything and both streams are properly separated.
use anyhow::Result;
use ms_pdb::Pdb;
use std::fs::File;
use std::path::Path;

fn main() -> Result<()> {
    let pdb_path = std::env::args()
        .nth(1)
        .expect("Usage: verify_fix <pdb_path>");

    println!("═══════════════════════════════════════════════════════════════");
    println!("    COMPREHENSIVE TPI/IPI PARSING VERIFICATION");
    println!("═══════════════════════════════════════════════════════════════\n");
    println!("PDB File: {}\n", pdb_path);

    // Parse with ezpdb
    let parsed_pdb = ezpdb::parse_pdb(Path::new(&pdb_path), None)?;

    // Also open with ms-pdb to get raw stream info
    let file = File::open(&pdb_path)?;
    let pdb = Pdb::open_from_file(file)?;
    let type_stream = pdb.read_type_stream()?;
    let ipi_stream = pdb.read_ipi_stream()?;

    println!("📊 RAW STREAM INFO:");
    println!(
        "   TPI: 0x{:08x} to 0x{:08x} ({} types in stream)",
        type_stream.type_index_begin().0,
        type_stream.type_index_end().0,
        type_stream.type_index_end().0 - type_stream.type_index_begin().0
    );
    println!(
        "   IPI: 0x{:08x} to 0x{:08x} ({} types in stream)",
        ipi_stream.type_index_begin().0,
        ipi_stream.type_index_end().0,
        ipi_stream.type_index_end().0 - ipi_stream.type_index_begin().0
    );

    println!("\n📊 EZPDB PARSED COUNTS:");
    println!("   TPI HashMap size: {} types", parsed_pdb.types.len());
    println!("   IPI HashMap size: {} types", parsed_pdb.ipi_types.len());

    // Verify 1: Check that we parsed a reasonable number of types
    let tpi_stream_count =
        (type_stream.type_index_end().0 - type_stream.type_index_begin().0) as usize;
    let ipi_stream_count =
        (ipi_stream.type_index_end().0 - ipi_stream.type_index_begin().0) as usize;

    println!("\n✓ VERIFICATION 1: Type Count Sanity Check");
    let tpi_ratio = (parsed_pdb.types.len() as f64) / (tpi_stream_count as f64) * 100.0;
    let ipi_ratio = (parsed_pdb.ipi_types.len() as f64) / (ipi_stream_count as f64) * 100.0;

    println!(
        "   TPI: Parsed {} / {} = {:.1}%",
        parsed_pdb.types.len(),
        tpi_stream_count,
        tpi_ratio
    );
    println!(
        "   IPI: Parsed {} / {} = {:.1}%",
        parsed_pdb.ipi_types.len(),
        ipi_stream_count,
        ipi_ratio
    );

    if tpi_ratio < 80.0 {
        println!("   ⚠️  WARNING: TPI parse rate is low (< 80%)");
    } else {
        println!("   ✅ TPI parse rate is good");
    }

    if ipi_ratio < 80.0 {
        println!("   ⚠️  WARNING: IPI parse rate is low (< 80%)");
    } else {
        println!("   ✅ IPI parse rate is good");
    }

    // Verify 2: Check for IPI types in TPI HashMap (the bug we fixed)
    println!("\n✓ VERIFICATION 2: No IPI Types in TPI HashMap");
    let mut ipi_types_in_tpi = 0;
    let mut checked = 0;

    for (type_idx, type_ref) in parsed_pdb.types.iter().take(1000) {
        checked += 1;
        let ty = type_ref.borrow();
        let type_name = format!("{:?}", ty);

        let is_ipi = type_name.starts_with("FuncId(")
            || type_name.starts_with("MFuncId(")
            || type_name.starts_with("StringId(")
            || type_name.starts_with("SubStrList(")
            || type_name.starts_with("BuildInfoType(")
            || type_name.starts_with("UdtSrcLineType(");

        if is_ipi {
            ipi_types_in_tpi += 1;
            if ipi_types_in_tpi <= 5 {
                println!(
                    "   ⚠️  Found IPI type at TPI index 0x{:08x}: {}",
                    type_idx,
                    &type_name[..type_name.len().min(60)]
                );
            }
        }
    }

    if ipi_types_in_tpi > 0 {
        println!(
            "   ❌ FAIL: Found {} IPI types in TPI HashMap (checked {} entries)",
            ipi_types_in_tpi, checked
        );
    } else {
        println!(
            "   ✅ PASS: No IPI types found in TPI HashMap (checked {} entries)",
            checked
        );
    }

    // Verify 3: Check for actual TPI types in TPI HashMap
    println!("\n✓ VERIFICATION 3: TPI HashMap Contains Actual Data Types");
    let mut tpi_type_counts = std::collections::HashMap::new();

    for type_ref in parsed_pdb.types.values().take(1000) {
        let ty = type_ref.borrow();
        let type_name = format!("{:?}", ty);

        // Extract the enum variant name
        let variant = type_name.split('(').next().unwrap_or("Unknown");
        *tpi_type_counts.entry(variant.to_string()).or_insert(0) += 1;
    }

    println!("   TPI type distribution (first 1000 types):");
    let mut types_vec: Vec<_> = tpi_type_counts.iter().collect();
    types_vec.sort_by_key(|(_, count)| std::cmp::Reverse(**count));

    for (type_name, count) in types_vec.iter().take(10) {
        println!("      {}: {}", type_name, count);
    }

    // Verify 4: Check for actual IPI types in IPI HashMap
    println!("\n✓ VERIFICATION 4: IPI HashMap Contains Metadata Types");
    let mut ipi_type_counts = std::collections::HashMap::new();

    for type_ref in parsed_pdb.ipi_types.values().take(1000) {
        let ty = type_ref.borrow();
        let type_name = format!("{:?}", ty);

        let variant = type_name.split('(').next().unwrap_or("Unknown");
        *ipi_type_counts.entry(variant.to_string()).or_insert(0) += 1;
    }

    println!("   IPI type distribution (first 1000 types):");
    let mut ipi_types_vec: Vec<_> = ipi_type_counts.iter().collect();
    ipi_types_vec.sort_by_key(|(_, count)| std::cmp::Reverse(**count));

    for (type_name, count) in ipi_types_vec.iter().take(10) {
        println!("      {}: {}", type_name, count);
    }

    // Check that we have IPI-specific types
    let has_funcid = ipi_type_counts.contains_key("FuncId");
    let has_udtsrcline = ipi_type_counts.contains_key("UdtSrcLineType");

    if has_funcid || has_udtsrcline {
        println!("   ✅ PASS: IPI HashMap contains expected metadata types");
    } else {
        println!("   ⚠️  WARNING: IPI HashMap doesn't have typical IPI types (FuncId, UdtSrcLine)");
    }

    // Verify 5: Check data symbols resolve to TPI types
    println!("\n✓ VERIFICATION 5: Data Symbols Resolve to TPI Types");
    let mut data_symbol_types = std::collections::HashMap::new();
    let mut ipi_in_data_symbols = 0;

    for data_symbol in parsed_pdb.global_data.iter().take(500) {
        let ty = data_symbol.ty.borrow();
        let type_name = format!("{:?}", ty);

        let variant = type_name.split('(').next().unwrap_or("Unknown");
        *data_symbol_types.entry(variant.to_string()).or_insert(0) += 1;

        // Check if it's an IPI type
        let is_ipi = variant == "FuncId"
            || variant == "MFuncId"
            || variant == "StringId"
            || variant == "SubStrList"
            || variant == "BuildInfoType"
            || variant == "UdtSrcLineType";

        if is_ipi {
            ipi_in_data_symbols += 1;
            if ipi_in_data_symbols <= 3 {
                println!(
                    "   ⚠️  Data symbol '{}' has IPI type: {}",
                    data_symbol.name, variant
                );
            }
        }
    }

    println!("   Data symbol type distribution (first 500 symbols):");
    let mut data_types_vec: Vec<_> = data_symbol_types.iter().collect();
    data_types_vec.sort_by_key(|(_, count)| std::cmp::Reverse(**count));

    for (type_name, count) in data_types_vec.iter().take(10) {
        println!("      {}: {}", type_name, count);
    }

    if ipi_in_data_symbols > 0 {
        println!(
            "   ❌ FAIL: Found {} data symbols with IPI types",
            ipi_in_data_symbols
        );
    } else {
        println!("   ✅ PASS: All data symbols have TPI types (no IPI types)");
    }

    // Verify 6: Check that procedures are parsed
    println!("\n✓ VERIFICATION 6: Procedures Parsed Successfully");
    println!("   Total procedures: {}", parsed_pdb.procedures.len());
    if parsed_pdb.procedures.is_empty() {
        println!("   ⚠️  WARNING: No procedures found");
    } else {
        println!("   ✅ PASS: Procedures were parsed");
    }

    // Verify 7: Check build info (IPI type)
    println!("\n✓ VERIFICATION 7: Build Info (IPI Type) Parsed");
    if let Some(ref build_info) = parsed_pdb.assembly_info.build_info {
        println!(
            "   ✅ PASS: Build info found with {} arguments",
            build_info.arguments.len()
        );
        println!("\n   📋 BUILD INFO CONTENTS:");
        let labels = [
            "Current Directory",
            "Compiler Path",
            "Source File",
            "PDB Path",
            "Command Line Args",
        ];
        for (i, arg) in build_info.arguments.iter().enumerate() {
            let label = labels.get(i).unwrap_or(&"Extra Arg");
            if arg.is_empty() {
                println!("      [{}] {} <empty>", i, label);
            } else if arg.len() > 100 {
                println!("      [{}] {} {}", i, label, &arg[..97]);
                println!("           ...{}", &arg[arg.len() - 40..]);
            } else {
                println!("      [{}] {} {}", i, label, arg);
            }
        }
    } else {
        println!("   ⚠️  INFO: No build info in this PDB (optional)");
    }

    // Verify 8: Sample IPI types from IPI HashMap
    println!("\n✓ VERIFICATION 8: Sample IPI Types from IPI Stream");
    let mut buildinfo_found = 0;
    let mut funcid_found = 0;
    let mut udtsrcline_found = 0;
    let mut stringid_found = 0;

    for (idx, type_ref) in parsed_pdb.ipi_types.iter().take(500) {
        let ty = type_ref.borrow();

        match &*ty {
            ezpdb::type_info::Type::BuildInfoType(build_info) if buildinfo_found < 2 => {
                buildinfo_found += 1;
                println!("   🔨 BuildInfo at 0x{:08x}:", idx);
                println!(
                    "      Contains {} items (refs to StringIds):",
                    build_info.items.len()
                );
                for (i, item) in build_info.items.iter().enumerate().take(5) {
                    let item_ty = item.borrow();
                    let item_str = format!("{:?}", item_ty);
                    let preview = if item_str.len() > 80 {
                        format!("{}...", &item_str[..77])
                    } else {
                        item_str
                    };
                    println!("         [{}] {}", i, preview);
                }
            }
            ezpdb::type_info::Type::FuncId(func_id) if funcid_found < 3 => {
                funcid_found += 1;
                println!("   🔧 FuncId at 0x{:08x}:", idx);
                println!("      name: {}", func_id.name);
                if let Some(ref func_type) = func_id.function_type {
                    let func_ty = func_type.borrow();
                    let func_str = format!("{:?}", func_ty);
                    let preview = if func_str.len() > 100 {
                        format!("{}...", &func_str[..97])
                    } else {
                        func_str
                    };
                    println!("      function_type: {}", preview);
                }
            }
            ezpdb::type_info::Type::UdtSrcLineType(udt_src) if udtsrcline_found < 2 => {
                udtsrcline_found += 1;
                println!("   📍 UdtSrcLine at 0x{:08x}:", idx);
                println!("      line_number: {}", udt_src.line_number);
                let udt_ty = udt_src.udt.borrow();
                let udt_str = format!("{:?}", udt_ty);
                let preview = if udt_str.len() > 100 {
                    format!("{}...", &udt_str[..97])
                } else {
                    udt_str
                };
                println!("      udt: {}", preview);
            }
            ezpdb::type_info::Type::StringId(string_id) if stringid_found < 3 => {
                stringid_found += 1;
                println!("   📝 StringId at 0x{:08x}:", idx);
                println!("      id: \"{}\"", string_id.id);
                if let Some(ref substr) = string_id.substring {
                    let substr_ty = substr.borrow();
                    println!("      substring: {:?}", substr_ty);
                }
            }
            _ => {}
        }
    }

    if buildinfo_found + funcid_found + udtsrcline_found + stringid_found == 0 {
        println!("   ℹ️  No sample IPI types found in first 500 entries");
    } else {
        println!(
            "   ✅ Displayed {} BuildInfo, {} FuncId, {} UdtSrcLine, {} StringId samples",
            buildinfo_found, funcid_found, udtsrcline_found, stringid_found
        );
    }

    // Final summary
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("                        FINAL SUMMARY");
    println!("═══════════════════════════════════════════════════════════════\n");

    let mut passed = 0;
    let mut failed = 0;
    let mut warnings = 0;

    // Count results
    if tpi_ratio >= 80.0 {
        passed += 1;
    } else {
        warnings += 1;
    }
    if ipi_ratio >= 80.0 {
        passed += 1;
    } else {
        warnings += 1;
    }
    if ipi_types_in_tpi == 0 {
        passed += 1;
    } else {
        failed += 1;
    }
    if has_funcid || has_udtsrcline {
        passed += 1;
    } else {
        warnings += 1;
    }
    if ipi_in_data_symbols == 0 {
        passed += 1;
    } else {
        failed += 1;
    }
    if !parsed_pdb.procedures.is_empty() {
        passed += 1;
    } else {
        warnings += 1;
    }

    println!("Results:");
    println!("   ✅ Passed: {}", passed);
    println!("   ❌ Failed: {}", failed);
    println!("   ⚠️  Warnings: {}", warnings);

    if failed > 0 {
        println!("\n❌ VERIFICATION FAILED - Issues detected!");
        println!("   The fix may have introduced bugs or incomplete fixes.");
    } else if warnings > 0 {
        println!("\n⚠️  VERIFICATION PASSED WITH WARNINGS");
        println!("   Core functionality works but some edge cases detected.");
    } else {
        println!("\n✅ VERIFICATION PASSED - All checks successful!");
        println!("   TPI and IPI types are properly separated.");
        println!("   Data symbols correctly resolve to TPI types.");
        println!("   No cross-contamination detected.");
    }

    println!("\n═══════════════════════════════════════════════════════════════\n");

    Ok(())
}
