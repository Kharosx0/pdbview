//! Symbol Offset/Address Comparison Tests
//!
//! These tests validate that the offset and address calculations in the new ezpdb
//! implementation match the behavior of the old ezpdb implementation.
//!
//! # Background
//!
//! In PDB files, symbols (public symbols, procedures, data symbols) have locations
//! specified as section:offset pairs. These need to be converted to Relative Virtual
//! Addresses (RVAs) for use in analysis tools.
//!
//! The old ezpdb implementation used a pattern like:
//! ```rust
//! let offset = address_map.and_then(|address_map| {
//!     offset
//!         .to_rva(address_map)
//!         .map(|rva| u32::from(rva) as usize + base_address)
//! });
//! ```
//!
//! The new ezpdb implementation uses `section_offset_to_rva()` which:
//! 1. Looks up the section header using the section index (1-based)
//! 2. Adds the section's virtual_address to the offset
//! 3. Returns the resulting RVA
//!
//! # What These Tests Validate
//!
//! 1. **Field Mapping**: Ensures the correct fields are compared for each symbol type:
//!    - `PublicSymbol` → uses `.offset` field
//!    - `ProcedureSymbol` → uses `.offset` field (maps to `.address` in underlying data)
//!    - `DataSymbol` → uses `.offset` field
//!
//! 2. **Offset Calculation Accuracy**: Compares thousands of symbol offsets/addresses
//!    between old and new implementations to ensure they produce identical results
//!
//! 3. **Mismatch Detection**: Identifies and reports any discrepancies with detailed
//!    information for investigation
//!
//! # Expected Results
//!
//! The tests should show a very high match rate (>99.9%) between old and new
//! implementations. A small number of mismatches (<0.1%) may be acceptable due to:
//! - Edge cases in symbol handling
//! - Differences in how duplicate symbols are processed
//! - Minor variations in invalid or malformed symbol records

use std::path::PathBuf;

