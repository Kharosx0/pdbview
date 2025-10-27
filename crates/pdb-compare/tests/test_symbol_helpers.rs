//! Tests for Symbol Helper Methods and Duplicate Detection
//!
//! This test file validates:
//! 1. Helper methods on symbol types (has_valid_location, format_location, etc.)
//! 2. Duplicate symbol detection and handling
//! 3. Address range queries (contains_rva, contains_address)

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Helper to get a test PDB path
fn get_test_pdb() -> Option<PathBuf> {
    let pdb_path = PathBuf::from("cache_test_pdbs/ntkrnlmp.pdb");
    if pdb_path.exists() {
        Some(pdb_path)
    } else {
        None
    }
}

#[test]
fn test_has_valid_location_helper() {
    let Some(pdb_path) = get_test_pdb() else {
        eprintln!("Skipping test: test PDB not found");
        return;
    };

    println!("Testing has_valid_location() helper method...");

    let pdb = ezpdb::parse_pdb(&pdb_path, None).expect("Failed to parse PDB");

    let mut valid_count = 0;
    let mut invalid_count = 0;

    // Test PublicSymbol::has_valid_location()
    for symbol in &pdb.public_symbols {
        let has_location = symbol.has_valid_location();
        let expected = symbol.section.is_some() && symbol.rva.is_some();

        assert_eq!(
            has_location, expected,
            "PublicSymbol '{}': has_valid_location() returned {} but expected {}",
            symbol.name, has_location, expected
        );

        if has_location {
            valid_count += 1;
        } else {
            invalid_count += 1;
        }
    }

    // Test Procedure::has_valid_location()
    for proc in &pdb.procedures {
        let has_location = proc.has_valid_location();
        let expected = proc.section.is_some() && proc.rva.is_some();

        assert_eq!(
            has_location, expected,
            "Procedure '{}': has_valid_location() returned {} but expected {}",
            proc.name, has_location, expected
        );

        if has_location {
            valid_count += 1;
        } else {
            invalid_count += 1;
        }
    }

    // Test Data::has_valid_location()
    for data in &pdb.global_data {
        let has_location = data.has_valid_location();
        let expected = data.section.is_some() && data.rva.is_some();

        assert_eq!(
            has_location, expected,
            "Data '{}': has_valid_location() returned {} but expected {}",
            data.name, has_location, expected
        );

        if has_location {
            valid_count += 1;
        } else {
            invalid_count += 1;
        }
    }

    println!(
        "✓ has_valid_location() works correctly: {} valid, {} invalid locations",
        valid_count, invalid_count
    );
}

#[test]
fn test_section_and_offset_helper() {
    let Some(pdb_path) = get_test_pdb() else {
        eprintln!("Skipping test: test PDB not found");
        return;
    };

    println!("Testing section_and_offset() helper method...");

    let pdb = ezpdb::parse_pdb(&pdb_path, None).expect("Failed to parse PDB");

    let mut checked = 0;

    // Test PublicSymbol::section_and_offset()
    for symbol in &pdb.public_symbols {
        let result = symbol.section_and_offset();

        if let Some(section) = symbol.section {
            // Should return Some with matching section and offset
            assert_eq!(
                result,
                Some((section, symbol.section_offset)),
                "PublicSymbol '{}': section_and_offset() mismatch",
                symbol.name
            );
        } else {
            // Should return None if section is None
            assert_eq!(
                result, None,
                "PublicSymbol '{}': section_and_offset() should be None when section is None",
                symbol.name
            );
        }

        checked += 1;
    }

    // Test Procedure::section_and_offset()
    for proc in &pdb.procedures {
        let result = proc.section_and_offset();

        if let Some(section) = proc.section {
            assert_eq!(
                result,
                Some((section, proc.section_offset)),
                "Procedure '{}': section_and_offset() mismatch",
                proc.name
            );
        } else {
            assert_eq!(
                result, None,
                "Procedure '{}': section_and_offset() should be None",
                proc.name
            );
        }

        checked += 1;
    }

    // Test Data::section_and_offset()
    for data in &pdb.global_data {
        let result = data.section_and_offset();

        if let Some(section) = data.section {
            assert_eq!(
                result,
                Some((section, data.section_offset)),
                "Data '{}': section_and_offset() mismatch",
                data.name
            );
        } else {
            assert_eq!(
                result, None,
                "Data '{}': section_and_offset() should be None",
                data.name
            );
        }

        checked += 1;
    }

    println!(
        "✓ section_and_offset() works correctly for {} symbols",
        checked
    );
}

