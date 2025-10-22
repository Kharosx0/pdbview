/// Print the actual type indices from data symbols to see which ones are IPI types
use anyhow::Result;
use ms_pdb::codeview::types::TypeIndex;
use ms_pdb::Pdb;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

fn main() -> Result<()> {
    let pdb_path = std::env::args()
        .nth(1)
        .expect("Usage: print_ipi_type_indices <pdb_path>");

    println!("═══════════════════════════════════════════════════════════════");
    println!("    PRINTING TYPE INDICES FROM DATA SYMBOLS");
    println!("═══════════════════════════════════════════════════════════════\n");
    println!("PDB File: {}\n", pdb_path);

    // Open PDB with ms-pdb (to read raw symbol data)
    let file = File::open(&pdb_path)?;
    let pdb = Pdb::open_from_file(file)?;

    // Also parse with ezpdb to see what it found
    let parsed_pdb = ezpdb::parse_pdb(Path::new(&pdb_path), None)?;

    // Get type streams
    let type_stream = pdb.read_type_stream()?;
    let ipi_stream = pdb.read_ipi_stream()?;

    println!("📊 STREAM INFO:");
    println!(
        "   TPI: 0x{:08x} to 0x{:08x}",
        type_stream.type_index_begin().0,
        type_stream.type_index_end().0
    );
    println!(
        "   IPI: 0x{:08x} to 0x{:08x}",
        ipi_stream.type_index_begin().0,
        ipi_stream.type_index_end().0
    );

    println!("\n🔍 READING RAW DATA SYMBOLS FROM GLOBAL SYMBOL STREAM...\n");

    let gss = pdb.read_gss()?;

    let mut type_index_to_symbols: HashMap<u32, Vec<String>> = HashMap::new();
    let mut total_data_symbols = 0;

    for symbol in gss.iter_syms() {
        // Check if this is a data symbol
        use ms_pdb::codeview::syms::SymKind;
        let is_data_symbol = matches!(
            symbol.kind,
            SymKind::S_GDATA32 | SymKind::S_LDATA32 | SymKind::S_GMANDATA | SymKind::S_LMANDATA
        );

        if !is_data_symbol {
            continue;
        }

        total_data_symbols += 1;

        // Parse the symbol data
        if let Ok(sym_data) = symbol.parse() {
            use ms_pdb::codeview::syms::SymData;
            if let SymData::Data(data) = sym_data {
                let type_index = data.header.type_.get().0;
                let name = data.name.to_string();

                type_index_to_symbols
                    .entry(type_index)
                    .or_insert_with(Vec::new)
                    .push(name);
            }
        }
    }

    println!("Total data symbols found: {}", total_data_symbols);
    println!("Unique type indices: {}", type_index_to_symbols.len());

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("    ANALYZING TYPE INDICES");
    println!("═══════════════════════════════════════════════════════════════\n");

    let mut in_tpi_only = 0;
    let mut in_ipi_only = 0;
    let mut in_both = 0;
    let mut in_neither = 0;

    let mut ipi_only_examples = Vec::new();

    for (type_idx, symbol_names) in type_index_to_symbols.iter() {
        let in_tpi = *type_idx >= type_stream.type_index_begin().0
            && *type_idx < type_stream.type_index_end().0;
        let in_ipi = *type_idx >= ipi_stream.type_index_begin().0
            && *type_idx < ipi_stream.type_index_end().0;

        match (in_tpi, in_ipi) {
            (true, true) => in_both += symbol_names.len(),
            (true, false) => in_tpi_only += symbol_names.len(),
            (false, true) => {
                in_ipi_only += symbol_names.len();
                for name in symbol_names {
                    ipi_only_examples.push((*type_idx, name.clone()));
                }
            }
            (false, false) => in_neither += symbol_names.len(),
        }
    }

    println!("Data symbols with type indices:");
    println!("   In TPI only: {}", in_tpi_only);
    println!("   In IPI only: {}", in_ipi_only);
    println!("   In both TPI and IPI: {}", in_both);
    println!("   In neither: {}", in_neither);

    if in_ipi_only > 0 {
        println!(
            "\n⚠️  {} data symbols have IPI-ONLY type indices!",
            in_ipi_only
        );
        println!("\nThis should be IMPOSSIBLE according to PDB spec!");
        println!("Data symbols should reference TPI (data types), not IPI (metadata).\n");

        println!("═══════════════════════════════════════════════════════════════");
        println!("    IPI-ONLY DATA SYMBOLS (first 20)");
        println!("═══════════════════════════════════════════════════════════════\n");

        for (type_idx, name) in ipi_only_examples.iter().take(20) {
            println!("Symbol: {}", name);
            println!("   Type Index: 0x{:08x} ({})", type_idx, type_idx);
            println!("   In TPI: false");
            println!("   In IPI: true");

            // Try to read from IPI stream to see what's there
            if let Ok(ipi_record) = ipi_stream.record(TypeIndex(*type_idx)) {
                println!(
                    "   IPI Leaf: {:?} (0x{:04x})",
                    ipi_record.kind, ipi_record.kind.0
                );

                if let Ok(parsed) = ipi_record.parse() {
                    let debug_str = format!("{:?}", parsed);
                    if let Some(first_line) = debug_str.lines().next() {
                        let preview = &first_line[..first_line.len().min(70)];
                        println!("   IPI Data: {}...", preview);
                    }
                }
            }
            println!();
        }
    }

    if in_both > 0 {
        println!("\n═══════════════════════════════════════════════════════════════");
        println!("    CHECKING TYPE INDICES IN BOTH STREAMS");
        println!("═══════════════════════════════════════════════════════════════\n");

        println!(
            "⚠️  {} data symbols have type indices that exist in BOTH TPI and IPI!",
            in_both
        );
        println!("\nFor these cases, we need to determine which stream the symbols are");
        println!("actually referencing. Let's check a few examples...\n");

        let mut checked = 0;
        for (type_idx, symbol_names) in type_index_to_symbols.iter() {
            let in_tpi = *type_idx >= type_stream.type_index_begin().0
                && *type_idx < type_stream.type_index_end().0;
            let in_ipi = *type_idx >= ipi_stream.type_index_begin().0
                && *type_idx < ipi_stream.type_index_end().0;

            if in_tpi && in_ipi && checked < 10 {
                checked += 1;

                println!("──────────────────────────────────────────────────────────────");
                println!(
                    "Symbols: {} (and {} more)",
                    symbol_names[0],
                    symbol_names.len() - 1
                );
                println!("Type Index: 0x{:08x} ({})", type_idx, type_idx);

                // Check what's in TPI
                if let Ok(tpi_record) = type_stream.record(TypeIndex(*type_idx)) {
                    println!(
                        "\n   TPI has: {:?} (0x{:04x})",
                        tpi_record.kind, tpi_record.kind.0
                    );
                    if let Ok(parsed) = tpi_record.parse() {
                        let debug_str = format!("{:?}", parsed);
                        if let Some(first_line) = debug_str.lines().next() {
                            println!("   TPI data: {}", &first_line[..first_line.len().min(60)]);
                        }
                    }
                }

                // Check what's in IPI
                if let Ok(ipi_record) = ipi_stream.record(TypeIndex(*type_idx)) {
                    println!(
                        "\n   IPI has: {:?} (0x{:04x})",
                        ipi_record.kind, ipi_record.kind.0
                    );
                    if let Ok(parsed) = ipi_record.parse() {
                        let debug_str = format!("{:?}", parsed);
                        if let Some(first_line) = debug_str.lines().next() {
                            println!("   IPI data: {}", &first_line[..first_line.len().min(60)]);
                        }
                    }
                }

                // Check what ezpdb parsed
                for parsed_sym in &parsed_pdb.global_data {
                    if symbol_names.contains(&parsed_sym.name) {
                        let ty = parsed_sym.ty.borrow();
                        let type_str = format!("{:?}", ty);
                        let type_kind = if type_str.contains("FuncId") {
                            "FuncId (IPI)"
                        } else if type_str.contains("UdtSrcLine") {
                            "UdtSrcLine (IPI)"
                        } else if type_str.contains("Class") {
                            "Class (TPI)"
                        } else if type_str.contains("Union") {
                            "Union (TPI)"
                        } else if type_str.contains("Pointer") {
                            "Pointer (TPI)"
                        } else {
                            "Unknown"
                        };
                        println!("\n   ezpdb parsed as: {}", type_kind);
                        break;
                    }
                }

                println!();
            }
        }
    }

    // Now find the specific symbols where ezpdb chose IPI over TPI
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("    FINDING SYMBOLS WHERE EZPDB CHOSE IPI OVER TPI");
    println!("═══════════════════════════════════════════════════════════════\n");

    let mut ipi_chosen_count = 0;
    let mut ipi_chosen_examples = Vec::new();

    // First, build a map from symbol name to raw type index
    let mut symbol_name_to_type_idx: HashMap<String, u32> = HashMap::new();
    for symbol in gss.iter_syms() {
        use ms_pdb::codeview::syms::SymKind;
        let is_data_symbol = matches!(
            symbol.kind,
            SymKind::S_GDATA32 | SymKind::S_LDATA32 | SymKind::S_GMANDATA | SymKind::S_LMANDATA
        );

        if !is_data_symbol {
            continue;
        }

        if let Ok(sym_data) = symbol.parse() {
            use ms_pdb::codeview::syms::SymData;
            if let SymData::Data(data) = sym_data {
                let type_index = data.header.type_.get().0;
                let name = data.name.to_string();
                symbol_name_to_type_idx.insert(name, type_index);
            }
        }
    }

    for parsed_sym in &parsed_pdb.global_data {
        let ty = parsed_sym.ty.borrow();
        let type_str = format!("{:?}", ty);

        let is_ipi = type_str.contains("FuncId")
            || type_str.contains("MFuncId")
            || type_str.contains("StringId")
            || type_str.contains("UdtSrcLine");

        if is_ipi {
            ipi_chosen_count += 1;
            if ipi_chosen_examples.len() < 20 {
                let type_idx = symbol_name_to_type_idx.get(&parsed_sym.name).copied();
                ipi_chosen_examples.push((parsed_sym.name.clone(), type_str, type_idx));
            }
        }
    }

    println!(
        "🚨 Found {} symbols where ezpdb parsed as IPI types!\n",
        ipi_chosen_count
    );

    for (name, type_str, type_idx_opt) in &ipi_chosen_examples {
        println!("Symbol: {}", name);

        if let Some(type_idx) = type_idx_opt {
            println!("   Type Index: 0x{:08x} ({})", type_idx, type_idx);

            // Check what's in both streams at this index
            let in_tpi = *type_idx >= type_stream.type_index_begin().0
                && *type_idx < type_stream.type_index_end().0;
            let in_ipi = *type_idx >= ipi_stream.type_index_begin().0
                && *type_idx < ipi_stream.type_index_end().0;

            println!("   In TPI: {}", in_tpi);
            println!("   In IPI: {}", in_ipi);

            if in_tpi {
                if let Ok(tpi_record) = type_stream.record(TypeIndex(*type_idx)) {
                    println!("   TPI has: {:?}", tpi_record.kind);
                }
            }

            if in_ipi {
                if let Ok(ipi_record) = ipi_stream.record(TypeIndex(*type_idx)) {
                    println!("   IPI has: {:?}", ipi_record.kind);
                }
            }
        }

        let type_kind = if type_str.contains("FuncId") {
            "FuncId (IPI)"
        } else if type_str.contains("UdtSrcLine") {
            "UdtSrcLine (IPI)"
        } else {
            "Other IPI"
        };
        println!("   Ezpdb parsed as: {}", type_kind);

        // Extract function/type name
        if let Some(start) = type_str.find("name: \"") {
            let rest = &type_str[start + 7..];
            if let Some(end) = rest.find('"') {
                println!("   Name: {}", &rest[..end]);
            }
        }
        println!();
    }

    if ipi_chosen_count > 20 {
        println!("... and {} more\n", ipi_chosen_count - 20);
    }

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("                        CONCLUSION");
    println!("═══════════════════════════════════════════════════════════════\n");

    if in_ipi_only > 0 {
        println!("🚨 CRITICAL BUG FOUND:");
        println!();
        println!(
            "   {} data symbols reference type indices that ONLY exist in IPI!",
            in_ipi_only
        );
        println!();
        println!("   This violates the PDB specification:");
        println!("   • Data symbols describe variables (global/local/static)");
        println!("   • They should reference TPI (Type) stream for the variable's type");
        println!("   • IPI (ID/Item) stream is for metadata only (FuncId, BuildInfo, etc.)");
        println!();
        println!("   ROOT CAUSE: Microsoft's compiler/linker bug");
        println!("   The symbol records contain type indices pointing to IPI-only types.");
        println!("   This is incorrect - data symbols must reference actual data types.");
        println!();
        println!("   IMPACT ON EZPDB:");
        println!("   Currently, ezpdb only looks in TPI HashMap for data symbol types.");
        println!(
            "   For these {} symbols, the lookup fails with UnresolvedType error.",
            in_ipi_only
        );
        println!("   We need to add fallback logic to check IPI HashMap as well.");
    } else if in_both > 0 {
        println!("⚠️  AMBIGUOUS SITUATION:");
        println!();
        println!("   All data symbols have type indices in the overlapping range");
        println!(
            "   between TPI and IPI streams (both start at 0x{:08x}).",
            type_stream.type_index_begin().0
        );
        println!();
        println!("   Without checking what ezpdb actually parsed, we can't determine");
        println!("   if there's a problem. The issue analysis showed 69 symbols have");
        println!("   IPI types, which suggests ezpdb is somehow selecting IPI over TPI.");
    } else {
        println!("✅ NO ISSUE DETECTED:");
        println!();
        println!("   All data symbols have type indices that are only in TPI.");
        println!("   This is correct behavior according to the PDB specification.");
    }

    println!("\n═══════════════════════════════════════════════════════════════\n");

    Ok(())
}