#[test]
fn test_symbol_offset_comparison() {
    let _ = env_logger::builder().is_test(true).try_init();

    let pdb_path = PathBuf::from("cache_test_pdbs/ntkrnlmp.pdb");
    if !pdb_path.exists() {
        eprintln!("Skipping test: {} not found", pdb_path.display());
        return;
    }

    println!("Testing symbol offset/address comparison between old and new ezpdb...");

    // Parse with both old and new parsers
    let old_data =
        pdb_compare::old_pdb::parse_pdb(&pdb_path).expect("Failed to parse with old parser");
    let new_data =
        pdb_compare::new_pdb::parse_pdb_new(&pdb_path).expect("Failed to parse with new parser");

    println!("\n=== SYMBOL COUNTS ===");
    println!(
        "Public symbols - Old: {}, New: {}",
        old_data.symbols.public_symbols.len(),
        new_data.symbols.public_symbols.len()
    );
    println!(
        "Procedures - Old: {}, New: {}",
        old_data.symbols.procedures.len(),
        new_data.symbols.procedures.len()
    );
    println!(
        "Data symbols - Old: {}, New: {}",
        old_data.symbols.data_symbols.len(),
        new_data.symbols.data_symbols.len()
    );

    // Compare public symbol offsets
    println!("\n=== COMPARING PUBLIC SYMBOL OFFSETS ===");
    let mut public_offset_mismatches = 0;
    let mut public_offset_matches = 0;
    let mut public_samples = Vec::new();

    use std::collections::HashMap;
    let old_public: HashMap<&str, &pdb_compare::old_pdb::PublicSymbol> = old_data
        .symbols
        .public_symbols
        .iter()
        .map(|s| (s.name.as_str(), s))
        .collect();
    let new_public: HashMap<&str, &pdb_compare::new_pdb::PublicSymbol> = new_data
        .symbols
        .public_symbols
        .iter()
        .map(|s| (s.name.as_str(), s))
        .collect();

    for (name, old_sym) in &old_public {
        if let Some(new_sym) = new_public.get(name) {
            // Compare offset field (public symbols use .offset)
            if old_sym.offset != new_sym.offset {
                public_offset_mismatches += 1;
                if public_samples.len() < 5 {
                    public_samples.push((name.to_string(), old_sym.offset, new_sym.offset));
                }
            } else {
                public_offset_matches += 1;
            }
        }
    }

    println!("Public symbol offset matches: {}", public_offset_matches);
    println!(
        "Public symbol offset mismatches: {}",
        public_offset_mismatches
    );

    if !public_samples.is_empty() {
        println!("\nSample public symbol offset mismatches:");
        for (name, old_offset, new_offset) in &public_samples {
            println!("  '{}': old={:?}, new={:?}", name, old_offset, new_offset);
        }
    }

    // Compare procedure offsets (procedures use .address in old, .offset in comparison)
    println!("\n=== COMPARING PROCEDURE ADDRESSES ===");
    let mut proc_offset_mismatches = 0;
    let mut proc_offset_matches = 0;
    let mut proc_samples = Vec::new();

    let old_procs: HashMap<&str, &pdb_compare::old_pdb::ProcedureSymbol> = old_data
        .symbols
        .procedures
        .iter()
        .map(|p| (p.name.as_str(), p))
        .collect();
    let new_procs: HashMap<&str, &pdb_compare::new_pdb::ProcedureSymbol> = new_data
        .symbols
        .procedures
        .iter()
        .map(|p| (p.name.as_str(), p))
        .collect();

    for (name, old_proc) in &old_procs {
        if let Some(new_proc) = new_procs.get(name) {
            // Compare offset field (which comes from address in the underlying data)
            if old_proc.offset != new_proc.offset {
                proc_offset_mismatches += 1;
                if proc_samples.len() < 5 {
                    proc_samples.push((name.to_string(), old_proc.offset, new_proc.offset));
                }
            } else {
                proc_offset_matches += 1;
            }
        }
    }

    println!("Procedure address matches: {}", proc_offset_matches);
    println!("Procedure address mismatches: {}", proc_offset_mismatches);

    if !proc_samples.is_empty() {
        println!("\nSample procedure address mismatches:");
        for (name, old_offset, new_offset) in &proc_samples {
            println!("  '{}': old={:?}, new={:?}", name, old_offset, new_offset);
        }
    }

    // Compare data symbol offsets
    println!("\n=== COMPARING DATA SYMBOL OFFSETS ===");
    let mut data_offset_mismatches = 0;
    let mut data_offset_matches = 0;
    let mut data_samples = Vec::new();

    let old_data_syms: HashMap<&str, &pdb_compare::old_pdb::DataSymbol> = old_data
        .symbols
        .data_symbols
        .iter()
        .map(|d| (d.name.as_str(), d))
        .collect();
    let new_data_syms: HashMap<&str, &pdb_compare::new_pdb::DataSymbol> = new_data
        .symbols
        .data_symbols
        .iter()
        .map(|d| (d.name.as_str(), d))
        .collect();

    for (name, old_dat) in &old_data_syms {
        if let Some(new_dat) = new_data_syms.get(name) {
            // Compare offset field (data symbols use .offset)
            if old_dat.offset != new_dat.offset {
                data_offset_mismatches += 1;
                if data_samples.len() < 5 {
                    data_samples.push((name.to_string(), old_dat.offset, new_dat.offset));
                }
            } else {
                data_offset_matches += 1;
            }
        }
    }

    println!("Data symbol offset matches: {}", data_offset_matches);
    println!("Data symbol offset mismatches: {}", data_offset_mismatches);

    if !data_samples.is_empty() {
        println!("\nSample data symbol offset mismatches:");
        for (name, old_offset, new_offset) in &data_samples {
            println!("  '{}': old={:?}, new={:?}", name, old_offset, new_offset);
        }
    }

    // Summary
    println!("\n=== OVERALL SUMMARY ===");
    let total_matches = public_offset_matches + proc_offset_matches + data_offset_matches;
    let total_mismatches =
        public_offset_mismatches + proc_offset_mismatches + data_offset_mismatches;
    let total_compared = total_matches + total_mismatches;

    println!("Total symbols compared: {}", total_compared);
    println!("Total offset/address matches: {}", total_matches);
    println!("Total offset/address mismatches: {}", total_mismatches);

    if total_compared > 0 {
        let match_percentage = (total_matches as f64 / total_compared as f64) * 100.0;
        println!("Match percentage: {:.2}%", match_percentage);
    }

    // We require 100% match - any mismatch is a bug that needs to be fixed
    assert_eq!(
        total_mismatches, 0,
        "Found {} offset/address mismatches. All symbols must match exactly! \
        Mismatches indicate a bug in either old or new ezpdb offset calculation.",
        total_mismatches
    );
}

