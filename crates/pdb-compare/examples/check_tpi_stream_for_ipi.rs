/// Check what types are actually in the TPI stream at the indices used by IPI data symbols
/// This will help us understand if MS-PDB is putting IPI records into the TPI stream
use anyhow::Result;
use ms_pdb::codeview::types::{Leaf, TypeIndex};
use ms_pdb::Pdb;
use std::fs::File;
use std::path::Path;

fn main() -> Result<()> {
    let pdb_path = std::env::args()
        .nth(1)
        .expect("Usage: check_tpi_stream_for_ipi <pdb_path>");

    println!("═══════════════════════════════════════════════════════════════");
    println!("    CHECKING TPI STREAM FOR IPI TYPE RECORDS");
    println!("═══════════════════════════════════════════════════════════════\n");
    println!("PDB File: {}\n", pdb_path);

    // Open PDB with both ms-pdb and ezpdb
    let file = File::open(&pdb_path)?;
    let pdb = Pdb::open_from_file(file)?;
    let parsed_pdb = ezpdb::parse_pdb(Path::new(&pdb_path), None)?;

    // Get type streams
    let type_stream = pdb.read_type_stream()?;
    let ipi_stream = pdb.read_ipi_stream()?;

    println!("📊 STREAM INFO:");
    println!(
        "   TPI: 0x{:08x} to 0x{:08x} ({} types)",
        type_stream.type_index_begin().0,
        type_stream.type_index_end().0,
        type_stream.type_index_end().0 - type_stream.type_index_begin().0
    );
    println!(
        "   IPI: 0x{:08x} to 0x{:08x} ({} types)",
        ipi_stream.type_index_begin().0,
        ipi_stream.type_index_end().0,
        ipi_stream.type_index_end().0 - ipi_stream.type_index_begin().0
    );

    println!("\n🔍 FINDING DATA SYMBOLS WITH IPI TYPES...\n");

    let mut ipi_type_indices = Vec::new();

    for data_symbol in parsed_pdb.global_data.iter() {
        let ty = data_symbol.ty.borrow();
        let type_name = format!("{:?}", ty);

        // Check if this is an IPI type
        let is_ipi = type_name.contains("FuncId")
            || type_name.contains("MFuncId")
            || type_name.contains("StringId")
            || type_name.contains("SubStrList")
            || type_name.contains("BuildInfo")
            || type_name.contains("UdtSrcLine");

        if !is_ipi {
            continue;
        }

        // Extract type index from debug string
        if let Some(pos) = type_name.find("type_index: TypeIndex(") {
            let rest = &type_name[pos + 22..];
            if let Some(end) = rest.find(')') {
                if let Ok(type_idx) = rest[..end].parse::<u32>() {
                    ipi_type_indices.push((data_symbol.name.clone(), type_idx, type_name));
                }
            }
        }
    }

    println!(
        "Found {} data symbols with IPI types\n",
        ipi_type_indices.len()
    );

    println!("═══════════════════════════════════════════════════════════════");
    println!("    CHECKING WHAT'S IN TPI STREAM AT THESE INDICES");
    println!("═══════════════════════════════════════════════════════════════\n");

    let mut ipi_records_in_tpi = 0;
    let mut tpi_only_records = 0;
    let mut errors = 0;

    for (symbol_name, type_idx, ezpdb_type) in ipi_type_indices.iter().take(20) {
        println!("──────────────────────────────────────────────────────────────");
        println!("Symbol: {}", symbol_name);
        println!("Type Index: 0x{:08x} ({})", type_idx, type_idx);

        // Check what ezpdb parsed
        let ezpdb_kind = if ezpdb_type.contains("FuncId") {
            "FuncId"
        } else if ezpdb_type.contains("UdtSrcLine") {
            "UdtSrcLine"
        } else if ezpdb_type.contains("StringId") {
            "StringId"
        } else {
            "Other IPI"
        };
        println!("Ezpdb parsed as: {}", ezpdb_kind);

        // Check if in TPI range
        let in_tpi = *type_idx >= type_stream.type_index_begin().0
            && *type_idx < type_stream.type_index_end().0;
        let in_ipi = *type_idx >= ipi_stream.type_index_begin().0
            && *type_idx < ipi_stream.type_index_end().0;

        println!("   In TPI range: {}", in_tpi);
        println!("   In IPI range: {}", in_ipi);

        // Try to read from TPI stream
        if in_tpi {
            match type_stream.record(TypeIndex(*type_idx)) {
                Ok(tpi_record) => {
                    println!("\n   ✅ TPI STREAM HAS RECORD:");
                    println!(
                        "      Leaf kind: {:?} (0x{:04x})",
                        tpi_record.kind, tpi_record.kind.0
                    );

                    // Check if this is an IPI-only leaf
                    let is_ipi_leaf = matches!(
                        tpi_record.kind,
                        Leaf::LF_FUNC_ID
                            | Leaf::LF_MFUNC_ID
                            | Leaf::LF_BUILDINFO
                            | Leaf::LF_SUBSTR_LIST
                            | Leaf::LF_STRING_ID
                            | Leaf::LF_UDT_SRC_LINE
                            | Leaf::LF_UDT_MOD_SRC_LINE
                    );

                    if is_ipi_leaf {
                        println!("      ⚠️  THIS IS AN IPI-ONLY LEAF TYPE!");
                        println!("      🚨 BUG CONFIRMED: TPI stream contains IPI records!");
                        ipi_records_in_tpi += 1;
                    } else {
                        println!("      ℹ️  This is a normal TPI leaf type");
                        tpi_only_records += 1;
                    }

                    // Try to parse it
                    match tpi_record.parse() {
                        Ok(parsed) => {
                            let debug_str = format!("{:?}", parsed);
                            if let Some(first_line) = debug_str.lines().next() {
                                let preview = &first_line[..first_line.len().min(70)];
                                println!("      Data: {}...", preview);
                            }
                        }
                        Err(e) => {
                            println!("      Parse error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("\n   ❌ TPI STREAM ERROR: {}", e);
                    errors += 1;
                }
            }
        }

        // Try to read from IPI stream
        if in_ipi {
            match ipi_stream.record(TypeIndex(*type_idx)) {
                Ok(ipi_record) => {
                    println!("\n   ✅ IPI STREAM HAS RECORD:");
                    println!(
                        "      Leaf kind: {:?} (0x{:04x})",
                        ipi_record.kind, ipi_record.kind.0
                    );

                    match ipi_record.parse() {
                        Ok(parsed) => {
                            let debug_str = format!("{:?}", parsed);
                            if let Some(first_line) = debug_str.lines().next() {
                                let preview = &first_line[..first_line.len().min(70)];
                                println!("      Data: {}...", preview);
                            }
                        }
                        Err(e) => {
                            println!("      Parse error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("\n   ❌ IPI STREAM ERROR: {}", e);
                }
            }
        }

        println!();
    }

    if ipi_type_indices.len() > 20 {
        println!(
            "... checking remaining {} symbols ...\n",
            ipi_type_indices.len() - 20
        );

        for (_, type_idx, _) in ipi_type_indices.iter().skip(20) {
            if let Ok(tpi_record) = type_stream.record(TypeIndex(*type_idx)) {
                let is_ipi_leaf = matches!(
                    tpi_record.kind,
                    Leaf::LF_FUNC_ID
                        | Leaf::LF_MFUNC_ID
                        | Leaf::LF_BUILDINFO
                        | Leaf::LF_SUBSTR_LIST
                        | Leaf::LF_STRING_ID
                        | Leaf::LF_UDT_SRC_LINE
                        | Leaf::LF_UDT_MOD_SRC_LINE
                );

                if is_ipi_leaf {
                    ipi_records_in_tpi += 1;
                } else {
                    tpi_only_records += 1;
                }
            }
        }
    }

    println!("═══════════════════════════════════════════════════════════════");
    println!("                        RESULTS");
    println!("═══════════════════════════════════════════════════════════════\n");
    println!(
        "Total IPI data symbols analyzed: {}",
        ipi_type_indices.len()
    );
    println!(
        "IPI-only leaf types found in TPI stream: {}",
        ipi_records_in_tpi
    );
    println!("Normal TPI leaf types: {}", tpi_only_records);
    println!("Errors: {}", errors);

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("                        CONCLUSION");
    println!("═══════════════════════════════════════════════════════════════\n");

    if ipi_records_in_tpi > 0 {
        println!("🚨 BUG CONFIRMED IN MICROSOFT PDB GENERATION:");
        println!();
        println!(
            "   The TPI stream contains {} IPI-only type records!",
            ipi_records_in_tpi
        );
        println!();
        println!("   According to the PDB specification:");
        println!("   - TPI stream should contain: structs, unions, enums, pointers, etc.");
        println!("   - IPI stream should contain: FuncId, StringId, BuildInfo, UdtSrcLine, etc.");
        println!();
        println!("   But Microsoft's compiler/linker is placing IPI records (LF_FUNC_ID,");
        println!("   LF_UDT_SRC_LINE, etc.) into the TPI stream!");
        println!();
        println!("   This is why ezpdb is finding IPI types in data symbols - it's parsing");
        println!("   the TPI stream and correctly identifying these as IPI types.");
        println!();
        println!("   The question is: Should ezpdb:");
        println!("   1. Skip IPI records when parsing TPI stream (treat as error)?");
        println!("   2. Accept them but store separately?");
        println!("   3. Handle them as if they were meant to be there?");
    } else {
        println!("✅ No IPI-only records found in TPI stream.");
        println!("   The issue must be something else.");
    }

    println!("\n═══════════════════════════════════════════════════════════════\n");

    Ok(())
}