#[test]
fn test_format_location_helper() {
    let Some(pdb_path) = get_test_pdb() else {
        eprintln!("Skipping test: test PDB not found");
        return;
    };

    println!("Testing format_location() helper method...");

    let pdb = ezpdb::parse_pdb(&pdb_path, None).expect("Failed to parse PDB");

    let mut samples = Vec::new();

    // Test PublicSymbol::format_location()
    for symbol in pdb.public_symbols.iter().take(5) {
        let formatted = symbol.format_location();

        // Format should be "xxxx:yyyyyyyy" (4 hex digits : 8 hex digits)
        assert_eq!(
            formatted.len(),
            13,
            "format_location() should return 'xxxx:yyyyyyyy' format (13 chars)"
        );
        assert_eq!(
            &formatted[4..5],
            ":",
            "format_location() should have ':' at position 4"
        );

        // Verify the values match
        let expected_section = symbol.section.unwrap_or(0);
        let expected_offset = symbol.section_offset;
        let expected = format!("{:04x}:{:08x}", expected_section, expected_offset);

        assert_eq!(
            formatted, expected,
            "PublicSymbol '{}': format_location() mismatch",
            symbol.name
        );

        samples.push(format!(
            "  {} -> {}",
            symbol.name.chars().take(30).collect::<String>(),
            formatted
        ));
    }

    // Test Procedure::format_location()
    for proc in pdb.procedures.iter().take(5) {
        let formatted = proc.format_location();
        assert_eq!(formatted.len(), 13);

        let expected_section = proc.section.unwrap_or(0);
        let expected_offset = proc.section_offset;
        let expected = format!("{:04x}:{:08x}", expected_section, expected_offset);

        assert_eq!(
            formatted, expected,
            "Procedure '{}': format_location() mismatch",
            proc.name
        );

        samples.push(format!(
            "  {} -> {}",
            proc.name.chars().take(30).collect::<String>(),
            formatted
        ));
    }

    // Test Data::format_location()
    for data in pdb.global_data.iter().take(5) {
        let formatted = data.format_location();
        assert_eq!(formatted.len(), 13);

        let expected_section = data.section.unwrap_or(0);
        let expected_offset = data.section_offset;
        let expected = format!("{:04x}:{:08x}", expected_section, expected_offset);

        assert_eq!(
            formatted, expected,
            "Data '{}': format_location() mismatch",
            data.name
        );

        samples.push(format!(
            "  {} -> {}",
            data.name.chars().take(30).collect::<String>(),
            formatted
        ));
    }

    println!("✓ format_location() works correctly. Sample outputs:");
    for sample in samples {
        println!("{}", sample);
    }
}

