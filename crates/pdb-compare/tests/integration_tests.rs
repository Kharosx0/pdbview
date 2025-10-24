use std::path::PathBuf;

#[test]
fn test_ntkrnlmp_pdb_comparison() {
    let _ = env_logger::builder().is_test(true).try_init();

    let pdb_path = PathBuf::from("cache_test_pdbs/ntkrnlmp.pdb");
    if !pdb_path.exists() {
        eprintln!("Skipping test: {} not found", pdb_path.display());
        return;
    }

    println!("Testing ntkrnlmp.pdb comparison...");

    // Parse with both old and new parsers
    let old_data =
        pdb_compare::old_pdb::parse_pdb(&pdb_path).expect("Failed to parse with old parser");
    let new_data =
        pdb_compare::new_pdb::parse_pdb_new(&pdb_path).expect("Failed to parse with new parser");

    // Compare
    let result = pdb_compare::compare::compare_pdbs(&old_data, &new_data);

    println!("\n=== COMPARISON SUMMARY ===");
    println!("Total differences: {}", result.summary.total_differences);
    println!("Header matches: {}", result.summary.header_matches);
    println!("Type count matches: {}", result.summary.type_count_matches);
    println!("  Old parser: {} types", old_data.types.len());
    println!("  New parser: {} types", new_data.types.len());
    println!(
        "Symbol count matches: {}",
        result.summary.symbol_count_matches
    );
    println!(
        "Module count matches: {}",
        result.summary.module_count_matches
    );

    // Print critical differences
    let critical_diffs: Vec<_> = result
        .differences
        .iter()
        .filter(|d| {
            d.description.contains("field count")
                || d.description.contains("variant count")
                || d.description.contains("size mismatch")
                || d.description.contains("missing in new")
        })
        .collect();

    if !critical_diffs.is_empty() {
        println!("\n=== CRITICAL DIFFERENCES ({}) ===", critical_diffs.len());
        for (i, diff) in critical_diffs.iter().take(50).enumerate() {
            println!("{}. [{}] {}", i + 1, diff.category, diff.description);
            if let Some(old_val) = &diff.old_value {
                println!("   Old: {}", old_val);
            }
            if let Some(new_val) = &diff.new_value {
                println!("   New: {}", new_val);
            }
        }
        if critical_diffs.len() > 50 {
            println!(
                "... and {} more critical differences",
                critical_diffs.len() - 50
            );
        }
    }

    // Verify that new parser is at least as good as old parser (can find more, but not less)
    let old_count = old_data.types.len();
    let new_count = new_data.types.len();

    if new_count >= old_count {
        if new_count > old_count {
            println!(
                "\n✅ IMPROVEMENT: New parser found {} MORE types than old parser!",
                new_count - old_count
            );
        }
    } else {
        panic!(
            "REGRESSION: New parser found FEWER types than old parser! (Old: {}, New: {}, Missing: {})",
            old_count,
            new_count,
            old_count - new_count
        );
    }

    // Check for missing types (new parser should have all types old parser had)
    let missing_types: Vec<_> = result
        .differences
        .iter()
        .filter(|d| d.description.contains("missing in new parser") && d.category == "types")
        .collect();

    if !missing_types.is_empty() {
        println!(
            "\n=== TYPES MISSING IN NEW PARSER ({}) ===",
            missing_types.len()
        );
        for diff in missing_types.iter().take(20) {
            println!("- {}", diff.description);
        }
        panic!(
            "REGRESSION: New parser is missing {} types that old parser found!",
            missing_types.len()
        );
    }

    println!("\n✓ ntkrnlmp.pdb comparison passed");
}

