/// Compare type counts before and after the fix to ensure we didn't lose any types
/// This tool shows detailed statistics about what types are being parsed
use anyhow::Result;
use ms_pdb::codeview::types::Leaf;
use ms_pdb::Pdb;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

fn main() -> Result<()> {
    let pdb_path = std::env::args()
        .nth(1)
        .expect("Usage: compare_type_counts <pdb_path>");

    println!("═══════════════════════════════════════════════════════════════");
    println!("         TYPE COUNT COMPARISON AND ANALYSIS");
    println!("═══════════════════════════════════════════════════════════════\n");
    println!("PDB File: {}\n", pdb_path);

    // Open PDB with ms-pdb to scan raw streams
    let file = File::open(&pdb_path)?;
    let pdb = Pdb::open_from_file(file)?;

    // Get type streams
    let type_stream = pdb.read_type_stream()?;
    let ipi_stream = pdb.read_ipi_stream()?;

    println!("📊 SCANNING RAW TPI STREAM...\n");

    let mut tpi_leaf_counts: HashMap<Leaf, usize> = HashMap::new();
    let mut tpi_total = 0;

    for type_record in type_stream.iter_type_records() {
        tpi_total += 1;
        let leaf = type_record.kind;
        *tpi_leaf_counts.entry(leaf).or_insert(0) += 1;
    }

    println!("TPI Stream Leaf Type Distribution:");
    println!("   Total records: {}\n", tpi_total);

    let mut tpi_vec: Vec<_> = tpi_leaf_counts.iter().collect();
    tpi_vec.sort_by_key(|(_, count)| std::cmp::Reverse(**count));

    for (leaf, count) in tpi_vec.iter().take(20) {
        let percentage = (**count as f64 / tpi_total as f64) * 100.0;
        println!("   {:?}: {} ({:.1}%)", leaf, count, percentage);
    }

    println!("\n📊 SCANNING RAW IPI STREAM...\n");

    let mut ipi_leaf_counts: HashMap<Leaf, usize> = HashMap::new();
    let mut ipi_total = 0;

    for type_record in ipi_stream.iter_type_records() {
        ipi_total += 1;
        let leaf = type_record.kind;
        *ipi_leaf_counts.entry(leaf).or_insert(0) += 1;
    }

    println!("IPI Stream Leaf Type Distribution:");
    println!("   Total records: {}\n", ipi_total);

    let mut ipi_vec: Vec<_> = ipi_leaf_counts.iter().collect();
    ipi_vec.sort_by_key(|(_, count)| std::cmp::Reverse(**count));

    for (leaf, count) in ipi_vec.iter().take(20) {
        let percentage = (**count as f64 / ipi_total as f64) * 100.0;
        println!("   {:?}: {} ({:.1}%)", leaf, count, percentage);
    }

    println!("\n📊 PARSING WITH EZPDB...\n");

    // Parse with ezpdb
    let parsed_pdb = ezpdb::parse_pdb(Path::new(&pdb_path), None)?;

    println!("EZPDB Parsed Results:");
    println!("   TPI HashMap: {} types", parsed_pdb.types.len());
    println!("   IPI HashMap: {} types", parsed_pdb.ipi_types.len());

    // Analyze what types were parsed into each HashMap
    println!("\n📊 ANALYZING EZPDB TPI HASHMAP...\n");

    let mut ezpdb_tpi_counts: HashMap<String, usize> = HashMap::new();

    for type_ref in parsed_pdb.types.values() {
        let ty = type_ref.borrow();
        let type_name = format!("{:?}", ty);
        let variant = type_name.split('(').next().unwrap_or("Unknown").to_string();
        *ezpdb_tpi_counts.entry(variant).or_insert(0) += 1;
    }

    println!("EZPDB TPI Type Distribution:");
    let mut ezpdb_tpi_vec: Vec<_> = ezpdb_tpi_counts.iter().collect();
    ezpdb_tpi_vec.sort_by_key(|(_, count)| std::cmp::Reverse(**count));

    for (type_name, count) in ezpdb_tpi_vec.iter().take(20) {
        let percentage = (**count as f64 / parsed_pdb.types.len() as f64) * 100.0;
        println!("   {}: {} ({:.1}%)", type_name, count, percentage);
    }

    println!("\n📊 ANALYZING EZPDB IPI HASHMAP...\n");

    let mut ezpdb_ipi_counts: HashMap<String, usize> = HashMap::new();

    for type_ref in parsed_pdb.ipi_types.values() {
        let ty = type_ref.borrow();
        let type_name = format!("{:?}", ty);
        let variant = type_name.split('(').next().unwrap_or("Unknown").to_string();
        *ezpdb_ipi_counts.entry(variant).or_insert(0) += 1;
    }

    println!("EZPDB IPI Type Distribution:");
    let mut ezpdb_ipi_vec: Vec<_> = ezpdb_ipi_counts.iter().collect();
    ezpdb_ipi_vec.sort_by_key(|(_, count)| std::cmp::Reverse(**count));

    for (type_name, count) in ezpdb_ipi_vec.iter().take(20) {
        let percentage = (**count as f64 / parsed_pdb.ipi_types.len() as f64) * 100.0;
        println!("   {}: {} ({:.1}%)", type_name, count, percentage);
    }

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("                    COMPARISON ANALYSIS");
    println!("═══════════════════════════════════════════════════════════════\n");

    let tpi_parse_rate = (parsed_pdb.types.len() as f64 / tpi_total as f64) * 100.0;
    let ipi_parse_rate = (parsed_pdb.ipi_types.len() as f64 / ipi_total as f64) * 100.0;

    println!("TPI Stream:");
    println!("   Raw records in stream: {}", tpi_total);
    println!("   Parsed into HashMap: {}", parsed_pdb.types.len());
    println!("   Parse rate: {:.1}%", tpi_parse_rate);
    println!(
        "   Unparsed: {} records",
        tpi_total - parsed_pdb.types.len()
    );

    if tpi_parse_rate < 80.0 {
        println!("   ⚠️  WARNING: Low parse rate!");
    } else if tpi_parse_rate < 95.0 {
        println!("   ✅ GOOD: Acceptable parse rate");
    } else {
        println!("   ✅ EXCELLENT: High parse rate");
    }

    println!("\nIPI Stream:");
    println!("   Raw records in stream: {}", ipi_total);
    println!("   Parsed into HashMap: {}", parsed_pdb.ipi_types.len());
    println!("   Parse rate: {:.1}%", ipi_parse_rate);
    println!(
        "   Unparsed: {} records",
        ipi_total - parsed_pdb.ipi_types.len()
    );

    if ipi_parse_rate < 80.0 {
        println!("   ⚠️  WARNING: Low parse rate!");
    } else if ipi_parse_rate < 95.0 {
        println!("   ✅ GOOD: Acceptable parse rate");
    } else {
        println!("   ✅ EXCELLENT: High parse rate");
    }

    // Check for IPI-only leaf types in TPI HashMap
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("              STREAM SEPARATION VERIFICATION");
    println!("═══════════════════════════════════════════════════════════════\n");

    let ipi_only_types = [
        "FuncId",
        "MFuncId",
        "StringId",
        "SubStrList",
        "BuildInfoType",
        "UdtSrcLineType",
    ];

    println!("Checking for IPI-only types in TPI HashMap:");
    let mut found_ipi_in_tpi = false;
    for ipi_type in &ipi_only_types {
        if let Some(count) = ezpdb_tpi_counts.get(*ipi_type) {
            println!("   ❌ Found {} {} types in TPI HashMap", count, ipi_type);
            found_ipi_in_tpi = true;
        }
    }

    if !found_ipi_in_tpi {
        println!("   ✅ PASS: No IPI-only types found in TPI HashMap");
    }

    println!("\nChecking for IPI-only types in IPI HashMap:");
    let mut ipi_type_count = 0;
    for ipi_type in &ipi_only_types {
        if let Some(count) = ezpdb_ipi_counts.get(*ipi_type) {
            println!("   ✅ Found {} {} types", count, ipi_type);
            ipi_type_count += count;
        }
    }

    if ipi_type_count == 0 {
        println!("   ⚠️  WARNING: No expected IPI types found in IPI HashMap");
    } else {
        println!(
            "   ✅ PASS: IPI HashMap contains {} IPI-specific types",
            ipi_type_count
        );
    }

    // Compare TPI-only leaf types in raw stream vs parsed
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("                   DETAILED LEAF COMPARISON");
    println!("═══════════════════════════════════════════════════════════════\n");

    println!("Comparing major TPI leaf types (raw stream vs parsed):");

    let tpi_leaf_types = [
        (Leaf::LF_STRUCTURE, "Class"),
        (Leaf::LF_UNION, "Union"),
        (Leaf::LF_ENUM, "Enumeration"),
        (Leaf::LF_POINTER, "Pointer"),
        (Leaf::LF_ARRAY, "Array"),
        (Leaf::LF_PROCEDURE, "Procedure"),
        (Leaf::LF_MFUNCTION, "MemberFunction"),
        (Leaf::LF_MODIFIER, "Modifier"),
        (Leaf::LF_FIELDLIST, "FieldList"),
        (Leaf::LF_ARGLIST, "ArgumentList"),
    ];

    let mut all_matched = true;
    for (leaf, ezpdb_name) in &tpi_leaf_types {
        let raw_count = tpi_leaf_counts.get(leaf).copied().unwrap_or(0);
        let parsed_count = ezpdb_tpi_counts.get(*ezpdb_name).copied().unwrap_or(0);

        if raw_count > 0 {
            let match_rate = (parsed_count as f64 / raw_count as f64) * 100.0;
            let status = if match_rate >= 95.0 {
                "✅"
            } else if match_rate >= 80.0 {
                "⚠️"
            } else {
                "❌"
            };

            println!(
                "   {} {:?}: {} raw → {} parsed ({:.1}%)",
                status, leaf, raw_count, parsed_count, match_rate
            );

            if match_rate < 95.0 {
                all_matched = false;
            }
        }
    }

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("                        FINAL VERDICT");
    println!("═══════════════════════════════════════════════════════════════\n");

    if !found_ipi_in_tpi && tpi_parse_rate >= 80.0 && ipi_parse_rate >= 80.0 && ipi_type_count > 0 {
        println!("✅ ALL CHECKS PASSED!");
        println!("\n   • TPI and IPI streams are properly separated");
        println!("   • Parse rates are acceptable");
        println!("   • No IPI types leaked into TPI HashMap");
        println!("   • IPI HashMap contains expected metadata types");
        println!("\n   The fix is working correctly!");
    } else {
        println!("⚠️  ISSUES DETECTED:");
        if found_ipi_in_tpi {
            println!("   • IPI types found in TPI HashMap (BUG!)");
        }
        if tpi_parse_rate < 80.0 {
            println!("   • Low TPI parse rate ({:.1}%)", tpi_parse_rate);
        }
        if ipi_parse_rate < 80.0 {
            println!("   • Low IPI parse rate ({:.1}%)", ipi_parse_rate);
        }
        if ipi_type_count == 0 {
            println!("   • No IPI types found in IPI HashMap");
        }
    }

    println!("\n═══════════════════════════════════════════════════════════════\n");

    Ok(())
}