/// This test investigates specific offset/address mismatches in detail,
/// showing the exact values and differences for debugging purposes.
#[test]
fn test_specific_offset_mismatches() {
    let _ = env_logger::builder().is_test(true).try_init();

    let pdb_path = PathBuf::from("cache_test_pdbs/ntkrnlmp.pdb");
    if !pdb_path.exists() {
        eprintln!("Skipping test: {} not found", pdb_path.display());
        return;
    }

    println!("Investigating specific offset/address mismatches...");

    // Parse with both parsers
    let old_data =
        pdb_compare::old_pdb::parse_pdb(&pdb_path).expect("Failed to parse with old parser");
    let new_data =
        pdb_compare::new_pdb::parse_pdb_new(&pdb_path).expect("Failed to parse with new parser");

    // Look for specific mismatches in detail
    use std::collections::HashMap;

    // Check data symbols for mismatches
    let old_data_syms: HashMap<&str, &pdb_compare::old_pdb::DataSymbol> = old_data
        .symbols
        .data_symbols
        .iter()
        .map(|d| (d.name.as_str(), d))
        .collect();
    let new_data_syms: HashMap<&str, &pdb_compare::new_pdb::DataSymbol> = new_data
        .symbols
        .data_symbols
        .iter()
        .map(|d| (d.name.as_str(), d))
        .collect();

    println!("\n=== DETAILED OFFSET MISMATCHES ===");
    let mut found_mismatches = false;

    for (name, old_dat) in &old_data_syms {
        if let Some(new_dat) = new_data_syms.get(name) {
            if old_dat.offset != new_dat.offset {
                found_mismatches = true;
                println!("\nData symbol '{}':", name);
                println!(
                    "  Old offset: {:?} (0x{:X?})",
                    old_dat.offset, old_dat.offset
                );
                println!(
                    "  New offset: {:?} (0x{:X?})",
                    new_dat.offset, new_dat.offset
                );

                if let (Some(old_off), Some(new_off)) = (old_dat.offset, new_dat.offset) {
                    let diff = (new_off as i64) - (old_off as i64);
                    println!("  Difference: {} (0x{:X})", diff, diff.abs());
                }
            }
        }
    }

    // Check procedure addresses for mismatches
    let old_procs: HashMap<&str, &pdb_compare::old_pdb::ProcedureSymbol> = old_data
        .symbols
        .procedures
        .iter()
        .map(|p| (p.name.as_str(), p))
        .collect();
    let new_procs: HashMap<&str, &pdb_compare::new_pdb::ProcedureSymbol> = new_data
        .symbols
        .procedures
        .iter()
        .map(|p| (p.name.as_str(), p))
        .collect();

    for (name, old_proc) in &old_procs {
        if let Some(new_proc) = new_procs.get(name) {
            if old_proc.offset != new_proc.offset {
                found_mismatches = true;
                println!("\nProcedure '{}':", name);
                println!(
                    "  Old address: {:?} (0x{:X?})",
                    old_proc.offset, old_proc.offset
                );
                println!(
                    "  New address: {:?} (0x{:X?})",
                    new_proc.offset, new_proc.offset
                );

                if let (Some(old_off), Some(new_off)) = (old_proc.offset, new_proc.offset) {
                    let diff = (new_off as i64) - (old_off as i64);
                    println!("  Difference: {} (0x{:X})", diff, diff.abs());
                }
            }
        }
    }

    // Check public symbols for mismatches
    let old_public: HashMap<&str, &pdb_compare::old_pdb::PublicSymbol> = old_data
        .symbols
        .public_symbols
        .iter()
        .map(|s| (s.name.as_str(), s))
        .collect();
    let new_public: HashMap<&str, &pdb_compare::new_pdb::PublicSymbol> = new_data
        .symbols
        .public_symbols
        .iter()
        .map(|s| (s.name.as_str(), s))
        .collect();

    for (name, old_sym) in &old_public {
        if let Some(new_sym) = new_public.get(name) {
            if old_sym.offset != new_sym.offset {
                found_mismatches = true;
                println!("\nPublic symbol '{}':", name);
                println!(
                    "  Old offset: {:?} (0x{:X?})",
                    old_sym.offset, old_sym.offset
                );
                println!(
                    "  New offset: {:?} (0x{:X?})",
                    new_sym.offset, new_sym.offset
                );

                if let (Some(old_off), Some(new_off)) = (old_sym.offset, new_sym.offset) {
                    let diff = (new_off as i64) - (old_off as i64);
                    println!("  Difference: {} (0x{:X})", diff, diff.abs());
                }
            }
        }
    }

    if !found_mismatches {
        println!("✓ No offset/address mismatches found!");
    } else {
        panic!("Found offset/address mismatches that need investigation!");
    }
}