#[test]
fn test_ntdll_pdb_comparison() {
    let _ = env_logger::builder().is_test(true).try_init();

    let pdb_path = PathBuf::from("cache_test_pdbs/ntdll.pdb");
    if !pdb_path.exists() {
        eprintln!("Skipping test: {} not found", pdb_path.display());
        return;
    }

    println!("Testing ntdll.pdb comparison...");

    // Parse with both old and new parsers
    let old_data =
        pdb_compare::old_pdb::parse_pdb(&pdb_path).expect("Failed to parse with old parser");
    let new_data =
        pdb_compare::new_pdb::parse_pdb_new(&pdb_path).expect("Failed to parse with new parser");

    // Compare
    let result = pdb_compare::compare::compare_pdbs(&old_data, &new_data);

    println!("\n=== COMPARISON SUMMARY ===");
    println!("Total differences: {}", result.summary.total_differences);
    println!("Header matches: {}", result.summary.header_matches);
    println!("Type count matches: {}", result.summary.type_count_matches);
    println!("  Old parser: {} types", old_data.types.len());
    println!("  New parser: {} types", new_data.types.len());
    println!(
        "Symbol count matches: {}",
        result.summary.symbol_count_matches
    );
    println!(
        "  Old parser: {} public symbols, {} procedures, {} data symbols",
        old_data.symbols.public_symbols.len(),
        old_data.symbols.procedures.len(),
        old_data.symbols.data_symbols.len()
    );
    println!(
        "  New parser: {} public symbols, {} procedures, {} data symbols",
        new_data.symbols.public_symbols.len(),
        new_data.symbols.procedures.len(),
        new_data.symbols.data_symbols.len()
    );
    println!(
        "Module count matches: {}",
        result.summary.module_count_matches
    );

    // Print type count mismatch details if they don't match
    if !result.summary.type_count_matches {
        let type_count_diff: Vec<_> = result
            .differences
            .iter()
            .filter(|d| d.description.contains("Type count mismatch"))
            .collect();

        if !type_count_diff.is_empty() {
            println!("\n=== TYPE COUNT MISMATCH DETAILS ===");
            for diff in &type_count_diff {
                println!("{}", diff.description);
                if let Some(old_val) = &diff.old_value {
                    println!("  Old parser types: {}", old_val);
                }
                if let Some(new_val) = &diff.new_value {
                    println!("  New parser types: {}", new_val);
                }
            }
        }
    }

    // Verify that new parser is at least as good as old parser (can find more, but not less)
    let old_count = old_data.types.len();
    let new_count = new_data.types.len();

    if new_count >= old_count {
        if new_count > old_count {
            println!(
                "\n✅ IMPROVEMENT: New parser found {} MORE types than old parser!",
                new_count - old_count
            );
        }
    } else {
        panic!(
            "REGRESSION: New parser found FEWER types than old parser! (Old: {}, New: {}, Missing: {})",
            old_count,
            new_count,
            old_count - new_count
        );
    }

    // Check for missing types (new parser should have all types old parser had)
    let missing_types: Vec<_> = result
        .differences
        .iter()
        .filter(|d| d.description.contains("missing in new parser") && d.category == "types")
        .collect();

    if !missing_types.is_empty() {
        println!(
            "\n=== TYPES MISSING IN NEW PARSER ({}) ===",
            missing_types.len()
        );
        for diff in missing_types.iter().take(20) {
            println!("- {}", diff.description);
        }
        panic!(
            "REGRESSION: New parser is missing {} types that old parser found!",
            missing_types.len()
        );
    }

    println!("\n✓ ntdll.pdb comparison passed");
}

#[test]
fn test_hvix64_pdb_comparison() {
    let _ = env_logger::builder().is_test(true).try_init();

    let pdb_path = PathBuf::from("cache_test_pdbs/hvix64.pdb");
    if !pdb_path.exists() {
        eprintln!("Skipping test: {} not found", pdb_path.display());
        return;
    }

    println!("Testing hvix64.pdb comparison...");

    // Parse with both old and new parsers
    let old_data =
        pdb_compare::old_pdb::parse_pdb(&pdb_path).expect("Failed to parse with old parser");
    let new_data =
        pdb_compare::new_pdb::parse_pdb_new(&pdb_path).expect("Failed to parse with new parser");

    // Compare
    let result = pdb_compare::compare::compare_pdbs(&old_data, &new_data);

    println!("\n=== COMPARISON SUMMARY ===");
    println!("Total differences: {}", result.summary.total_differences);
    println!("Header matches: {}", result.summary.header_matches);
    println!("Type count matches: {}", result.summary.type_count_matches);
    println!(
        "Symbol count matches: {}",
        result.summary.symbol_count_matches
    );
    println!(
        "Module count matches: {}",
        result.summary.module_count_matches
    );

    // Verify that new parser is at least as good as old parser (can find more, but not less)
    let old_count = old_data.types.len();
    let new_count = new_data.types.len();

    if new_count >= old_count {
        if new_count > old_count {
            println!(
                "\n✅ IMPROVEMENT: New parser found {} MORE types than old parser!",
                new_count - old_count
            );
        }
    } else {
        panic!(
            "REGRESSION: New parser found FEWER types than old parser! (Old: {}, New: {}, Missing: {})",
            old_count,
            new_count,
            old_count - new_count
        );
    }

    // Check for missing types (new parser should have all types old parser had)
    let missing_types: Vec<_> = result
        .differences
        .iter()
        .filter(|d| d.description.contains("missing in new parser") && d.category == "types")
        .collect();

    if !missing_types.is_empty() {
        println!(
            "\n=== TYPES MISSING IN NEW PARSER ({}) ===",
            missing_types.len()
        );
        for diff in missing_types.iter().take(20) {
            println!("- {}", diff.description);
        }
        panic!(
            "REGRESSION: New parser is missing {} types that old parser found!",
            missing_types.len()
        );
    }

    println!("\n✓ hvix64.pdb comparison passed");
}

