//! Comprehensive tests for section and section_offset fields
//!
//! These tests validate that the new section and section_offset fields are:
//! 1. Correctly populated from PDB data
//! 2. Consistent with RVA calculations
//! 3. Properly handled for edge cases (invalid sections)

use ezpdb::parse_pdb;
use std::path::PathBuf;

/// Helper to get a test PDB path
fn get_test_pdb() -> Option<PathBuf> {
    let pdb_path = PathBuf::from("../pdb-compare/cache_test_pdbs/ntkrnlmp.pdb");
    if pdb_path.exists() {
        Some(pdb_path)
    } else {
        None
    }
}

#[test]
fn test_public_symbol_section_fields() {
    let Some(pdb_path) = get_test_pdb() else {
        eprintln!("Skipping test: test PDB not found");
        return;
    };

    println!("Testing PublicSymbol section and section_offset fields...");

    let pdb = parse_pdb(&pdb_path, None).expect("Failed to parse PDB");

    let mut symbols_with_section = 0;
    let mut symbols_without_section = 0;
    let mut symbols_with_rva = 0;
    let mut symbols_without_rva = 0;
    let mut inconsistent_symbols = Vec::new();

    for symbol in &pdb.public_symbols {
        // Count symbols by field presence
        if symbol.section.is_some() {
            symbols_with_section += 1;
        } else {
            symbols_without_section += 1;
        }

        if symbol.rva.is_some() {
            symbols_with_rva += 1;
        } else {
            symbols_without_rva += 1;
        }

        // INVARIANT: If section is None, then RVA should be None
        // (Invalid section => no valid RVA)
        if symbol.section.is_none() && symbol.rva.is_some() {
            inconsistent_symbols.push(format!(
                "Symbol '{}': section is None but RVA is Some({})",
                symbol.name,
                symbol.rva.unwrap()
            ));
        }

        // INVARIANT: If section is Some, then section_offset should be meaningful
        // (We can't validate the exact value, but we can check it exists)
        if symbol.section.is_some() {
            // section_offset is always present (u32), so just verify section is in valid range
            let section_num = symbol.section.unwrap();
            assert!(
                section_num >= 1,
                "Symbol '{}': section number should be >= 1 (1-based), got {}",
                symbol.name,
                section_num
            );
        }

        // INVARIANT: section_offset should always be present (it's u32, not Option)
        // This is by design - we preserve the raw PDB data even if section is invalid
        let _offset = symbol.section_offset; // Just verify it compiles and exists
    }

    println!("PublicSymbol statistics:");
    println!("  Total symbols: {}", pdb.public_symbols.len());
    println!("  Symbols with valid section: {}", symbols_with_section);
    println!(
        "  Symbols with invalid section: {}",
        symbols_without_section
    );
    println!("  Symbols with RVA: {}", symbols_with_rva);
    println!("  Symbols without RVA: {}", symbols_without_rva);

    // Most symbols should have valid sections
    assert!(
        symbols_with_section > 0,
        "Expected at least some symbols with valid sections"
    );

    // Most symbols with valid sections should have RVAs
    let rva_percentage = (symbols_with_rva as f64 / symbols_with_section as f64) * 100.0;
    println!("  RVA availability: {:.1}%", rva_percentage);
    assert!(
        rva_percentage > 90.0,
        "Expected >90% of symbols with valid sections to have RVAs, got {:.1}%",
        rva_percentage
    );

    // Verify no inconsistencies
    if !inconsistent_symbols.is_empty() {
        println!(
            "\n❌ Found {} inconsistent symbols:",
            inconsistent_symbols.len()
        );
        for (i, msg) in inconsistent_symbols.iter().enumerate().take(10) {
            println!("  [{}] {}", i, msg);
        }
        panic!("Found symbols with inconsistent section/RVA fields");
    }

    println!("✓ All PublicSymbol section fields are consistent");
}

