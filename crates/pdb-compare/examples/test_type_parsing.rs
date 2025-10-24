//! Comprehensive TPI/IPI Type Parsing Test
//!
//! This example thoroughly tests all type parsing against PDB files,
//! providing detailed statistics and validation of recently added types.

use ms_pdb::types::TypeData;
use ms_pdb::{Pdb, RandomAccessFile};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("========================================");
    println!("TPI/IPI Type Parsing Validation Test");
    println!("========================================");
    println!();

    // Find all PDB files in cache_test_pdbs
    let test_dir = PathBuf::from("cache_test_pdbs");

    if !test_dir.exists() {
        eprintln!("Error: cache_test_pdbs directory not found");
        eprintln!("Please run this from the pdb-compare directory");
        return Err("Test directory not found".into());
    }

    let pdb_files = find_pdb_files(&test_dir)?;

    if pdb_files.is_empty() {
        eprintln!("Error: No PDB files found in cache_test_pdbs");
        return Err("No test files found".into());
    }

    println!("Found {} PDB file(s) to test\n", pdb_files.len());

    let mut total_stats = TestStats::default();
    let mut all_passed = true;

    for pdb_path in &pdb_files {
        println!("==========================================");
        println!(
            "Testing: {}",
            pdb_path.file_name().unwrap().to_string_lossy()
        );
        println!("==========================================");

        match test_pdb(pdb_path) {
            Ok(stats) => {
                print_stats(&stats);
                total_stats.merge(&stats);

                if stats.errors > 0 {
                    all_passed = false;
                }
            }
            Err(e) => {
                eprintln!("✗ Failed to test PDB: {}", e);
                all_passed = false;
            }
        }
        println!();
    }

    // Print final summary
    println!("==========================================");
    println!("FINAL TEST SUMMARY");
    println!("==========================================");
    println!();
    println!("PDBs tested: {}", pdb_files.len());
    println!("Total types parsed: {}", total_stats.total_types());
    println!("  TPI types: {}", total_stats.tpi_types);
    println!("  IPI types: {}", total_stats.ipi_types);
    println!();
    println!("Type Distribution:");

    let mut type_counts: Vec<_> = total_stats.type_kinds.iter().collect();
    type_counts.sort_by(|a, b| b.1.cmp(a.1));

    for (type_name, count) in type_counts {
        println!("  {:<20} {:>8}", type_name, count);
    }
    println!();

    println!("Recently Added Types:");
    println!(
        "  Alias:       {}",
        total_stats.type_kinds.get("Alias").unwrap_or(&0)
    );
    println!(
        "  VTableShape: {}",
        total_stats.type_kinds.get("VTableShape").unwrap_or(&0)
    );
    println!(
        "  VFTable:     {}",
        total_stats.type_kinds.get("VFTable").unwrap_or(&0)
    );
    println!();

    println!("Errors: {}", total_stats.errors);
    println!("Warnings: {}", total_stats.warnings);
    println!();

    // Show aggregate samples of recently added types
    if !total_stats.alias_samples.is_empty() {
        println!("📋 Alias Sample Values (across all PDBs):");
        for sample in &total_stats.alias_samples {
            println!("  {}", sample);
        }
        println!();
    }

    if !total_stats.vtable_shape_samples.is_empty() {
        println!("📋 VTableShape Sample Values (across all PDBs):");
        for sample in &total_stats.vtable_shape_samples {
            println!("  {}", sample);
        }
        println!();
    }

    if !total_stats.vftable_samples.is_empty() {
        println!("📋 VFTable Sample Values (across all PDBs):");
        for sample in &total_stats.vftable_samples {
            println!("  {}", sample);
        }
        println!();
    }

    if all_passed && total_stats.errors == 0 {
        println!("✓✓✓ ALL TESTS PASSED ✓✓✓");
        println!("All PDBs parsed successfully with no errors!");
        Ok(())
    } else {
        println!("✗✗✗ TESTS FAILED ✗✗✗");
        println!("Found errors during parsing");
        Err("Tests failed".into())
    }
}

#[derive(Default, Debug)]
struct TestStats {
    tpi_types: usize,
    ipi_types: usize,
    type_kinds: HashMap<String, usize>,
    errors: usize,
    warnings: usize,
    // Samples of recently added types for detailed inspection
    alias_samples: Vec<String>,
    vtable_shape_samples: Vec<String>,
    vftable_samples: Vec<String>,
}