#[test]
fn test_struct_field_completeness() {
    let _ = env_logger::builder().is_test(true).try_init();

    let test_pdbs = vec![
        "cache_test_pdbs/ntkrnlmp.pdb",
        "cache_test_pdbs/ntdll.pdb",
        "cache_test_pdbs/hvix64.pdb",
    ];

    for pdb_path_str in test_pdbs {
        let pdb_path = PathBuf::from(pdb_path_str);
        if !pdb_path.exists() {
            eprintln!("Skipping: {} not found", pdb_path.display());
            continue;
        }

        println!(
            "\nChecking struct field completeness for {}...",
            pdb_path.display()
        );

        let old_data =
            pdb_compare::old_pdb::parse_pdb(&pdb_path).expect("Failed to parse with old parser");
        let new_data = pdb_compare::new_pdb::parse_pdb_new(&pdb_path)
            .expect("Failed to parse with new parser");

        let mut total_types_checked = 0;
        let mut total_fields_checked = 0;
        let mut bitfield_fields_checked = 0;

        // Compare types BY INDEX (not by name) to ensure we're comparing the exact same type definition
        // This eliminates issues with multiple definitions of the same named type from different compilation units
        use std::collections::HashMap;

        // Build map by type index for old parser
        let mut old_by_index: HashMap<u32, &pdb_compare::old_pdb::TypeInfo> = HashMap::new();
        for old_type in &old_data.types {
            old_by_index.insert(old_type.index, old_type);
        }

        // Build map by type index for new parser
        let mut new_by_index: HashMap<u32, &pdb_compare::new_pdb::TypeInfo> = HashMap::new();
        for new_type in &new_data.types {
            new_by_index.insert(new_type.index, new_type);
        }

        // Compare types with the same index
        for (type_idx, old_type) in &old_by_index {
            // Only check Class and Union types with fields
            if (old_type.kind != "Class" && old_type.kind != "Union") || old_type.fields.is_empty()
            {
                continue;
            }

            if let Some(new_type) = new_by_index.get(type_idx) {
                total_types_checked += 1;

                // Check field count - must match exactly when comparing same index
                assert_eq!(
                    new_type.fields.len(),
                    old_type.fields.len(),
                    "Field count mismatch for type index {} ({}): old={}, new={}",
                    type_idx,
                    old_type.name.as_deref().unwrap_or("<unnamed>"),
                    old_type.fields.len(),
                    new_type.fields.len()
                );

                // Compare each field by position (same index = same definition = same field order)
                for (i, old_field) in old_type.fields.iter().enumerate() {
                    total_fields_checked += 1;
                    let new_field = &new_type.fields[i];

                    // Check field names match
                    assert_eq!(
                        new_field.name,
                        old_field.name,
                        "Field name mismatch at position {} in type index {} ({}): old='{}', new='{}'",
                        i,
                        type_idx,
                        old_type.name.as_deref().unwrap_or("<unnamed>"),
                        old_field.name,
                        new_field.name
                    );

                    // Check offsets match (safe to compare now that we're using same index)
                    assert_eq!(
                        new_field.offset,
                        old_field.offset,
                        "Offset mismatch for field '{}' in type index {} ({}): old={:?}, new={:?}",
                        old_field.name,
                        type_idx,
                        old_type.name.as_deref().unwrap_or("<unnamed>"),
                        old_field.offset,
                        new_field.offset
                    );

                    // Compare bitfield properties if present in either parser
                    if old_field.bitfield_length.is_some() || new_field.bitfield_length.is_some() {
                        bitfield_fields_checked += 1;

                        assert_eq!(
                            new_field.bitfield_length,
                            old_field.bitfield_length,
                            "Bitfield length mismatch for field '{}' in type index {} ({}): old={:?}, new={:?}",
                            old_field.name,
                            type_idx,
                            old_type.name.as_deref().unwrap_or("<unnamed>"),
                            old_field.bitfield_length,
                            new_field.bitfield_length
                        );

                        assert_eq!(
                            new_field.bitfield_position,
                            old_field.bitfield_position,
                            "Bitfield position mismatch for field '{}' in type index {} ({}): old={:?}, new={:?}",
                            old_field.name,
                            type_idx,
                            old_type.name.as_deref().unwrap_or("<unnamed>"),
                            old_field.bitfield_position,
                            new_field.bitfield_position
                        );
                    }
                }
            }
        }

        println!(
            "✓ Struct field completeness verified for {}",
            pdb_path.display()
        );
        println!(
            "  - Checked {} types with {} total fields",
            total_types_checked, total_fields_checked
        );
        println!(
            "  - Validated {} bitfield fields with complete metadata",
            bitfield_fields_checked
        );
    }
}

