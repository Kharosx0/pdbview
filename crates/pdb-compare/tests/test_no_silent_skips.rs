use pdb_compare::{new_pdb, old_pdb};
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn test_no_silent_skips_that_hide_bugs() {
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

        println!("\nChecking for silent skips in {}", pdb_path.display());
        println!("{:=<80}", "");

        let old_data = old_pdb::parse_pdb(&pdb_path).expect("Failed to parse with old parser");
        let new_data = new_pdb::parse_pdb_new(&pdb_path).expect("Failed to parse with new parser");

        // Build index maps for fast lookup
        let mut old_by_index: HashMap<u32, &old_pdb::TypeInfo> = HashMap::new();
        for old_type in &old_data.types {
            old_by_index.insert(old_type.index, old_type);
        }

        let mut new_by_index: HashMap<u32, &new_pdb::TypeInfo> = HashMap::new();
        for new_type in &new_data.types {
            new_by_index.insert(new_type.index, new_type);
        }

        let mut missing_in_new = Vec::new();
        let mut missing_in_old = Vec::new();
        let mut kind_mismatches = Vec::new();
        let mut size_mismatches = Vec::new();
        let mut field_count_mismatches = Vec::new();

        // Check: Are there types in old that aren't in new?
        for (index, old_type) in &old_by_index {
            if let Some(new_type) = new_by_index.get(index) {
                // Type exists in both - check for differences

                // Check kind mismatch
                if old_type.kind != new_type.kind {
                    kind_mismatches.push((*index, old_type, new_type));
                }

                // Check size mismatch (only for non-forward-references)
                if old_type.size.is_some()
                    && new_type.size.is_some()
                    && old_type.size != new_type.size
                {
                    size_mismatches.push((*index, old_type, new_type));
                }

                // Check field count mismatch (only for same kind)
                if old_type.kind == new_type.kind && old_type.fields.len() != new_type.fields.len()
                {
                    field_count_mismatches.push((*index, old_type, new_type));
                }
            } else {
                // Type in old but not in new
                missing_in_new.push((*index, old_type));
            }
        }

        // Check: Are there types in new that aren't in old?
        for (index, new_type) in &new_by_index {
            if !old_by_index.contains_key(index) {
                missing_in_old.push((*index, new_type));
            }
        }

        // Report findings
        println!("\n=== DIAGNOSTIC RESULTS ===");

        if !missing_in_new.is_empty() {
            println!(
                "\n⚠️  WARNING: {} types in OLD but NOT in NEW:",
                missing_in_new.len()
            );
            for (index, old_type) in missing_in_new.iter().take(10) {
                println!(
                    "  - Index {}: {:?} ({})",
                    index, old_type.name, old_type.kind
                );
            }
            if missing_in_new.len() > 10 {
                println!("  ... and {} more", missing_in_new.len() - 10);
            }
        }

        if !missing_in_old.is_empty() {
            println!(
                "\n⚠️  WARNING: {} types in NEW but NOT in OLD:",
                missing_in_old.len()
            );
            for (index, new_type) in missing_in_old.iter().take(10) {
                println!(
                    "  - Index {}: {:?} ({})",
                    index, new_type.name, new_type.kind
                );
            }
            if missing_in_old.len() > 10 {
                println!("  ... and {} more", missing_in_old.len() - 10);
            }
        }

        if !kind_mismatches.is_empty() {
            println!(
                "\n⚠️  WARNING: {} types with KIND MISMATCH at same index:",
                kind_mismatches.len()
            );
            for (index, old_type, new_type) in kind_mismatches.iter().take(10) {
                println!(
                    "  - Index {}: {:?} - old={}, new={}",
                    index, old_type.name, old_type.kind, new_type.kind
                );
            }
            if kind_mismatches.len() > 10 {
                println!("  ... and {} more", kind_mismatches.len() - 10);
            }
        }

        if !size_mismatches.is_empty() {
            println!(
                "\n⚠️  WARNING: {} types with SIZE MISMATCH at same index:",
                size_mismatches.len()
            );
            for (index, old_type, new_type) in size_mismatches.iter().take(10) {
                println!(
                    "  - Index {}: {:?} ({}) - old={:?}, new={:?}",
                    index, old_type.name, old_type.kind, old_type.size, new_type.size
                );
            }
            if size_mismatches.len() > 10 {
                println!("  ... and {} more", size_mismatches.len() - 10);
            }
        }

        if !field_count_mismatches.is_empty() {
            println!(
                "\n⚠️  WARNING: {} types with FIELD COUNT MISMATCH at same index:",
                field_count_mismatches.len()
            );
            for (index, old_type, new_type) in field_count_mismatches.iter().take(10) {
                println!(
                    "  - Index {}: {:?} ({}) - old={} fields, new={} fields",
                    index,
                    old_type.name,
                    old_type.kind,
                    old_type.fields.len(),
                    new_type.fields.len()
                );
            }
            if field_count_mismatches.len() > 10 {
                println!("  ... and {} more", field_count_mismatches.len() - 10);
            }
        }

        // Summary
        println!("\n=== SUMMARY for {} ===", pdb_path.display());
        println!("Total types in OLD: {}", old_by_index.len());
        println!("Total types in NEW: {}", new_by_index.len());
        println!("Types only in OLD:  {}", missing_in_new.len());
        println!("Types only in NEW:  {}", missing_in_old.len());
        println!("Kind mismatches:    {}", kind_mismatches.len());
        println!("Size mismatches:    {}", size_mismatches.len());
        println!("Field mismatches:   {}", field_count_mismatches.len());

        // Determine if this is acceptable or a bug
        let total_issues = missing_in_new.len()
            + missing_in_old.len()
            + kind_mismatches.len()
            + size_mismatches.len()
            + field_count_mismatches.len();

        if total_issues == 0 {
            println!("\n✅ PERFECT: No silent skips or differences detected");
        } else {
            println!("\n⚠️  ATTENTION: {} differences detected", total_issues);
            println!("\nThese differences were being SILENTLY SKIPPED by continue statements.");
            println!("We need to investigate whether these are:");
            println!(
                "  1. Expected differences (e.g., old parser includes FieldList, new doesn't)"
            );
            println!("  2. Real bugs that need fixing");
        }

        // ASSERTIONS: Fail the test if we find unexpected differences
        // We expect some differences due to implementation details, but let's document them

        // Expected: Old parser stores FieldList/MethodList/ArgumentList as top-level types
        // New parser correctly does not store these
        let expected_missing_in_new: usize = missing_in_new
            .iter()
            .filter(|(_, old_type)| {
                old_type.kind == "FieldList"
                    || old_type.kind == "MethodList"
                    || old_type.kind == "ArgumentList"
            })
            .count();

        let unexpected_missing_in_new = missing_in_new.len() - expected_missing_in_new;

        // Expected: New parser has IPI stream types (FuncId, MFuncId, etc) and VTableShape that old parser doesn't
        let expected_missing_in_old: usize = missing_in_old
            .iter()
            .filter(|(_, new_type)| {
                new_type.kind == "FuncId"
                    || new_type.kind == "MFuncId"
                    || new_type.kind == "StringId"
                    || new_type.kind == "SubStrList"
                    || new_type.kind == "BuildInfoType"
                    || new_type.kind == "UdtSrcLineType"
                    || new_type.kind == "VTableShape"
            })
            .count();

        let unexpected_missing_in_old = missing_in_old.len() - expected_missing_in_old;

        println!("\n=== EXPECTED vs UNEXPECTED ===");
        println!(
            "Expected missing in NEW: {} (FieldList/MethodList/ArgumentList)",
            expected_missing_in_new
        );
        println!("Unexpected missing in NEW: {}", unexpected_missing_in_new);
        println!(
            "Expected missing in OLD: {} (IPI stream types + VTableShape)",
            expected_missing_in_old
        );
        println!("Unexpected missing in OLD: {}", unexpected_missing_in_old);

        // FAIL the test if we have unexpected differences
        assert_eq!(
            unexpected_missing_in_new, 0,
            "Found {} types in OLD parser that are unexpectedly missing in NEW parser",
            unexpected_missing_in_new
        );

        assert_eq!(
            unexpected_missing_in_old, 0,
            "Found {} types in NEW parser that are unexpectedly missing in OLD parser",
            unexpected_missing_in_old
        );

        // Expected: Primitive types (index < 4096) may have different names between parsers
        // These are built-in types and don't affect actual struct parsing
        let expected_kind_mismatches: usize = kind_mismatches
            .iter()
            .filter(|(index, old_type, new_type)| {
                *index < 4096
                    && old_type.kind.starts_with("Primitive(")
                    && new_type.kind.starts_with("Primitive(")
            })
            .count();

        let unexpected_kind_mismatches = kind_mismatches.len() - expected_kind_mismatches;

        println!(
            "Expected kind mismatches: {} (built-in primitive types < 4096)",
            expected_kind_mismatches
        );
        println!("Unexpected kind mismatches: {}", unexpected_kind_mismatches);

        assert_eq!(
            unexpected_kind_mismatches,
            0,
            "Found {} UNEXPECTED types where OLD and NEW parsers disagree on type kind at same index",
            unexpected_kind_mismatches
        );

        assert_eq!(
            size_mismatches.len(),
            0,
            "Found {} types where OLD and NEW parsers disagree on type size at same index",
            size_mismatches.len()
        );

        assert_eq!(
            field_count_mismatches.len(),
            0,
            "Found {} types where OLD and NEW parsers disagree on field count at same index",
            field_count_mismatches.len()
        );

        println!("\n✅ All differences are expected and documented");
    }
}