impl TestStats {
    fn total_types(&self) -> usize {
        self.tpi_types + self.ipi_types
    }

    fn merge(&mut self, other: &TestStats) {
        self.tpi_types += other.tpi_types;
        self.ipi_types += other.ipi_types;
        self.errors += other.errors;
        self.warnings += other.warnings;

        for (kind, count) in &other.type_kinds {
            *self.type_kinds.entry(kind.clone()).or_insert(0) += count;
        }

        // Merge samples (keep up to 3 of each)
        self.alias_samples
            .extend(other.alias_samples.iter().cloned());
        self.alias_samples.truncate(3);
        self.vtable_shape_samples
            .extend(other.vtable_shape_samples.iter().cloned());
        self.vtable_shape_samples.truncate(3);
        self.vftable_samples
            .extend(other.vftable_samples.iter().cloned());
        self.vftable_samples.truncate(3);
    }

    fn increment_type(&mut self, type_name: &str) {
        *self.type_kinds.entry(type_name.to_string()).or_insert(0) += 1;
    }

    fn add_type_sample(&mut self, data: &TypeData) {
        match data {
            TypeData::Alias(alias) => {
                if self.alias_samples.len() < 3 {
                    self.alias_samples.push(format!("{:?}", alias));
                }
            }
            TypeData::VTableShape(vtshape) => {
                if self.vtable_shape_samples.len() < 3 {
                    self.vtable_shape_samples.push(format!("{:?}", vtshape));
                }
            }
            TypeData::VFTable(vftable) => {
                if self.vftable_samples.len() < 3 {
                    self.vftable_samples.push(format!("{:?}", vftable));
                }
            }
            _ => {}
        }
    }
}

fn find_pdb_files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut pdb_files = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.eq_ignore_ascii_case("pdb") {
                    pdb_files.push(path);
                }
            }
        }
    }

    pdb_files.sort();
    Ok(pdb_files)
}

fn test_pdb(path: &Path) -> Result<TestStats, Box<dyn std::error::Error>> {
    let pdb = Pdb::open(path)?;

    let mut stats = TestStats::default();

    // Test TPI stream
    println!("\n📊 Testing TPI Stream...");
    match test_tpi_stream(&pdb, &mut stats) {
        Ok(count) => {
            stats.tpi_types = count;
            println!("✓ TPI: {} types parsed", count);
        }
        Err(e) => {
            eprintln!("✗ TPI Error: {}", e);
            stats.errors += 1;
        }
    }

    // Test IPI stream
    println!("📊 Testing IPI Stream...");
    match test_ipi_stream(&pdb, &mut stats) {
        Ok(count) => {
            stats.ipi_types = count;
            println!("✓ IPI: {} types parsed", count);
        }
        Err(e) => {
            // IPI stream might not exist, which is OK
            let err_str = e.to_string();
            if err_str.contains("stream") || err_str.contains("not found") {
                println!("  (No IPI stream - this is OK)");
            } else {
                eprintln!("✗ IPI Error: {}", e);
                stats.errors += 1;
            }
        }
    }

    Ok(stats)
}

fn test_tpi_stream(
    pdb: &Pdb<RandomAccessFile>,
    stats: &mut TestStats,
) -> Result<usize, Box<dyn std::error::Error>> {
    let type_stream = pdb.read_type_stream()?;

    let mut count = 0;
    let mut parse_errors = 0;

    for record in type_stream.iter_type_records() {
        count += 1;

        // Try to parse the type record
        match record.parse() {
            Ok(data) => {
                // Categorize the type
                let type_name = get_type_name(&data);
                stats.increment_type(type_name);
                // Collect samples of recently added types
                stats.add_type_sample(&data);
            }
            Err(e) => {
                parse_errors += 1;
                if parse_errors <= 5 {
                    eprintln!("  Parse error: {}", e);
                }
            }
        }
    }

    if parse_errors > 0 {
        stats.errors += parse_errors;
        eprintln!("  Total parse errors: {}", parse_errors);
    }

    Ok(count)
}