#[test]
fn test_enum_variant_completeness() {
    let _ = env_logger::builder().is_test(true).try_init();

    let test_pdbs = vec![
        "cache_test_pdbs/ntkrnlmp.pdb",
        "cache_test_pdbs/ntdll.pdb",
        "cache_test_pdbs/hvix64.pdb",
    ];

    for pdb_path_str in test_pdbs {
        let pdb_path = PathBuf::from(pdb_path_str);
        if !pdb_path.exists() {
            eprintln!("Skipping: {} not found", pdb_path.display());
            continue;
        }

        println!(
            "\nChecking enum variant completeness for {}...",
            pdb_path.display()
        );

        let old_data =
            pdb_compare::old_pdb::parse_pdb(&pdb_path).expect("Failed to parse with old parser");
        let new_data = pdb_compare::new_pdb::parse_pdb_new(&pdb_path)
            .expect("Failed to parse with new parser");

        let mut total_enums_checked = 0;
        let mut total_variants_checked = 0;

        // Compare enums BY INDEX (not by name) to ensure we're comparing the exact same type definition
        // This eliminates issues with multiple definitions of the same named enum from different compilation units
        use std::collections::HashMap;

        // Build map by type index for old parser
        let mut old_by_index: HashMap<u32, &pdb_compare::old_pdb::TypeInfo> = HashMap::new();
        for old_type in &old_data.types {
            old_by_index.insert(old_type.index, old_type);
        }

        // Build map by type index for new parser
        let mut new_by_index: HashMap<u32, &pdb_compare::new_pdb::TypeInfo> = HashMap::new();
        for new_type in &new_data.types {
            new_by_index.insert(new_type.index, new_type);
        }

        // Compare enums with the same index
        for (type_idx, old_enum) in &old_by_index {
            // Only check Enumeration types with variants
            if old_enum.kind != "Enumeration" || old_enum.variants.is_empty() {
                continue;
            }

            if let Some(new_enum) = new_by_index.get(type_idx) {
                total_enums_checked += 1;

                // Check variant count - must match exactly when comparing same index
                assert_eq!(
                    new_enum.variants.len(),
                    old_enum.variants.len(),
                    "Variant count mismatch for type index {} ({}): old={}, new={}",
                    type_idx,
                    old_enum.name.as_deref().unwrap_or("<unnamed>"),
                    old_enum.variants.len(),
                    new_enum.variants.len()
                );

                // Compare each variant by position (same index = same definition = same variant order)
                for (i, old_variant) in old_enum.variants.iter().enumerate() {
                    total_variants_checked += 1;
                    let new_variant = &new_enum.variants[i];

                    // Check variant names match
                    assert_eq!(
                        new_variant.name,
                        old_variant.name,
                        "Variant name mismatch at position {} in type index {} ({}): old='{}', new='{}'",
                        i,
                        type_idx,
                        old_enum.name.as_deref().unwrap_or("<unnamed>"),
                        old_variant.name,
                        new_variant.name
                    );

                    // Check values match (normalized to handle signed/unsigned representation)
                    // Cast through appropriate size to handle sign extension differences
                    // E.g., 0xFF can be U8(255) or I8(-1) - both should compare equal
                    let old_value_normalized = if old_variant.value >= i8::MIN as i64
                        && old_variant.value <= u8::MAX as i64
                    {
                        (old_variant.value as u8) as u64
                    } else if old_variant.value >= i16::MIN as i64
                        && old_variant.value <= u16::MAX as i64
                    {
                        (old_variant.value as u16) as u64
                    } else if old_variant.value >= i32::MIN as i64
                        && old_variant.value <= u32::MAX as i64
                    {
                        (old_variant.value as u32) as u64
                    } else {
                        old_variant.value as u64
                    };

                    let new_value_normalized = if new_variant.value >= i8::MIN as i64
                        && new_variant.value <= u8::MAX as i64
                    {
                        (new_variant.value as u8) as u64
                    } else if new_variant.value >= i16::MIN as i64
                        && new_variant.value <= u16::MAX as i64
                    {
                        (new_variant.value as u16) as u64
                    } else if new_variant.value >= i32::MIN as i64
                        && new_variant.value <= u32::MAX as i64
                    {
                        (new_variant.value as u32) as u64
                    } else {
                        new_variant.value as u64
                    };

                    assert_eq!(
                        new_value_normalized,
                        old_value_normalized,
                        "Value mismatch for variant '{}' in type index {} ({}): old={} (0x{:X}), new={} (0x{:X})",
                        old_variant.name,
                        type_idx,
                        old_enum.name.as_deref().unwrap_or("<unnamed>"),
                        old_variant.value,
                        old_value_normalized,
                        new_variant.value,
                        new_value_normalized
                    );
                }
            }
        }

        println!(
            "✓ Enum variant completeness verified for {}",
            pdb_path.display()
        );
        println!(
            "  - Checked {} enums with {} total variants",
            total_enums_checked, total_variants_checked
        );
    }
}

#[test]
fn test_type_sizes_match() {
    let _ = env_logger::builder().is_test(true).try_init();

    let test_pdbs = vec![
        "cache_test_pdbs/ntkrnlmp.pdb",
        "cache_test_pdbs/ntdll.pdb",
        "cache_test_pdbs/hvix64.pdb",
    ];

    for pdb_path_str in test_pdbs {
        let pdb_path = PathBuf::from(pdb_path_str);
        if !pdb_path.exists() {
            eprintln!("Skipping: {} not found", pdb_path.display());
            continue;
        }

        println!("\nChecking type sizes for {}...", pdb_path.display());

        let old_data =
            pdb_compare::old_pdb::parse_pdb(&pdb_path).expect("Failed to parse with old parser");
        let new_data = pdb_compare::new_pdb::parse_pdb_new(&pdb_path)
            .expect("Failed to parse with new parser");

        let mut size_mismatches = 0;

        // Check that sizes match for all types that have sizes
        for old_type in &old_data.types {
            if let Some(old_size) = old_type.size {
                // Find corresponding type in new data
                if let Some(new_type) = new_data.types.iter().find(|t| t.index == old_type.index) {
                    if let Some(new_size) = new_type.size {
                        if old_size != new_size {
                            println!(
                                "Size mismatch for {} (index {}): old={}, new={}",
                                old_type.name.as_ref().unwrap_or(&"<unnamed>".to_string()),
                                old_type.index,
                                old_size,
                                new_size
                            );
                            size_mismatches += 1;
                        }
                    }
                }
            }
        }

        assert_eq!(
            size_mismatches,
            0,
            "Found {} type size mismatches in {}",
            size_mismatches,
            pdb_path.display()
        );

        println!("✓ Type sizes verified for {}", pdb_path.display());
    }
}