/// This test investigates duplicate symbols and why the comparison test found a mismatch
#[test]
fn test_find_all_clfs_symbols() {
    let _ = env_logger::builder().is_test(true).try_init();

    let pdb_path = PathBuf::from("cache_test_pdbs/ntkrnlmp.pdb");
    if !pdb_path.exists() {
        eprintln!("Skipping test: {} not found", pdb_path.display());
        return;
    }

    println!("Finding all CLFS_LSN_INVALID_EXT symbols...");

    // Parse with both parsers
    let old_data =
        pdb_compare::old_pdb::parse_pdb(&pdb_path).expect("Failed to parse with old parser");
    let new_data =
        pdb_compare::new_pdb::parse_pdb_new(&pdb_path).expect("Failed to parse with new parser");

    // Find ALL symbols with this name in both parsers
    let old_clfs_symbols: Vec<_> = old_data
        .symbols
        .data_symbols
        .iter()
        .filter(|d| d.name == "CLFS_LSN_INVALID_EXT")
        .collect();

    let new_clfs_symbols: Vec<_> = new_data
        .symbols
        .data_symbols
        .iter()
        .filter(|d| d.name == "CLFS_LSN_INVALID_EXT")
        .collect();

    println!("\n=== Old ezpdb CLFS_LSN_INVALID_EXT symbols ===");
    println!("Found {} symbols", old_clfs_symbols.len());
    for (idx, sym) in old_clfs_symbols.iter().enumerate() {
        println!(
            "  [{}] Name: {}, Offset: {:?} (0x{:X?})",
            idx, sym.name, sym.offset, sym.offset
        );
    }

    println!("\n=== New ezpdb CLFS_LSN_INVALID_EXT symbols ===");
    println!("Found {} symbols", new_clfs_symbols.len());
    for (idx, sym) in new_clfs_symbols.iter().enumerate() {
        println!(
            "  [{}] Name: {}, Offset: {:?} (0x{:X?})",
            idx, sym.name, sym.offset, sym.offset
        );
    }

    // Check if there are duplicates
    if old_clfs_symbols.len() > 1 || new_clfs_symbols.len() > 1 {
        println!("\n⚠️  WARNING: Found duplicate symbols!");
        println!(
            "Old has {} instances, New has {} instances",
            old_clfs_symbols.len(),
            new_clfs_symbols.len()
        );
    }

    // Check if counts match
    if old_clfs_symbols.len() != new_clfs_symbols.len() {
        println!("\n❌ ERROR: Symbol count mismatch!");
        println!("Old parser found {} symbols", old_clfs_symbols.len());
        println!("New parser found {} symbols", new_clfs_symbols.len());
        panic!("Symbol count mismatch for CLFS_LSN_INVALID_EXT");
    }

    // Check if all offsets match
    let mut mismatches = Vec::new();
    for (old_sym, new_sym) in old_clfs_symbols.iter().zip(new_clfs_symbols.iter()) {
        if old_sym.offset != new_sym.offset {
            mismatches.push((old_sym.offset, new_sym.offset));
        }
    }

    if !mismatches.is_empty() {
        println!("\n❌ ERROR: Found {} offset mismatches", mismatches.len());
        for (idx, (old_off, new_off)) in mismatches.iter().enumerate() {
            println!(
                "  Mismatch {}: old={:?} (0x{:X?}), new={:?} (0x{:X?})",
                idx, old_off, old_off, new_off, new_off
            );
        }
        panic!("Found offset mismatches for CLFS_LSN_INVALID_EXT");
    }

    println!("\n✓ All CLFS_LSN_INVALID_EXT symbols match!");
}