fn test_ipi_stream(
    pdb: &Pdb<RandomAccessFile>,
    stats: &mut TestStats,
) -> Result<usize, Box<dyn std::error::Error>> {
    let ipi_stream = pdb.read_ipi_stream()?;

    let mut count = 0;
    let mut parse_errors = 0;

    for record in ipi_stream.iter_type_records() {
        count += 1;

        // Try to parse the type record
        match record.parse() {
            Ok(data) => {
                // Categorize the type
                let type_name = get_type_name(&data);
                stats.increment_type(type_name);
                // Collect samples of recently added types
                stats.add_type_sample(&data);
            }
            Err(e) => {
                parse_errors += 1;
                if parse_errors <= 5 {
                    eprintln!("  Parse error: {}", e);
                }
            }
        }
    }

    if parse_errors > 0 {
        stats.errors += parse_errors;
        eprintln!("  Total parse errors: {}", parse_errors);
    }

    Ok(count)
}

fn get_type_name(data: &TypeData) -> &'static str {
    match data {
        TypeData::Array(_) => "Array",
        TypeData::Struct(_) => "Struct",
        TypeData::Union(_) => "Union",
        TypeData::Enum(_) => "Enum",
        TypeData::Proc(_) => "Proc",
        TypeData::MemberFunc(_) => "MemberFunc",
        TypeData::VTableShape(_) => "VTableShape",
        TypeData::Pointer(_) => "Pointer",
        TypeData::Modifier(_) => "Modifier",
        TypeData::Bitfield(_) => "Bitfield",
        TypeData::FieldList(_) => "FieldList",
        TypeData::MethodList(_) => "MethodList",
        TypeData::ArgList(_) => "ArgList",
        TypeData::Alias(_) => "Alias",
        TypeData::UdtSrcLine(_) => "UdtSrcLine",
        TypeData::UdtModSrcLine(_) => "UdtModSrcLine",
        TypeData::FuncId(_) => "FuncId",
        TypeData::MFuncId(_) => "MFuncId",
        TypeData::StringId(_) => "StringId",
        TypeData::SubStrList(_) => "SubStrList",
        TypeData::BuildInfo(_) => "BuildInfo",
        TypeData::VFTable(_) => "VFTable",
        TypeData::Unknown => "Unknown",
    }
}

fn print_stats(stats: &TestStats) {
    println!("\n📈 Statistics:");
    println!("  TPI Types: {}", stats.tpi_types);
    println!("  IPI Types: {}", stats.ipi_types);
    println!("  Total: {}", stats.total_types());

    println!("\n🔍 Type Distribution:");
    let mut type_counts: Vec<_> = stats.type_kinds.iter().collect();
    type_counts.sort_by(|a, b| b.1.cmp(a.1));

    for (type_name, count) in type_counts.iter().take(10) {
        println!("  {:<20} {:>6}", type_name, count);
    }

    if type_counts.len() > 10 {
        println!("  ... and {} more types", type_counts.len() - 10);
    }

    println!("\n✨ Recently Added Types:");
    println!(
        "  Alias:       {:>6}",
        stats.type_kinds.get("Alias").unwrap_or(&0)
    );
    println!(
        "  VTableShape: {:>6}",
        stats.type_kinds.get("VTableShape").unwrap_or(&0)
    );
    println!(
        "  VFTable:     {:>6}",
        stats.type_kinds.get("VFTable").unwrap_or(&0)
    );

    // Show sample values for recently added types
    if !stats.alias_samples.is_empty() {
        println!("\n📋 Alias Sample Values:");
        for sample in &stats.alias_samples {
            println!("  {}", sample);
        }
    }

    if !stats.vtable_shape_samples.is_empty() {
        println!("\n📋 VTableShape Sample Values:");
        for sample in &stats.vtable_shape_samples {
            println!("  {}", sample);
        }
    }

    if !stats.vftable_samples.is_empty() {
        println!("\n📋 VFTable Sample Values:");
        for sample in &stats.vftable_samples {
            println!("  {}", sample);
        }
    }

    if stats.errors > 0 {
        println!("\n❌ Errors: {}", stats.errors);
    } else {
        println!("\n✅ No errors!");
    }

    if stats.warnings > 0 {
        println!("⚠️  Warnings: {}", stats.warnings);
    }
}