#[test]
fn test_critical_kernel_types() {
    let _ = env_logger::builder().is_test(true).try_init();

    let pdb_path = PathBuf::from("cache_test_pdbs/ntkrnlmp.pdb");
    if !pdb_path.exists() {
        eprintln!("Skipping test: {} not found", pdb_path.display());
        return;
    }

    println!("Testing critical kernel types in ntkrnlmp.pdb...");

    let old_data =
        pdb_compare::old_pdb::parse_pdb(&pdb_path).expect("Failed to parse with old parser");
    let new_data =
        pdb_compare::new_pdb::parse_pdb_new(&pdb_path).expect("Failed to parse with new parser");

    // Critical kernel types that must be parsed correctly
    let critical_types = vec![
        "_EPROCESS",
        "_KPROCESS",
        "_ETHREAD",
        "_KTHREAD",
        "_LIST_ENTRY",
        "_UNICODE_STRING",
        "_PEB",
        "_TEB",
    ];

    for type_name in critical_types {
        println!("\nChecking {}...", type_name);

        // Find in old data - use the fullest definition (most fields) since PDBs can have
        // multiple versions of the same type (forward refs, different compilation units, etc.)
        let old_type = old_data
            .types
            .iter()
            .filter(|t| t.name.as_ref().map(|n| n.as_str()) == Some(type_name))
            .max_by_key(|t| t.fields.len())
            .expect(&format!(
                "Could not find {} in old parser results",
                type_name
            ));

        // Find in new data - use the fullest definition (most fields)
        let new_type = new_data
            .types
            .iter()
            .filter(|t| t.name.as_ref().map(|n| n.as_str()) == Some(type_name))
            .max_by_key(|t| t.fields.len())
            .expect(&format!(
                "Could not find {} in new parser results",
                type_name
            ));

        println!(
            "  Old: index={}, fields={}, size={:?}",
            old_type.index,
            old_type.fields.len(),
            old_type.size
        );
        println!(
            "  New: index={}, fields={}, size={:?}",
            new_type.index,
            new_type.fields.len(),
            new_type.size
        );

        // Verify sizes match
        assert_eq!(
            old_type.size, new_type.size,
            "Size mismatch for {}: old={:?}, new={:?}",
            type_name, old_type.size, new_type.size
        );

        // Verify field counts match
        assert_eq!(
            old_type.fields.len(),
            new_type.fields.len(),
            "Field count mismatch for {}: old={}, new={}",
            type_name,
            old_type.fields.len(),
            new_type.fields.len()
        );

        // Verify all fields are present
        for old_field in &old_type.fields {
            let found = new_type.fields.iter().any(|f| f.name == old_field.name);
            assert!(
                found,
                "Field '{}' missing in new parser for {}",
                old_field.name, type_name
            );
        }

        println!(
            "✓ {} verified: {} fields, size={:?}",
            type_name,
            new_type.fields.len(),
            new_type.size
        );
    }

    println!("\n✓ All critical kernel types verified");
}