#[test]
fn test_procedure_end_rva_and_address() {
    let Some(pdb_path) = get_test_pdb() else {
        eprintln!("Skipping test: test PDB not found");
        return;
    };

    println!("Testing Procedure::end_rva() and end_address() methods...");

    let base_addr = 0x140000000usize;
    let pdb = ezpdb::parse_pdb(&pdb_path, Some(base_addr)).expect("Failed to parse PDB");

    let mut checked_rva = 0;
    let mut checked_address = 0;

    for proc in &pdb.procedures {
        // Test end_rva()
        if let Some(start_rva) = proc.rva {
            let end_rva = proc.end_rva();
            assert!(
                end_rva.is_some(),
                "Procedure '{}': end_rva() should return Some when rva is Some",
                proc.name
            );

            let expected_end = start_rva.saturating_add(proc.len);
            assert_eq!(
                end_rva,
                Some(expected_end),
                "Procedure '{}': end_rva() mismatch. Expected start({:#x}) + len({:#x}) = {:#x}",
                proc.name,
                start_rva,
                proc.len,
                expected_end
            );

            checked_rva += 1;
        } else {
            assert_eq!(
                proc.end_rva(),
                None,
                "Procedure '{}': end_rva() should be None when rva is None",
                proc.name
            );
        }

        // Test end_address()
        if let Some(start_addr) = proc.address {
            let end_addr = proc.end_address();
            assert!(
                end_addr.is_some(),
                "Procedure '{}': end_address() should return Some when address is Some",
                proc.name
            );

            let expected_end = start_addr.saturating_add(proc.len);
            assert_eq!(
                end_addr,
                Some(expected_end),
                "Procedure '{}': end_address() mismatch",
                proc.name
            );

            checked_address += 1;
        } else {
            assert_eq!(
                proc.end_address(),
                None,
                "Procedure '{}': end_address() should be None when address is None",
                proc.name
            );
        }
    }

    println!(
        "✓ end_rva() and end_address() work correctly ({} rva, {} address checks)",
        checked_rva, checked_address
    );
}

#[test]
fn test_procedure_contains_rva() {
    let Some(pdb_path) = get_test_pdb() else {
        eprintln!("Skipping test: test PDB not found");
        return;
    };

    println!("Testing Procedure::contains_rva() method...");

    let pdb = ezpdb::parse_pdb(&pdb_path, None).expect("Failed to parse PDB");

    let mut checked = 0;
    let mut overlap_count = 0;

    for proc in &pdb.procedures {
        if let Some(start_rva) = proc.rva {
            let end_rva = start_rva.saturating_add(proc.len);

            // Test that start RVA is contained
            assert!(
                proc.contains_rva(start_rva),
                "Procedure '{}': should contain its start RVA {:#x}",
                proc.name,
                start_rva
            );

            // Test that one byte before start is NOT contained
            if start_rva > 0 {
                assert!(
                    !proc.contains_rva(start_rva - 1),
                    "Procedure '{}': should NOT contain RVA before start ({:#x})",
                    proc.name,
                    start_rva - 1
                );
            }

            // Test that end RVA is NOT contained (range is [start, end))
            assert!(
                !proc.contains_rva(end_rva),
                "Procedure '{}': should NOT contain end RVA {:#x} (exclusive range)",
                proc.name,
                end_rva
            );

            // Test middle of procedure
            if proc.len > 2 {
                let mid_rva = start_rva + (proc.len / 2);
                assert!(
                    proc.contains_rva(mid_rva),
                    "Procedure '{}': should contain middle RVA {:#x}",
                    proc.name,
                    mid_rva
                );
            }

            checked += 1;

            // Check for overlapping procedures
            for other_proc in &pdb.procedures {
                if proc.name == other_proc.name {
                    continue;
                }

                if let Some(other_rva) = other_proc.rva {
                    if proc.contains_rva(other_rva) {
                        overlap_count += 1;
                    }
                }
            }
        }
    }

    println!(
        "✓ contains_rva() works correctly for {} procedures",
        checked
    );

    if overlap_count > 0 {
        println!(
            "  Note: Found {} procedure overlaps (this can happen with inline functions)",
            overlap_count
        );
    }
}

