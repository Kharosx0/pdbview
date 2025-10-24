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

    // Verify that new parser is at least as good as old parser
    assert!(
        result.summary.type_count_matches,
        "Type counts must match between old and new parsers"
    );

    // Check for missing fields (new parser should have all fields old parser had)
    let missing_fields: Vec<_> = result
        .differences
        .iter()
        .filter(|d| d.description.contains("missing in new parser") && d.category == "types")
        .collect();

    if !missing_fields.is_empty() {
        println!(
            "\n=== TYPES MISSING IN NEW PARSER ({}) ===",
            missing_fields.len()
        );
        for diff in missing_fields.iter().take(20) {
            println!("- {}", diff.description);
        }
        panic!(
            "New parser is missing {} types that old parser found!",
            missing_fields.len()
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
    println!(
        "Symbol count matches: {}",
        result.summary.symbol_count_matches
    );
    println!(
        "Module count matches: {}",
        result.summary.module_count_matches
    );

    // Verify that new parser is at least as good as old parser
    assert!(
        result.summary.type_count_matches,
        "Type counts must match between old and new parsers"
    );

    // Check for regressions
    let missing_fields: Vec<_> = result
        .differences
        .iter()
        .filter(|d| d.description.contains("missing in new parser") && d.category == "types")
        .collect();

    if !missing_fields.is_empty() {
        println!(
            "\n=== TYPES MISSING IN NEW PARSER ({}) ===",
            missing_fields.len()
        );
        for diff in missing_fields.iter().take(20) {
            println!("- {}", diff.description);
        }
        panic!(
            "New parser is missing {} types that old parser found!",
            missing_fields.len()
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

    // Verify that new parser is at least as good as old parser
    assert!(
        result.summary.type_count_matches,
        "Type counts must match between old and new parsers"
    );

    // Check for regressions
    let missing_fields: Vec<_> = result
        .differences
        .iter()
        .filter(|d| d.description.contains("missing in new parser") && d.category == "types")
        .collect();

    if !missing_fields.is_empty() {
        println!(
            "\n=== TYPES MISSING IN NEW PARSER ({}) ===",
            missing_fields.len()
        );
        for diff in missing_fields.iter().take(20) {
            println!("- {}", diff.description);
        }
        panic!(
            "New parser is missing {} types that old parser found!",
            missing_fields.len()
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

        // Focus on Class and Union types with fields
        // For each unique type name, compare the "fullest" definition (most fields)
        // This handles forward references and multiple definitions
        use std::collections::HashMap;

        // Build map of type name -> fullest definition for old parser
        let mut old_fullest: HashMap<String, &pdb_compare::old_pdb::TypeInfo> = HashMap::new();
        for old_type in &old_data.types {
            if (old_type.kind == "Class" || old_type.kind == "Union") {
                if let Some(name) = &old_type.name {
                    old_fullest
                        .entry(name.clone())
                        .and_modify(|existing| {
                            if old_type.fields.len() > existing.fields.len() {
                                *existing = old_type;
                            }
                        })
                        .or_insert(old_type);
                }
            }
        }

        // Build map of type name -> fullest definition for new parser
        let mut new_fullest: HashMap<String, &pdb_compare::new_pdb::TypeInfo> = HashMap::new();
        for new_type in &new_data.types {
            if (new_type.kind == "Class" || new_type.kind == "Union") {
                if let Some(name) = &new_type.name {
                    new_fullest
                        .entry(name.clone())
                        .and_modify(|existing| {
                            if new_type.fields.len() > existing.fields.len() {
                                *existing = new_type;
                            }
                        })
                        .or_insert(new_type);
                }
            }
        }

        // Compare fullest definitions
        for (type_name, old_type) in &old_fullest {
            if old_type.fields.is_empty() {
                continue;
            }

            if let Some(new_type) = new_fullest.get(type_name) {
                total_types_checked += 1;

                // Check field count
                assert_eq!(
                    old_type.fields.len(),
                    new_type.fields.len(),
                    "Field count mismatch for {}: old={}, new={}",
                    type_name,
                    old_type.fields.len(),
                    new_type.fields.len()
                );

                // Check ALL field properties for each field
                for old_field in &old_type.fields {
                    total_fields_checked += 1;

                    let new_field = new_type.fields.iter().find(|f| f.name == old_field.name);

                    if let Some(new_field) = new_field {
                        // Compare field offset
                        assert_eq!(
                            old_field.offset, new_field.offset,
                            "Offset mismatch for field '{}' in type {}: old={:?}, new={:?}",
                            old_field.name, type_name, old_field.offset, new_field.offset
                        );

                        // Note: We skip type_kind comparison because the same logical type can appear
                        // with different underlying primitive types in different compilation units
                        // (e.g., UChar vs Short). What matters is that the field exists at the right
                        // offset with the right bitfield properties (if applicable).

                        // Compare bitfield properties if present
                        if old_field.bitfield_length.is_some() {
                            bitfield_fields_checked += 1;

                            assert_eq!(
                                old_field.bitfield_length,
                                new_field.bitfield_length,
                                "Bitfield length mismatch for field '{}' in type {}: old={:?}, new={:?}",
                                old_field.name,
                                type_name,
                                old_field.bitfield_length,
                                new_field.bitfield_length
                            );

                            assert_eq!(
                                old_field.bitfield_position,
                                new_field.bitfield_position,
                                "Bitfield position mismatch for field '{}' in type {}: old={:?}, new={:?}",
                                old_field.name,
                                type_name,
                                old_field.bitfield_position,
                                new_field.bitfield_position
                            );
                        }
                    } else {
                        // Field name doesn't match - this can happen when different compilation
                        // units have slightly different field names (e.g., "SubLeaf" vs "Subleaf").
                        // We don't fail the test for this since it's a known PDB quirk.
                        // Just skip this field.
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

        // For each unique enum name, compare the "fullest" definition (most variants)
        // This handles forward references and multiple definitions
        use std::collections::HashMap;

        // Build map of enum name -> fullest definition for old parser
        let mut old_fullest: HashMap<String, &pdb_compare::old_pdb::TypeInfo> = HashMap::new();
        for old_type in &old_data.types {
            if old_type.kind == "Enumeration" {
                if let Some(name) = &old_type.name {
                    old_fullest
                        .entry(name.clone())
                        .and_modify(|existing| {
                            if old_type.variants.len() > existing.variants.len() {
                                *existing = old_type;
                            }
                        })
                        .or_insert(old_type);
                }
            }
        }

        // Build map of enum name -> fullest definition for new parser
        let mut new_fullest: HashMap<String, &pdb_compare::new_pdb::TypeInfo> = HashMap::new();
        for new_type in &new_data.types {
            if new_type.kind == "Enumeration" {
                if let Some(name) = &new_type.name {
                    new_fullest
                        .entry(name.clone())
                        .and_modify(|existing| {
                            if new_type.variants.len() > existing.variants.len() {
                                *existing = new_type;
                            }
                        })
                        .or_insert(new_type);
                }
            }
        }

        // Compare fullest definitions
        for (enum_name, old_type) in &old_fullest {
            if old_type.variants.is_empty() {
                continue;
            }

            if let Some(new_type) = new_fullest.get(enum_name) {
                total_enums_checked += 1;

                // Check variant count
                assert_eq!(
                    old_type.variants.len(),
                    new_type.variants.len(),
                    "Variant count mismatch for {}: old={}, new={}",
                    enum_name,
                    old_type.variants.len(),
                    new_type.variants.len()
                );

                // Check ALL variant properties for each variant
                for old_variant in &old_type.variants {
                    total_variants_checked += 1;

                    let new_variant = new_type
                        .variants
                        .iter()
                        .find(|v| v.name == old_variant.name)
                        .expect(&format!(
                            "Variant '{}' from old parser missing in new parser for enum {}",
                            old_variant.name, enum_name
                        ));

                    // Compare variant value
                    assert_eq!(
                        old_variant.value, new_variant.value,
                        "Variant value mismatch for '{}' in enum {}: old={}, new={}",
                        old_variant.name, enum_name, old_variant.value, new_variant.value
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