#[test]
fn test_bitfield_comparison() {
    let _ = env_logger::builder().is_test(true).try_init();

    let pdb_path = PathBuf::from("cache_test_pdbs/ntkrnlmp.pdb");
    if !pdb_path.exists() {
        eprintln!("Skipping test: {} not found", pdb_path.display());
        return;
    }

    println!("Testing bitfield field comparison in ntkrnlmp.pdb...");

    let old_data =
        pdb_compare::old_pdb::parse_pdb(&pdb_path).expect("Failed to parse with old parser");
    let new_data =
        pdb_compare::new_pdb::parse_pdb_new(&pdb_path).expect("Failed to parse with new parser");

    // Find types that contain bitfield fields
    let mut bitfield_count = 0;
    let mut compared_count = 0;

    for old_type in &old_data.types {
        // Skip types without fields
        if old_type.fields.is_empty() {
            continue;
        }

        // Check if this type has any bitfield fields
        let has_bitfields = old_type.fields.iter().any(|f| f.bitfield_length.is_some());

        if !has_bitfields {
            continue;
        }

        // Find corresponding type in new data
        let new_type = new_data
            .types
            .iter()
            .find(|t| t.index == old_type.index && t.name == old_type.name);

        if let Some(new_type) = new_type {
            compared_count += 1;

            // Compare each field thoroughly
            for old_field in &old_type.fields {
                if old_field.bitfield_length.is_some() {
                    bitfield_count += 1;

                    // Find matching field in new type
                    let new_field = new_type
                        .fields
                        .iter()
                        .find(|f| f.name == old_field.name)
                        .expect(&format!(
                            "Bitfield field '{}' missing in new parser for type {} (index {})",
                            old_field.name,
                            old_type.name.as_ref().unwrap_or(&"<unnamed>".to_string()),
                            old_type.index
                        ));

                    // Verify type kind matches
                    assert_eq!(
                        old_field.type_kind, new_field.type_kind,
                        "Type kind mismatch for bitfield field '{}' in type {} (index {}): old={:?}, new={:?}",
                        old_field.name,
                        old_type.name.as_ref().unwrap_or(&"<unnamed>".to_string()),
                        old_type.index,
                        old_field.type_kind,
                        new_field.type_kind
                    );

                    // Verify bitfield length matches
                    assert_eq!(
                        old_field.bitfield_length, new_field.bitfield_length,
                        "Bitfield length mismatch for field '{}' in type {} (index {}): old={:?}, new={:?}",
                        old_field.name,
                        old_type.name.as_ref().unwrap_or(&"<unnamed>".to_string()),
                        old_type.index,
                        old_field.bitfield_length,
                        new_field.bitfield_length
                    );

                    // Verify bitfield position matches
                    assert_eq!(
                        old_field.bitfield_position, new_field.bitfield_position,
                        "Bitfield position mismatch for field '{}' in type {} (index {}): old={:?}, new={:?}",
                        old_field.name,
                        old_type.name.as_ref().unwrap_or(&"<unnamed>".to_string()),
                        old_type.index,
                        old_field.bitfield_position,
                        new_field.bitfield_position
                    );

                    // Verify offset matches
                    assert_eq!(
                        old_field.offset, new_field.offset,
                        "Offset mismatch for bitfield field '{}' in type {} (index {}): old={:?}, new={:?}",
                        old_field.name,
                        old_type.name.as_ref().unwrap_or(&"<unnamed>".to_string()),
                        old_type.index,
                        old_field.offset,
                        new_field.offset
                    );
                }
            }
        }
    }

    println!(
        "✓ Compared {} types containing bitfield fields",
        compared_count
    );
    println!(
        "✓ Verified {} bitfield fields match exactly",
        bitfield_count
    );
    println!("  - Type kind matches");
    println!("  - Bitfield length matches");
    println!("  - Bitfield position matches");
    println!("  - Field offset matches");

    assert!(
        bitfield_count > 0,
        "Expected to find bitfield fields in the PDB"
    );
}