#[test]
fn test_procedure_section_fields() {
    let Some(pdb_path) = get_test_pdb() else {
        eprintln!("Skipping test: test PDB not found");
        return;
    };

    println!("Testing Procedure section and section_offset fields...");

    let pdb = parse_pdb(&pdb_path, None).expect("Failed to parse PDB");

    let mut procedures_with_section = 0;
    let mut procedures_without_section = 0;
    let mut procedures_with_rva = 0;
    let mut procedures_without_rva = 0;
    let mut inconsistent_procedures = Vec::new();

    for proc in &pdb.procedures {
        // Count procedures by field presence
        if proc.section.is_some() {
            procedures_with_section += 1;
        } else {
            procedures_without_section += 1;
        }

        if proc.rva.is_some() {
            procedures_with_rva += 1;
        } else {
            procedures_without_rva += 1;
        }

        // INVARIANT: If section is None, then RVA should be None
        if proc.section.is_none() && proc.rva.is_some() {
            inconsistent_procedures.push(format!(
                "Procedure '{}': section is None but RVA is Some({})",
                proc.name,
                proc.rva.unwrap()
            ));
        }

        // INVARIANT: If section is Some, verify section number is valid (1-based)
        if let Some(section_num) = proc.section {
            assert!(
                section_num >= 1,
                "Procedure '{}': section number should be >= 1 (1-based), got {}",
                proc.name,
                section_num
            );
        }

        // Verify section_offset exists (u32, always present)
        let _offset = proc.section_offset;
    }

    println!("Procedure statistics:");
    println!("  Total procedures: {}", pdb.procedures.len());
    println!(
        "  Procedures with valid section: {}",
        procedures_with_section
    );
    println!(
        "  Procedures with invalid section: {}",
        procedures_without_section
    );
    println!("  Procedures with RVA: {}", procedures_with_rva);
    println!("  Procedures without RVA: {}", procedures_without_rva);

    // Most procedures should have valid sections (they're executable code)
    assert!(
        procedures_with_section > 0,
        "Expected at least some procedures with valid sections"
    );

    // Procedures should have very high RVA availability (they're in .text section)
    let rva_percentage = (procedures_with_rva as f64 / procedures_with_section as f64) * 100.0;
    println!("  RVA availability: {:.1}%", rva_percentage);
    assert!(
        rva_percentage > 95.0,
        "Expected >95% of procedures with valid sections to have RVAs, got {:.1}%",
        rva_percentage
    );

    // Verify no inconsistencies
    if !inconsistent_procedures.is_empty() {
        println!(
            "\n❌ Found {} inconsistent procedures:",
            inconsistent_procedures.len()
        );
        for (i, msg) in inconsistent_procedures.iter().enumerate().take(10) {
            println!("  [{}] {}", i, msg);
        }
        panic!("Found procedures with inconsistent section/RVA fields");
    }

    println!("✓ All Procedure section fields are consistent");
}

#[test]
fn test_data_symbol_section_fields() {
    let Some(pdb_path) = get_test_pdb() else {
        eprintln!("Skipping test: test PDB not found");
        return;
    };

    println!("Testing Data symbol section and section_offset fields...");

    let pdb = parse_pdb(&pdb_path, None).expect("Failed to parse PDB");

    let mut data_with_section = 0;
    let mut data_without_section = 0;
    let mut data_with_rva = 0;
    let mut data_without_rva = 0;
    let mut inconsistent_data = Vec::new();

    for data in &pdb.global_data {
        // Count data symbols by field presence
        if data.section.is_some() {
            data_with_section += 1;
        } else {
            data_without_section += 1;
        }

        if data.rva.is_some() {
            data_with_rva += 1;
        } else {
            data_without_rva += 1;
        }

        // INVARIANT: If section is None, then RVA should be None
        if data.section.is_none() && data.rva.is_some() {
            inconsistent_data.push(format!(
                "Data '{}': section is None but RVA is Some({})",
                data.name,
                data.rva.unwrap()
            ));
        }

        // INVARIANT: If section is Some, verify section number is valid (1-based)
        if let Some(section_num) = data.section {
            assert!(
                section_num >= 1,
                "Data '{}': section number should be >= 1 (1-based), got {}",
                data.name,
                section_num
            );
        }

        // Verify section_offset exists (u32, always present)
        let _offset = data.section_offset;
    }

    println!("Data symbol statistics:");
    println!("  Total data symbols: {}", pdb.global_data.len());
    println!("  Data with valid section: {}", data_with_section);
    println!("  Data with invalid section: {}", data_without_section);
    println!("  Data with RVA: {}", data_with_rva);
    println!("  Data without RVA: {}", data_without_rva);

    // Most data symbols should have valid sections
    assert!(
        data_with_section > 0,
        "Expected at least some data symbols with valid sections"
    );

    // Data symbols should have high RVA availability
    let rva_percentage = (data_with_rva as f64 / data_with_section as f64) * 100.0;
    println!("  RVA availability: {:.1}%", rva_percentage);
    assert!(
        rva_percentage > 90.0,
        "Expected >90% of data symbols with valid sections to have RVAs, got {:.1}%",
        rva_percentage
    );

    // Verify no inconsistencies
    if !inconsistent_data.is_empty() {
        println!(
            "\n❌ Found {} inconsistent data symbols:",
            inconsistent_data.len()
        );
        for (i, msg) in inconsistent_data.iter().enumerate().take(10) {
            println!("  [{}] {}", i, msg);
        }
        panic!("Found data symbols with inconsistent section/RVA fields");
    }

    println!("✓ All Data symbol section fields are consistent");
}