#[test]
fn test_procedure_contains_address() {
    let Some(pdb_path) = get_test_pdb() else {
        eprintln!("Skipping test: test PDB not found");
        return;
    };

    println!("Testing Procedure::contains_address() method...");

    let base_addr = 0x140000000usize;
    let pdb = ezpdb::parse_pdb(&pdb_path, Some(base_addr)).expect("Failed to parse PDB");

    let mut checked = 0;

    for proc in &pdb.procedures {
        if let Some(start_addr) = proc.address {
            let end_addr = start_addr.saturating_add(proc.len);

            // Test that start address is contained
            assert!(
                proc.contains_address(start_addr),
                "Procedure '{}': should contain its start address {:#x}",
                proc.name,
                start_addr
            );

            // Test that one byte before start is NOT contained
            if start_addr > 0 {
                assert!(
                    !proc.contains_address(start_addr - 1),
                    "Procedure '{}': should NOT contain address before start ({:#x})",
                    proc.name,
                    start_addr - 1
                );
            }

            // Test that end address is NOT contained (range is [start, end))
            assert!(
                !proc.contains_address(end_addr),
                "Procedure '{}': should NOT contain end address {:#x} (exclusive range)",
                proc.name,
                end_addr
            );

            // Test middle of procedure
            if proc.len > 2 {
                let mid_addr = start_addr + (proc.len / 2);
                assert!(
                    proc.contains_address(mid_addr),
                    "Procedure '{}': should contain middle address {:#x}",
                    proc.name,
                    mid_addr
                );
            }

            checked += 1;
        }
    }

    println!(
        "✓ contains_address() works correctly for {} procedures",
        checked
    );
}

#[test]
fn test_duplicate_symbol_detection() {
    let Some(pdb_path) = get_test_pdb() else {
        eprintln!("Skipping test: test PDB not found");
        return;
    };

    println!("Testing for duplicate symbols by name...");

    let pdb = ezpdb::parse_pdb(&pdb_path, None).expect("Failed to parse PDB");

    // Track symbol names and their counts
    let mut public_symbol_names: HashMap<String, Vec<usize>> = HashMap::new();
    let mut procedure_names: HashMap<String, Vec<usize>> = HashMap::new();
    let mut data_names: HashMap<String, Vec<usize>> = HashMap::new();

    // Collect public symbols
    for (idx, symbol) in pdb.public_symbols.iter().enumerate() {
        public_symbol_names
            .entry(symbol.name.clone())
            .or_insert_with(Vec::new)
            .push(idx);
    }

    // Collect procedures
    for (idx, proc) in pdb.procedures.iter().enumerate() {
        procedure_names
            .entry(proc.name.clone())
            .or_insert_with(Vec::new)
            .push(idx);
    }

    // Collect data symbols
    for (idx, data) in pdb.global_data.iter().enumerate() {
        data_names
            .entry(data.name.clone())
            .or_insert_with(Vec::new)
            .push(idx);
    }

    // Find duplicates
    let mut public_duplicates = 0;
    let mut procedure_duplicates = 0;
    let mut data_duplicates = 0;

    for (name, indices) in &public_symbol_names {
        if indices.len() > 1 {
            public_duplicates += 1;

            if public_duplicates <= 5 {
                println!(
                    "  Duplicate public symbol '{}': {} occurrences",
                    name,
                    indices.len()
                );

                // Show their locations
                for &idx in indices {
                    let sym = &pdb.public_symbols[idx];
                    println!("    - {}", sym.format_location());
                }
            }
        }
    }

    for (name, indices) in &procedure_names {
        if indices.len() > 1 {
            procedure_duplicates += 1;

            if procedure_duplicates <= 5 {
                println!(
                    "  Duplicate procedure '{}': {} occurrences",
                    name,
                    indices.len()
                );

                for &idx in indices {
                    let proc = &pdb.procedures[idx];
                    println!("    - {} (len: {} bytes)", proc.format_location(), proc.len);
                }
            }
        }
    }

    for (name, indices) in &data_names {
        if indices.len() > 1 {
            data_duplicates += 1;

            if data_duplicates <= 5 {
                println!(
                    "  Duplicate data symbol '{}': {} occurrences",
                    name,
                    indices.len()
                );

                for &idx in indices {
                    let data = &pdb.global_data[idx];
                    println!("    - {}", data.format_location());
                }
            }
        }
    }

    println!("\n=== DUPLICATE SYMBOL STATISTICS ===");
    println!(
        "Public symbols: {} duplicates out of {} unique names ({} total symbols)",
        public_duplicates,
        public_symbol_names.len(),
        pdb.public_symbols.len()
    );
    println!(
        "Procedures: {} duplicates out of {} unique names ({} total procedures)",
        procedure_duplicates,
        procedure_names.len(),
        pdb.procedures.len()
    );
    println!(
        "Data symbols: {} duplicates out of {} unique names ({} total data)",
        data_duplicates,
        data_names.len(),
        pdb.global_data.len()
    );

    // This test is informational - duplicates are allowed in PDB files
    // (e.g., symbols from different compilation units, or weak symbols)
    println!("\n✓ Duplicate detection complete (duplicates are allowed and tracked correctly)");
}

