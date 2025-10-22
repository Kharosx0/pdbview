/// Scan the TPI stream to see if it contains IPI-only leaf types
/// This will help us determine if Microsoft is putting IPI records into the TPI stream
use anyhow::Result;
use ms_pdb::codeview::types::Leaf;
use ms_pdb::Pdb;
use std::collections::HashMap;
use std::fs::File;

fn main() -> Result<()> {
    let pdb_path = std::env::args()
        .nth(1)
        .expect("Usage: scan_tpi_for_ipi_leaves <pdb_path>");

    println!("═══════════════════════════════════════════════════════════════");
    println!("    SCANNING TPI STREAM FOR IPI-ONLY LEAF TYPES");
    println!("═══════════════════════════════════════════════════════════════\n");
    println!("PDB File: {}\n", pdb_path);

    // Open PDB
    let file = File::open(&pdb_path)?;
    let pdb = Pdb::open_from_file(file)?;

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

    println!("\n🔍 SCANNING TPI STREAM...\n");

    let mut tpi_leaf_counts: HashMap<Leaf, usize> = HashMap::new();
    let mut ipi_only_count = 0;
    let mut total_records = 0;

    for type_record in type_stream.iter_type_records() {
        total_records += 1;
        let leaf = type_record.kind;
        *tpi_leaf_counts.entry(leaf).or_insert(0) += 1;

        // Check if this is an IPI-only leaf type
        let is_ipi_only = matches!(
            leaf,
            Leaf::LF_FUNC_ID
                | Leaf::LF_MFUNC_ID
                | Leaf::LF_BUILDINFO
                | Leaf::LF_SUBSTR_LIST
                | Leaf::LF_STRING_ID
                | Leaf::LF_UDT_SRC_LINE
                | Leaf::LF_UDT_MOD_SRC_LINE
        );

        if is_ipi_only {
            ipi_only_count += 1;
        }
    }

    println!("Total TPI records scanned: {}", total_records);
    println!("IPI-only leaf types found in TPI: {}", ipi_only_count);

    if ipi_only_count > 0 {
        println!("\n⚠️  IPI-ONLY LEAF TYPES IN TPI STREAM:");
        println!("────────────────────────────────────────");

        let ipi_leaves = [
            Leaf::LF_FUNC_ID,
            Leaf::LF_MFUNC_ID,
            Leaf::LF_BUILDINFO,
            Leaf::LF_SUBSTR_LIST,
            Leaf::LF_STRING_ID,
            Leaf::LF_UDT_SRC_LINE,
            Leaf::LF_UDT_MOD_SRC_LINE,
        ];

        for leaf in &ipi_leaves {
            if let Some(&count) = tpi_leaf_counts.get(leaf) {
                println!("   {:?}: {} records", leaf, count);
            }
        }
    }

    println!("\n🔍 SCANNING IPI STREAM...\n");

    let mut ipi_leaf_counts: HashMap<Leaf, usize> = HashMap::new();
    let mut ipi_total_records = 0;

    for type_record in ipi_stream.iter_type_records() {
        ipi_total_records += 1;
        let leaf = type_record.kind;
        *ipi_leaf_counts.entry(leaf).or_insert(0) += 1;
    }

    println!("Total IPI records scanned: {}", ipi_total_records);

    println!("\n📊 IPI STREAM LEAF BREAKDOWN:");
    println!("────────────────────────────────────────");

    let mut ipi_leaves_vec: Vec<_> = ipi_leaf_counts.iter().collect();
    ipi_leaves_vec.sort_by_key(|(_, count)| std::cmp::Reverse(**count));

    for (leaf, count) in ipi_leaves_vec.iter().take(15) {
        println!("   {:?}: {} records", leaf, count);
    }

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("                        CONCLUSION");
    println!("═══════════════════════════════════════════════════════════════\n");

    if ipi_only_count > 0 {
        println!("🚨 BUG CONFIRMED: MICROSOFT PDB GENERATION ERROR");
        println!();
        println!(
            "   The TPI stream contains {} records with IPI-only leaf types!",
            ipi_only_count
        );
        println!();
        println!("   According to the CodeView specification:");
        println!("   • Leaf types 0x1601-0x1607 are IPI-only (FuncId, StringId, etc.)");
        println!("   • These should NEVER appear in the TPI stream");
        println!("   • TPI stream is for data types (structs, unions, pointers, etc.)");
        println!("   • IPI stream is for metadata (function IDs, build info, etc.)");
        println!();
        println!("   This explains why ezpdb finds IPI types in data symbols:");
        println!("   1. Microsoft's compiler/linker puts IPI records into TPI stream");
        println!("   2. ezpdb correctly parses the TPI stream");
        println!("   3. When data symbols reference these type indices, they get IPI types");
        println!();
        println!("   SOLUTION OPTIONS:");
        println!("   A) STRICT: Skip IPI leaf types when parsing TPI (may break symbols)");
        println!("   B) PERMISSIVE: Accept IPI types in TPI but warn about it");
        println!("   C) DUAL-PARSE: Store IPI types from TPI in separate map");
        println!("   D) WORKAROUND: Check IPI stream first for data symbols (current behavior)");
    } else {
        println!("✅ NO ISSUE DETECTED");
        println!();
        println!("   The TPI stream does not contain any IPI-only leaf types.");
        println!("   This PDB file follows the specification correctly.");
        println!("   If IPI types are appearing in data symbols, the issue is");
        println!("   in how ezpdb is resolving type indices.");
    }

    println!("\n═══════════════════════════════════════════════════════════════\n");

    Ok(())
}