#[test]
fn test_section_offset_rva_relationship() {
    let Some(pdb_path) = get_test_pdb() else {
        eprintln!("Skipping test: test PDB not found");
        return;
    };

    println!("Testing section_offset and RVA relationship...");

    let pdb = parse_pdb(&pdb_path, None).expect("Failed to parse PDB");

    // Test a few public symbols to verify the relationship
    let mut samples_tested = 0;
    const SAMPLES_TO_TEST: usize = 10;

    for symbol in pdb.public_symbols.iter().take(SAMPLES_TO_TEST) {
        if let (Some(section), Some(rva)) = (symbol.section, symbol.rva) {
            println!(
                "Symbol '{}': section={}, section_offset=0x{:x}, rva=0x{:x}",
                symbol.name, section, symbol.section_offset, rva
            );

            // We can't fully validate the formula (we'd need section headers),
            // but we can do sanity checks:

            // 1. Section number should be reasonable (typically 1-10 for most PEs)
            assert!(
                section <= 100,
                "Section number {} seems unreasonably high",
                section
            );

            // 2. RVA should be >= section_offset (since section has a base address)
            // Actually, this may not always be true if section has high virtual_address
            // So we just check that both values are reasonable
            assert!(
                rva < 0x100000000,
                "RVA 0x{:x} seems unreasonably large",
                rva
            );

            samples_tested += 1;
        }
    }

    println!(
        "✓ Tested {} symbol section/offset/RVA relationships",
        samples_tested
    );
    assert!(
        samples_tested > 0,
        "Expected to test at least some symbols with complete information"
    );
}

#[test]
fn test_section_fields_with_base_address() {
    let Some(pdb_path) = get_test_pdb() else {
        eprintln!("Skipping test: test PDB not found");
        return;
    };

    println!("Testing that section/section_offset are independent of base_address...");

    let base_addr = 0x140000000;
    let pdb_no_base = parse_pdb(&pdb_path, None).expect("Failed to parse PDB without base");
    let pdb_with_base =
        parse_pdb(&pdb_path, Some(base_addr)).expect("Failed to parse PDB with base");

    // Find a symbol that exists in both
    let symbol_no_base = pdb_no_base
        .public_symbols
        .iter()
        .find(|s| s.section.is_some() && s.rva.is_some())
        .expect("Should have at least one symbol with section and RVA");

    let symbol_with_base = pdb_with_base
        .public_symbols
        .iter()
        .find(|s| s.name == symbol_no_base.name)
        .expect("Symbol should exist in both parses");

    // CRITICAL: section and section_offset should be IDENTICAL regardless of base_address
    assert_eq!(
        symbol_no_base.section, symbol_with_base.section,
        "section should be independent of base_address"
    );
    assert_eq!(
        symbol_no_base.section_offset, symbol_with_base.section_offset,
        "section_offset should be independent of base_address"
    );
    assert_eq!(
        symbol_no_base.rva, symbol_with_base.rva,
        "RVA should be independent of base_address"
    );

    // The address field SHOULD be different
    assert_eq!(
        symbol_no_base.address, None,
        "address should be None when no base_address provided"
    );
    assert_eq!(
        symbol_with_base.address,
        Some(symbol_no_base.rva.unwrap() + base_addr),
        "address should be rva + base_address when base_address provided"
    );

    println!("✓ section and section_offset are independent of base_address");
    println!("✓ address field correctly uses base_address");
}

#[test]
fn test_all_symbols_have_section_offset() {
    let Some(pdb_path) = get_test_pdb() else {
        eprintln!("Skipping test: test PDB not found");
        return;
    };

    println!("Verifying that all symbols have section_offset field...");

    let pdb = parse_pdb(&pdb_path, None).expect("Failed to parse PDB");

    // section_offset is u32, not Option<u32>, so it should always be present
    // This test just verifies the type is correct and accessible

    for symbol in &pdb.public_symbols {
        let _: u32 = symbol.section_offset; // Type check
    }

    for proc in &pdb.procedures {
        let _: u32 = proc.section_offset; // Type check
    }

    for data in &pdb.global_data {
        let _: u32 = data.section_offset; // Type check
    }

    println!("✓ All symbols have section_offset field (u32, always present)");
}