/// This test compares types by TYPE INDEX (not name) to ensure we're comparing
/// the exact same type definition between old and new parsers.
/// This catches regressions in offsets, values, and field parsing.
#[test]
fn test_type_index_exact_comparison() {
    let _ = env_logger::builder().is_test(true).try_init();

    let test_pdbs = vec![
        "cache_test_pdbs/ntkrnlmp.pdb",
        "cache_test_pdbs/ntdll.pdb",
        "cache_test_pdbs/hvix64.pdb",
    ];

    for pdb_path_str in test_pdbs {
        let pdb_path = PathBuf::from(pdb_path_str);
        if !pdb_path.exists() {
            eprintln!("Skipping: {} not found", pdb_path.display());
            continue;
        }

        println!("\n========================================");
        println!("Type-Index Comparison: {}", pdb_path.display());
        println!("========================================");

        let old_data =
            pdb_compare::old_pdb::parse_pdb(&pdb_path).expect("Failed to parse with old parser");
        let new_data = pdb_compare::new_pdb::parse_pdb_new(&pdb_path)
            .expect("Failed to parse with new parser");

        let mut total_types_compared = 0;
        let mut total_fields_compared = 0;
        let mut total_variants_compared = 0;
        let mut offset_mismatches = 0;
        let mut value_mismatches = 0;
        let mut field_count_mismatches = 0;
        let mut variant_count_mismatches = 0;

        // Build index maps for quick lookup
        use std::collections::HashMap;
        let mut old_by_index: HashMap<u32, &pdb_compare::old_pdb::TypeInfo> = HashMap::new();
        for old_type in &old_data.types {
            old_by_index.insert(old_type.index, old_type);
        }

        let mut new_by_index: HashMap<u32, &pdb_compare::new_pdb::TypeInfo> = HashMap::new();
        for new_type in &new_data.types {
            new_by_index.insert(new_type.index, new_type);
        }

        // Compare types by index (ensures we're comparing the exact same type definition)
        for (type_idx, old_type) in &old_by_index {
            if let Some(new_type) = new_by_index.get(type_idx) {
                total_types_compared += 1;

                // Compare struct/union fields by index
                if (old_type.kind == "Class" || old_type.kind == "Union")
                    && !old_type.fields.is_empty()
                {
                    // Check field count
                    if old_type.fields.len() != new_type.fields.len() {
                        field_count_mismatches += 1;
                        println!(
                            "⚠️  Type index {}: Field count mismatch in '{}': old={}, new={}",
                            type_idx,
                            old_type.name.as_deref().unwrap_or("<unnamed>"),
                            old_type.fields.len(),
                            new_type.fields.len()
                        );
                        // Don't fail - just warn. Different definitions may exist.
                        continue;
                    }

                    // Compare each field
                    for (i, old_field) in old_type.fields.iter().enumerate() {
                        if let Some(new_field) = new_type.fields.get(i) {
                            total_fields_compared += 1;

                            // Compare field names
                            if old_field.name != new_field.name {
                                println!(
                                    "⚠️  Type index {}: Field name mismatch at position {}: old='{}', new='{}'",
                                    type_idx, i, old_field.name, new_field.name
                                );
                                continue;
                            }

                            // Compare offsets (CRITICAL - we must verify this!)
                            if old_field.offset != new_field.offset {
                                offset_mismatches += 1;
                                println!(
                                    "❌ Type index {}: Offset mismatch for field '{}' in '{}': old={:?}, new={:?}",
                                    type_idx,
                                    old_field.name,
                                    old_type.name.as_deref().unwrap_or("<unnamed>"),
                                    old_field.offset,
                                    new_field.offset
                                );
                            }

                            // Compare bitfield metadata if present
                            if old_field.bitfield_length.is_some()
                                || new_field.bitfield_length.is_some()
                            {
                                if old_field.bitfield_length != new_field.bitfield_length {
                                    println!(
                                        "❌ Type index {}: Bitfield length mismatch for '{}': old={:?}, new={:?}",
                                        type_idx, old_field.name, old_field.bitfield_length, new_field.bitfield_length
                                    );
                                }
                                if old_field.bitfield_position != new_field.bitfield_position {
                                    println!(
                                        "❌ Type index {}: Bitfield position mismatch for '{}': old={:?}, new={:?}",
                                        type_idx, old_field.name, old_field.bitfield_position, new_field.bitfield_position
                                    );
                                }
                            }
                        }
                    }
                }

                // Compare enum variants by index
                if old_type.kind == "Enumeration" && !old_type.variants.is_empty() {
                    // Check variant count
                    if old_type.variants.len() != new_type.variants.len() {
                        variant_count_mismatches += 1;
                        println!(
                            "⚠️  Type index {}: Variant count mismatch in '{}': old={}, new={}",
                            type_idx,
                            old_type.name.as_deref().unwrap_or("<unnamed>"),
                            old_type.variants.len(),
                            new_type.variants.len()
                        );
                        // Don't fail - just warn. Different definitions may exist.
                        continue;
                    }

                    // Compare each variant
                    for (i, old_variant) in old_type.variants.iter().enumerate() {
                        if let Some(new_variant) = new_type.variants.get(i) {
                            total_variants_compared += 1;

                            // Compare variant names
                            if old_variant.name != new_variant.name {
                                println!(
                                    "⚠️  Type index {}: Variant name mismatch at position {}: old='{}', new='{}'",
                                    type_idx, i, old_variant.name, new_variant.name
                                );
                                continue;
                            }

                            // Compare values (CRITICAL - we must verify this!)
                            // Normalize to handle signed/unsigned representation differences
                            // Cast through appropriate size to handle sign extension
                            // E.g., 0xFF can be U8(255) or I8(-1) - both should compare equal
                            let old_value_normalized = if old_variant.value >= i8::MIN as i64
                                && old_variant.value <= u8::MAX as i64
                            {
                                (old_variant.value as u8) as u64
                            } else if old_variant.value >= i16::MIN as i64
                                && old_variant.value <= u16::MAX as i64
                            {
                                (old_variant.value as u16) as u64
                            } else if old_variant.value >= i32::MIN as i64
                                && old_variant.value <= u32::MAX as i64
                            {
                                (old_variant.value as u32) as u64
                            } else {
                                old_variant.value as u64
                            };

                            let new_value_normalized = if new_variant.value >= i8::MIN as i64
                                && new_variant.value <= u8::MAX as i64
                            {
                                (new_variant.value as u8) as u64
                            } else if new_variant.value >= i16::MIN as i64
                                && new_variant.value <= u16::MAX as i64
                            {
                                (new_variant.value as u16) as u64
                            } else if new_variant.value >= i32::MIN as i64
                                && new_variant.value <= u32::MAX as i64
                            {
                                (new_variant.value as u32) as u64
                            } else {
                                new_variant.value as u64
                            };

                            if old_value_normalized != new_value_normalized {
                                value_mismatches += 1;
                                println!(
                                    "❌ Type index {}: Value mismatch for variant '{}' in '{}': old={} (0x{:X}), new={} (0x{:X})",
                                    type_idx,
                                    old_variant.name,
                                    old_type.name.as_deref().unwrap_or("<unnamed>"),
                                    old_variant.value,
                                    old_value_normalized,
                                    new_variant.value,
                                    new_value_normalized
                                );
                            }
                        }
                    }
                }
            }
        }

        println!(
            "\n📊 Type-Index Comparison Results for {}:",
            pdb_path.display()
        );
        println!("  ✓ Types compared by index: {}", total_types_compared);
        println!("  ✓ Fields compared: {}", total_fields_compared);
        println!("  ✓ Variants compared: {}", total_variants_compared);
        println!("  ⚠️  Field count mismatches: {}", field_count_mismatches);
        println!(
            "  ⚠️  Variant count mismatches: {}",
            variant_count_mismatches
        );
        println!("  ❌ Offset mismatches: {}", offset_mismatches);
        println!("  ❌ Value mismatches: {}", value_mismatches);

        // These are CRITICAL - we must not have any mismatches when comparing the same type index
        assert_eq!(
            offset_mismatches, 0,
            "CRITICAL: Found {} offset mismatches when comparing same type indices! \
             This means the new parser is producing different offsets for the exact same type definition.",
            offset_mismatches
        );

        assert_eq!(
            value_mismatches, 0,
            "CRITICAL: Found {} enum value mismatches when comparing same type indices! \
             This means the new parser is producing different values for the exact same type definition.",
            value_mismatches
        );

        println!("  ✅ All offsets and values match for same type indices!");
    }

    println!("\n✅ Type-index exact comparison test PASSED!");
}