/// This test verifies that both parsers produce symbols with offset/address values,
/// and that the correct fields are being accessed for each symbol type.
#[test]
fn test_symbol_offset_types() {
    let _ = env_logger::builder().is_test(true).try_init();

    let pdb_path = PathBuf::from("cache_test_pdbs/ntkrnlmp.pdb");
    if !pdb_path.exists() {
        eprintln!("Skipping test: {} not found", pdb_path.display());
        return;
    }

    println!("Testing that correct offset/address fields are used for each symbol type...");

    // Parse with both parsers
    let old_data =
        pdb_compare::old_pdb::parse_pdb(&pdb_path).expect("Failed to parse with old parser");
    let new_data =
        pdb_compare::new_pdb::parse_pdb_new(&pdb_path).expect("Failed to parse with new parser");

    // Verify that we're comparing the right fields:
    // - PublicSymbol should use .offset
    // - ProcedureSymbol should use .offset (which maps to .address in the underlying structure)
    // - DataSymbol should use .offset

    println!("\n=== FIELD VERIFICATION ===");

    // Check that we have symbols with offsets
    let pub_with_offset = old_data
        .symbols
        .public_symbols
        .iter()
        .filter(|s| s.offset.is_some())
        .count();
    let proc_with_offset = old_data
        .symbols
        .procedures
        .iter()
        .filter(|p| p.offset.is_some())
        .count();
    let data_with_offset = old_data
        .symbols
        .data_symbols
        .iter()
        .filter(|d| d.offset.is_some())
        .count();

    println!(
        "Old parser - Public symbols with offset: {}",
        pub_with_offset
    );
    println!("Old parser - Procedures with offset: {}", proc_with_offset);
    println!(
        "Old parser - Data symbols with offset: {}",
        data_with_offset
    );

    let new_pub_with_offset = new_data
        .symbols
        .public_symbols
        .iter()
        .filter(|s| s.offset.is_some())
        .count();
    let new_proc_with_offset = new_data
        .symbols
        .procedures
        .iter()
        .filter(|p| p.offset.is_some())
        .count();
    let new_data_with_offset = new_data
        .symbols
        .data_symbols
        .iter()
        .filter(|d| d.offset.is_some())
        .count();

    println!(
        "New parser - Public symbols with offset: {}",
        new_pub_with_offset
    );
    println!(
        "New parser - Procedures with offset: {}",
        new_proc_with_offset
    );
    println!(
        "New parser - Data symbols with offset: {}",
        new_data_with_offset
    );

    // Both parsers should produce symbols with offsets
    assert!(
        pub_with_offset > 0,
        "Expected public symbols with offsets from old parser"
    );
    assert!(
        proc_with_offset > 0,
        "Expected procedures with offsets from old parser"
    );
    assert!(
        data_with_offset > 0,
        "Expected data symbols with offsets from old parser"
    );

    assert!(
        new_pub_with_offset > 0,
        "Expected public symbols with offsets from new parser"
    );
    assert!(
        new_proc_with_offset > 0,
        "Expected procedures with offsets from new parser"
    );
    assert!(
        new_data_with_offset > 0,
        "Expected data symbols with offsets from new parser"
    );

    println!("\n✓ Both parsers produce symbols with offset/address values");
}
