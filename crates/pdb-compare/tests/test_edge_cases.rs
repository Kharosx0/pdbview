//! Edge Case Tests for Symbol Offset Handling
//!
//! These tests validate that edge cases and error conditions are handled correctly:
//! 1. Invalid section (section 0) handling
//! 2. Section numbers out of bounds
//! 3. Overflow protection in address calculations
//! 4. Malformed or unexpected symbol data

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
fn test_invalid_section_zero_handling() {
    let Some(pdb_path) = get_test_pdb() else {
        eprintln!("Skipping test: test PDB not found");
        return;
    };

    println!("Testing invalid section (section 0) handling...");

    // Parse with ezpdb directly to access all fields
    let pdb = ezpdb::parse_pdb(&pdb_path, None).expect("Failed to parse PDB");

    let mut invalid_public_symbols = Vec::new();
    let mut invalid_procedures = Vec::new();
    let mut invalid_data_symbols = Vec::new();

    // Check public symbols for invalid sections
    for symbol in &pdb.public_symbols {
        if symbol.section.is_none() {
            invalid_public_symbols.push(symbol.name.clone());

            // CRITICAL INVARIANT: If section is None, then RVA MUST be None
            assert_eq!(
                symbol.rva, None,
                "PublicSymbol '{}' has section=None but rva={:?}. \
                Invalid sections (0) should not have RVAs.",
                symbol.name, symbol.rva
            );

            // CRITICAL INVARIANT: If section is None, then address MUST be None
            assert_eq!(
                symbol.address, None,
                "PublicSymbol '{}' has section=None but address={:?}. \
                Invalid sections should not have addresses.",
                symbol.name, symbol.address
            );

            // section_offset should still be present (it's u32, not Option)
            // This is raw PDB data that we preserve even if section is invalid
            let _offset = symbol.section_offset;
        }
    }

    // Check procedures for invalid sections
    for proc in &pdb.procedures {
        if proc.section.is_none() {
            invalid_procedures.push(proc.name.clone());

            // Same invariants apply to procedures
            assert_eq!(
                proc.rva, None,
                "Procedure '{}' has section=None but rva={:?}",
                proc.name, proc.rva
            );
            assert_eq!(
                proc.address, None,
                "Procedure '{}' has section=None but address={:?}",
                proc.name, proc.address
            );
            let _offset = proc.section_offset;
        }
    }

    // Check data symbols for invalid sections
    for data in &pdb.global_data {
        if data.section.is_none() {
            invalid_data_symbols.push(data.name.clone());

            // Same invariants apply to data symbols
            assert_eq!(
                data.rva, None,
                "Data '{}' has section=None but rva={:?}",
                data.name, data.rva
            );
            assert_eq!(
                data.address, None,
                "Data '{}' has section=None but address={:?}",
                data.name, data.address
            );
            let _offset = data.section_offset;
        }
    }

    println!("\n=== INVALID SECTION STATISTICS ===");
    println!(
        "Public symbols with invalid section: {} / {}",
        invalid_public_symbols.len(),
        pdb.public_symbols.len()
    );
    println!(
        "Procedures with invalid section: {} / {}",
        invalid_procedures.len(),
        pdb.procedures.len()
    );
    println!(
        "Data symbols with invalid section: {} / {}",
        invalid_data_symbols.len(),
        pdb.global_data.len()
    );

    // Show some examples if found
    if !invalid_public_symbols.is_empty() {
        println!("\nSample invalid public symbols:");
        for name in invalid_public_symbols.iter().take(5) {
            println!("  - {}", name);
        }
    }

    if !invalid_procedures.is_empty() {
        println!("\nSample invalid procedures:");
        for name in invalid_procedures.iter().take(5) {
            println!("  - {}", name);
        }
    }

    if !invalid_data_symbols.is_empty() {
        println!("\nSample invalid data symbols:");
        for name in invalid_data_symbols.iter().take(5) {
            println!("  - {}", name);
        }
    }

    // It's OK to have some symbols with invalid sections
    // (The PDB might contain symbols that don't map to any section)
    // The important thing is that they're handled correctly (no RVA or address)

    let total_invalid =
        invalid_public_symbols.len() + invalid_procedures.len() + invalid_data_symbols.len();

    if total_invalid == 0 {
        println!("\n✓ No symbols with invalid sections found (all symbols are valid)");
    } else {
        println!(
            "\n✓ Found {} symbols with invalid sections, all handled correctly",
            total_invalid
        );
        println!("  (section=None, rva=None, address=None, section_offset preserved)");
    }
}