#[test]
fn test_duplicate_symbols_at_different_locations() {
    let Some(pdb_path) = get_test_pdb() else {
        eprintln!("Skipping test: test PDB not found");
        return;
    };

    println!("Testing that duplicate symbol names can have different locations...");

    let pdb = ezpdb::parse_pdb(&pdb_path, None).expect("Failed to parse PDB");

    let mut duplicates_with_different_locations = 0;
    let mut duplicates_with_same_location = 0;

    // Group public symbols by name
    let mut name_to_symbols: HashMap<String, Vec<&ezpdb::PublicSymbol>> = HashMap::new();
    for symbol in &pdb.public_symbols {
        name_to_symbols
            .entry(symbol.name.clone())
            .or_insert_with(Vec::new)
            .push(symbol);
    }

    // Check duplicates
    for (name, symbols) in &name_to_symbols {
        if symbols.len() > 1 {
            // Check if they have different locations
            let locations: HashSet<_> = symbols
                .iter()
                .filter_map(|s| s.section_and_offset())
                .collect();

            if locations.len() > 1 {
                duplicates_with_different_locations += 1;

                if duplicates_with_different_locations <= 3 {
                    println!(
                        "  Symbol '{}' appears at {} different locations",
                        name,
                        locations.len()
                    );
                }
            } else if locations.len() == 1 {
                duplicates_with_same_location += 1;

                if duplicates_with_same_location <= 3 {
                    println!(
                        "  Symbol '{}' appears {} times at the same location",
                        name,
                        symbols.len()
                    );
                }
            }
        }
    }

    println!("\n=== DUPLICATE LOCATION STATISTICS ===");
    println!(
        "Duplicates at different locations: {}",
        duplicates_with_different_locations
    );
    println!(
        "Duplicates at same location: {}",
        duplicates_with_same_location
    );

    println!("\n✓ Duplicate symbol location analysis complete");
}

#[test]
fn test_symbol_name_uniqueness_across_types() {
    let Some(pdb_path) = get_test_pdb() else {
        eprintln!("Skipping test: test PDB not found");
        return;
    };

    println!("Testing if symbol names appear across different symbol types...");

    let pdb = ezpdb::parse_pdb(&pdb_path, None).expect("Failed to parse PDB");

    let public_names: HashSet<_> = pdb.public_symbols.iter().map(|s| &s.name).collect();
    let procedure_names: HashSet<_> = pdb.procedures.iter().map(|p| &p.name).collect();
    let data_names: HashSet<_> = pdb.global_data.iter().map(|d| &d.name).collect();

    // Find names that appear in multiple categories
    let public_and_proc: HashSet<_> = public_names.intersection(&procedure_names).collect();
    let public_and_data: HashSet<_> = public_names.intersection(&data_names).collect();
    let proc_and_data: HashSet<_> = procedure_names.intersection(&data_names).collect();

    println!("\n=== CROSS-TYPE SYMBOL NAME STATISTICS ===");
    println!(
        "Names in both PublicSymbol and Procedure: {}",
        public_and_proc.len()
    );
    println!(
        "Names in both PublicSymbol and Data: {}",
        public_and_data.len()
    );
    println!("Names in both Procedure and Data: {}", proc_and_data.len());

    // Show some examples
    if !public_and_proc.is_empty() {
        println!("\nSample names in both PublicSymbol and Procedure:");
        for name in public_and_proc.iter().take(5) {
            println!("  - {}", name);
        }
    }

    println!(
        "\n✓ Cross-type symbol name analysis complete ({} public, {} procedures, {} data)",
        public_names.len(),
        procedure_names.len(),
        data_names.len()
    );
}