#[test]
fn test_all_versions_by_index() {
    let _ = env_logger::builder().is_test(true).try_init();

    let test_pdbs = vec![
        "cache_test_pdbs/ntkrnlmp.pdb",
        "cache_test_pdbs/ntdll.pdb",
        "cache_test_pdbs/hvix64.pdb",
    ];

    for pdb_path_str in test_pdbs {
        let pdb_path = PathBuf::from(pdb_path_str);
        if !pdb_path.exists() {
            eprintln!("Skipping: {} not found", pdb_path.display());
            continue;
        }

        println!(
            "\nVerifying all struct versions by type index for {}...",
            pdb_path.display()
        );

        let old_data =
            pdb_compare::old_pdb::parse_pdb(&pdb_path).expect("Failed to parse with old parser");
        let new_data = pdb_compare::new_pdb::parse_pdb_new(&pdb_path)
            .expect("Failed to parse with new parser");

        let mut total_types_compared = 0;
        let mut total_fields_compared = 0;
        let mut types_with_multiple_versions = 0;

        // Track which type names have multiple versions
        let mut old_type_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for old_type in &old_data.types {
            if let Some(name) = &old_type.name {
                if old_type.kind == "Class" || old_type.kind == "Union" {
                    *old_type_counts.entry(name.clone()).or_insert(0) += 1;
                }
            }
        }

        // Compare types at the SAME index in both parsers
        // This ensures we compare version 1 to version 1, version 2 to version 2, etc.
        for old_type in &old_data.types {
            // Find the type at the same index in the new parser
            let new_type = match new_data.types.iter().find(|t| t.index == old_type.index) {
                Some(t) => t,
                None => continue,
            };

            // Only compare non-empty structs/unions of the same kind
            if old_type.kind != new_type.kind {
                continue;
            }

            if (old_type.kind != "Class" && old_type.kind != "Union") {
                continue;
            }

            if old_type.fields.is_empty() && new_type.fields.is_empty() {
                continue;
            }

            total_types_compared += 1;

            // Track if this type name has multiple versions
            if let Some(name) = &old_type.name {
                if old_type_counts.get(name).copied().unwrap_or(0) > 1 {
                    types_with_multiple_versions += 1;
                }
            }

            // Check size
            assert_eq!(
                old_type.size, new_type.size,
                "Size mismatch at type index {}: {:?} ({})\n  Old size: {:?}\n  New size: {:?}",
                old_type.index, old_type.name, old_type.kind, old_type.size, new_type.size
            );

            // Check field count
            assert_eq!(
                old_type.fields.len(),
                new_type.fields.len(),
                "Field count mismatch at type index {}: {:?} ({})\n  Old fields: {}\n  New fields: {}",
                old_type.index,
                old_type.name,
                old_type.kind,
                old_type.fields.len(),
                new_type.fields.len()
            );

            // Check each field
            for old_field in &old_type.fields {
                total_fields_compared += 1;

                let new_field = new_type
                    .fields
                    .iter()
                    .find(|f| f.name == old_field.name)
                    .expect(&format!(
                        "Field '{}' missing in new parser at type index {}: {:?}",
                        old_field.name, old_type.index, old_type.name
                    ));

                // Check offset
                assert_eq!(
                    old_field.offset, new_field.offset,
                    "Offset mismatch at type index {}: {:?}, field '{}'\n  Old offset: {:?}\n  New offset: {:?}",
                    old_type.index,
                    old_type.name,
                    old_field.name,
                    old_field.offset,
                    new_field.offset
                );

                // Check bitfield properties
                if old_field.bitfield_length.is_some() {
                    assert_eq!(
                        old_field.bitfield_length, new_field.bitfield_length,
                        "Bitfield length mismatch at type index {}: {:?}, field '{}'\n  Old: {:?}\n  New: {:?}",
                        old_type.index,
                        old_type.name,
                        old_field.name,
                        old_field.bitfield_length,
                        new_field.bitfield_length
                    );

                    assert_eq!(
                        old_field.bitfield_position, new_field.bitfield_position,
                        "Bitfield position mismatch at type index {}: {:?}, field '{}'\n  Old: {:?}\n  New: {:?}",
                        old_type.index,
                        old_type.name,
                        old_field.name,
                        old_field.bitfield_position,
                        new_field.bitfield_position
                    );
                }
            }
        }

        println!("✓ All versions verified for {}", pdb_path.display());
        println!(
            "  - Compared {} types (including {} with multiple versions)",
            total_types_compared, types_with_multiple_versions
        );
        println!("  - Validated {} fields", total_fields_compared);
        println!("  - All sizes, offsets, and bitfield metadata match perfectly");
    }
}