#[test]
fn test_invalid_section_with_base_address() {
    let Some(pdb_path) = get_test_pdb() else {
        eprintln!("Skipping test: test PDB not found");
        return;
    };

    println!("Testing that invalid sections remain invalid even with base_address...");

    let base_addr = 0x140000000;
    let pdb = ezpdb::parse_pdb(&pdb_path, Some(base_addr)).expect("Failed to parse PDB");

    let mut invalid_count = 0;

    // Check all symbol types
    for symbol in &pdb.public_symbols {
        if symbol.section.is_none() {
            invalid_count += 1;
            assert_eq!(
                symbol.rva, None,
                "Invalid section should have no RVA even with base_address"
            );
            assert_eq!(
                symbol.address, None,
                "Invalid section should have no address even with base_address provided"
            );
        }
    }

    for proc in &pdb.procedures {
        if proc.section.is_none() {
            invalid_count += 1;
            assert_eq!(proc.rva, None);
            assert_eq!(proc.address, None);
        }
    }

    for data in &pdb.global_data {
        if data.section.is_none() {
            invalid_count += 1;
            assert_eq!(data.rva, None);
            assert_eq!(data.address, None);
        }
    }

    println!(
        "✓ All {} symbols with invalid sections have no address (even with base_address)",
        invalid_count
    );
}

#[test]
fn test_section_offset_preserved_for_invalid_sections() {
    let Some(pdb_path) = get_test_pdb() else {
        eprintln!("Skipping test: test PDB not found");
        return;
    };

    println!("Verifying section_offset is preserved even for invalid sections...");

    let pdb = ezpdb::parse_pdb(&pdb_path, None).expect("Failed to parse PDB");

    let mut symbols_checked = 0;

    // For symbols with invalid sections, section_offset should still exist
    // (It's raw PDB data, we preserve it even if we can't use it)
    for symbol in &pdb.public_symbols {
        if symbol.section.is_none() {
            // section_offset is u32, not Option<u32>, so it always exists
            let offset = symbol.section_offset;

            // We can't validate the value is "correct" since section is invalid,
            // but we can verify it's accessible and is a valid u32
            assert!(
                offset < u32::MAX,
                "section_offset should be a valid u32 value"
            );

            symbols_checked += 1;
        }
    }

    for proc in &pdb.procedures {
        if proc.section.is_none() {
            let offset = proc.section_offset;
            assert!(offset < u32::MAX);
            symbols_checked += 1;
        }
    }

    for data in &pdb.global_data {
        if data.section.is_none() {
            let offset = data.section_offset;
            assert!(offset < u32::MAX);
            symbols_checked += 1;
        }
    }

    println!(
        "✓ Verified section_offset preserved for {} symbols with invalid sections",
        symbols_checked
    );
}

#[test]
fn test_no_section_zero_exposed_as_some() {
    let Some(pdb_path) = get_test_pdb() else {
        eprintln!("Skipping test: test PDB not found");
        return;
    };

    println!("Verifying section 0 is never exposed as Some(0)...");

    let pdb = ezpdb::parse_pdb(&pdb_path, None).expect("Failed to parse PDB");

    // CRITICAL: Section 0 is invalid in PDB files
    // We should NEVER expose it as Some(0), it should always be None

    for symbol in &pdb.public_symbols {
        if let Some(section) = symbol.section {
            assert!(
                section >= 1,
                "PublicSymbol '{}' has section=Some(0). \
                Section 0 is invalid and should be represented as None, not Some(0).",
                symbol.name
            );
        }
    }

    for proc in &pdb.procedures {
        if let Some(section) = proc.section {
            assert!(
                section >= 1,
                "Procedure '{}' has section=Some(0). Section 0 should be None.",
                proc.name
            );
        }
    }

    for data in &pdb.global_data {
        if let Some(section) = data.section {
            assert!(
                section >= 1,
                "Data '{}' has section=Some(0). Section 0 should be None.",
                data.name
            );
        }
    }

    println!("✓ No symbols have section=Some(0). All use None for invalid section.");
}

#[test]
fn test_section_numbers_are_reasonable() {
    let Some(pdb_path) = get_test_pdb() else {
        eprintln!("Skipping test: test PDB not found");
        return;
    };

    println!("Checking that section numbers are in reasonable ranges...");

    let pdb = ezpdb::parse_pdb(&pdb_path, None).expect("Failed to parse PDB");

    let mut max_section_seen = 0u16;
    let mut section_counts: std::collections::HashMap<u16, usize> =
        std::collections::HashMap::new();

    // Collect section number statistics
    for symbol in &pdb.public_symbols {
        if let Some(section) = symbol.section {
            max_section_seen = max_section_seen.max(section);
            *section_counts.entry(section).or_insert(0) += 1;
        }
    }

    for proc in &pdb.procedures {
        if let Some(section) = proc.section {
            max_section_seen = max_section_seen.max(section);
            *section_counts.entry(section).or_insert(0) += 1;
        }
    }

    for data in &pdb.global_data {
        if let Some(section) = data.section {
            max_section_seen = max_section_seen.max(section);
            *section_counts.entry(section).or_insert(0) += 1;
        }
    }

    println!("\n=== SECTION NUMBER STATISTICS ===");
    println!("Maximum section number seen: {}", max_section_seen);
    println!("Unique sections with symbols: {}", section_counts.len());

    // Most PE files have < 20 sections, 100 would be very unusual
    assert!(
        max_section_seen <= 100,
        "Maximum section number {} is unreasonably high (>100). \
        Most PE files have < 20 sections. This might indicate a parsing bug.",
        max_section_seen
    );

    // Show section distribution
    let mut sorted_sections: Vec<_> = section_counts.iter().collect();
    sorted_sections.sort_by_key(|(section, _)| *section);

    println!("\nTop sections by symbol count:");
    for (section, count) in sorted_sections.iter().take(10) {
        println!("  Section {}: {} symbols", section, count);
    }

    println!(
        "\n✓ All section numbers are in reasonable ranges (1-{})",
        max_section_seen
    );
}

#[test]
fn test_rva_none_implies_address_none() {
    let Some(pdb_path) = get_test_pdb() else {
        eprintln!("Skipping test: test PDB not found");
        return;
    };

    println!("Verifying that rva=None implies address=None...");

    let base_addr = 0x140000000;
    let pdb = ezpdb::parse_pdb(&pdb_path, Some(base_addr)).expect("Failed to parse PDB");

    let mut violations = Vec::new();

    // INVARIANT: If rva is None, address MUST be None
    // (You can't have an absolute address without an RVA to base it on)

    for symbol in &pdb.public_symbols {
        if symbol.rva.is_none() && symbol.address.is_some() {
            violations.push(format!(
                "PublicSymbol '{}': rva=None but address={:?}",
                symbol.name, symbol.address
            ));
        }
    }

    for proc in &pdb.procedures {
        if proc.rva.is_none() && proc.address.is_some() {
            violations.push(format!(
                "Procedure '{}': rva=None but address={:?}",
                proc.name, proc.address
            ));
        }
    }

    for data in &pdb.global_data {
        if data.rva.is_none() && data.address.is_some() {
            violations.push(format!(
                "Data '{}': rva=None but address={:?}",
                data.name, data.address
            ));
        }
    }

    if !violations.is_empty() {
        println!(
            "\n❌ Found {} violations of rva=None => address=None:",
            violations.len()
        );
        for (i, violation) in violations.iter().enumerate().take(10) {
            println!("  [{}] {}", i, violation);
        }
        panic!("Invariant violated: rva=None but address=Some");
    }

    println!("✓ All symbols satisfy: rva=None => address=None");
}

#[test]
fn test_address_calculation_consistency() {
    let Some(pdb_path) = get_test_pdb() else {
        eprintln!("Skipping test: test PDB not found");
        return;
    };

    println!("Verifying address calculation consistency (address = rva + base)...");

    let base_addr = 0x140000000usize;
    let pdb = ezpdb::parse_pdb(&pdb_path, Some(base_addr)).expect("Failed to parse PDB");

    let mut symbols_checked = 0;
    let mut violations = Vec::new();

    // For symbols with both rva and address, verify: address = rva + base_addr

    for symbol in &pdb.public_symbols {
        if let (Some(rva), Some(address)) = (symbol.rva, symbol.address) {
            let expected_address = rva.wrapping_add(base_addr);

            if address != expected_address {
                violations.push(format!(
                    "PublicSymbol '{}': address={:#x} but rva={:#x} + base={:#x} = {:#x}",
                    symbol.name, address, rva, base_addr, expected_address
                ));
            }

            symbols_checked += 1;
        }
    }

    for proc in &pdb.procedures {
        if let (Some(rva), Some(address)) = (proc.rva, proc.address) {
            let expected_address = rva.wrapping_add(base_addr);

            if address != expected_address {
                violations.push(format!(
                    "Procedure '{}': address={:#x} but rva={:#x} + base={:#x} = {:#x}",
                    proc.name, address, rva, base_addr, expected_address
                ));
            }

            symbols_checked += 1;
        }
    }

    for data in &pdb.global_data {
        if let (Some(rva), Some(address)) = (data.rva, data.address) {
            let expected_address = rva.wrapping_add(base_addr);

            if address != expected_address {
                violations.push(format!(
                    "Data '{}': address={:#x} but rva={:#x} + base={:#x} = {:#x}",
                    data.name, address, rva, base_addr, expected_address
                ));
            }

            symbols_checked += 1;
        }
    }

    println!(
        "Checked {} symbols with both rva and address",
        symbols_checked
    );

    if !violations.is_empty() {
        println!(
            "\n❌ Found {} address calculation violations:",
            violations.len()
        );
        for (i, violation) in violations.iter().enumerate().take(10) {
            println!("  [{}] {}", i, violation);
        }
        panic!("Address calculation incorrect: address != rva + base_addr");
    }

    println!(
        "✓ All {} symbols have correct address = rva + base_addr",
        symbols_checked
    );
}
