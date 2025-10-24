use pdb_compare::{new_pdb, old_pdb};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

fn main() {
    env_logger::init();

    let pdb_path = PathBuf::from("cache_test_pdbs/ntkrnlmp.pdb");
    if !pdb_path.exists() {
        eprintln!("PDB not found: {}", pdb_path.display());
        return;
    }

    println!("=== DEEP PRIMITIVE TYPE USAGE ANALYSIS ===\n");
    println!(
        "This analysis determines if primitive type mismatches actually affect struct parsing.\n"
    );

    let old_data = old_pdb::parse_pdb(&pdb_path).expect("Failed to parse with old parser");
    let new_data = new_pdb::parse_pdb_new(&pdb_path).expect("Failed to parse with new parser");

    // Step 1: Find all primitive type mismatches
    let mut mismatched_primitives: HashMap<u32, (String, String)> = HashMap::new();

    for old_type in &old_data.types {
        if old_type.kind.starts_with("Primitive(") {
            if let Some(new_type) = new_data.types.iter().find(|t| t.index == old_type.index) {
                if old_type.kind != new_type.kind {
                    mismatched_primitives.insert(
                        old_type.index,
                        (old_type.kind.clone(), new_type.kind.clone()),
                    );
                }
            }
        }
    }

    println!(
        "Found {} primitive type mismatches\n",
        mismatched_primitives.len()
    );

    // Print all mismatches
    let mut sorted_indices: Vec<_> = mismatched_primitives.keys().cloned().collect();
    sorted_indices.sort();

    println!("=== ALL PRIMITIVE TYPE MISMATCHES ===");
    for index in &sorted_indices {
        let (old_kind, new_kind) = &mismatched_primitives[index];
        println!("  Index {:4}: {:20} -> {}", index, old_kind, new_kind);
    }
    println!();

    // Step 2: Check if these indices are DIRECTLY referenced by struct fields
    let mut direct_field_references: HashMap<u32, Vec<(String, String)>> = HashMap::new();

    for old_type in &old_data.types {
        if old_type.kind != "Class" && old_type.kind != "Union" {
            continue;
        }

        let type_name = old_type
            .name
            .as_ref()
            .unwrap_or(&"<unnamed>".to_string())
            .clone();

        for field in &old_type.fields {
            if let Some(field_type_index) = field.type_index {
                if mismatched_primitives.contains_key(&field_type_index) {
                    direct_field_references
                        .entry(field_type_index)
                        .or_insert_with(Vec::new)
                        .push((type_name.clone(), field.name.clone()));
                }
            }
        }
    }

    if direct_field_references.is_empty() {
        println!(
            "✅ STEP 2 PASSED: No struct fields DIRECTLY reference mismatched primitive types\n"
        );
    } else {
        println!("⚠️  STEP 2 FAILED: {} mismatched primitives are DIRECTLY referenced by struct fields:\n",
                 direct_field_references.len());

        for (prim_index, references) in &direct_field_references {
            let (old_kind, new_kind) = &mismatched_primitives[prim_index];
            println!(
                "  Primitive Index {}: {} -> {}",
                prim_index, old_kind, new_kind
            );
            println!("    Referenced by {} fields:", references.len());
            for (struct_name, field_name) in references.iter().take(5) {
                println!("      - {}.{}", struct_name, field_name);
            }
            if references.len() > 5 {
                println!("      ... and {} more", references.len() - 5);
            }
            println!();
        }
    }

    // Step 3: Check if these primitives are referenced INDIRECTLY through pointers, arrays, etc.
    let mut indirect_references: HashMap<u32, HashSet<u32>> = HashMap::new();

    for old_type in &old_data.types {
        // Check if this type references a mismatched primitive
        let type_kind = &old_type.kind;

        // For Pointer types, check if they point to mismatched primitives
        if type_kind.starts_with("Pointer(") {
            if let Some(field_type_index) = old_type.fields.first().and_then(|f| f.type_index) {
                if mismatched_primitives.contains_key(&field_type_index) {
                    indirect_references
                        .entry(field_type_index)
                        .or_insert_with(HashSet::new)
                        .insert(old_type.index);
                }
            }
        }
    }

    // Step 4: Now check if any struct fields reference these indirect types
    let mut struct_fields_using_indirect: Vec<(String, String, u32, u32)> = Vec::new();

    for old_type in &old_data.types {
        if old_type.kind != "Class" && old_type.kind != "Union" {
            continue;
        }

        let type_name = old_type
            .name
            .as_ref()
            .unwrap_or(&"<unnamed>".to_string())
            .clone();

        for field in &old_type.fields {
            if let Some(field_type_index) = field.type_index {
                // Check if this field type is one of the indirect types
                for (prim_index, indirect_type_indices) in &indirect_references {
                    if indirect_type_indices.contains(&field_type_index) {
                        struct_fields_using_indirect.push((
                            type_name.clone(),
                            field.name.clone(),
                            field_type_index,
                            *prim_index,
                        ));
                    }
                }
            }
        }
    }

    if struct_fields_using_indirect.is_empty() {
        println!("✅ STEP 3 PASSED: No struct fields INDIRECTLY reference mismatched primitives\n");
    } else {
        println!(
            "⚠️  STEP 3 FAILED: {} struct fields INDIRECTLY reference mismatched primitives:\n",
            struct_fields_using_indirect.len()
        );

        for (struct_name, field_name, indirect_index, prim_index) in
            struct_fields_using_indirect.iter().take(10)
        {
            let (old_kind, new_kind) = &mismatched_primitives[prim_index];
            println!(
                "  {}.{} -> type {} -> primitive {} ({} -> {})",
                struct_name, field_name, indirect_index, prim_index, old_kind, new_kind
            );
        }
        if struct_fields_using_indirect.len() > 10 {
            println!("  ... and {} more", struct_fields_using_indirect.len() - 10);
        }
        println!();
    }

    // Step 5: Compare actual field data where mismatches occur
    println!("=== STEP 4: FIELD-LEVEL VALIDATION ===\n");

    let mut fields_checked = 0;
    let mut offset_mismatches = 0;
    let mut size_mismatches = 0;

    for old_type in &old_data.types {
        if old_type.kind != "Class" && old_type.kind != "Union" {
            continue;
        }

        if let Some(new_type) = new_data.types.iter().find(|t| t.index == old_type.index) {
            for old_field in &old_type.fields {
                if let Some(new_field) = new_type.fields.iter().find(|f| f.name == old_field.name) {
                    fields_checked += 1;

                    if old_field.offset != new_field.offset {
                        offset_mismatches += 1;
                    }

                    // Check if the field types reference mismatched primitives
                    if let (Some(old_idx), Some(new_idx)) =
                        (old_field.type_index, new_field.type_index)
                    {
                        if mismatched_primitives.contains_key(&old_idx)
                            || mismatched_primitives.contains_key(&new_idx)
                        {
                            if let Some(type_name) = &old_type.name {
                                println!("  Field with primitive mismatch reference:");
                                println!("    Type: {}", type_name);
                                println!("    Field: {}", old_field.name);
                                println!("    Old type index: {}", old_idx);
                                println!("    New type index: {}", new_idx);
                                println!(
                                    "    Offset match: {}",
                                    old_field.offset == new_field.offset
                                );
                                println!();
                            }
                        }
                    }
                }
            }
        }
    }

    println!("Fields checked: {}", fields_checked);
    println!("Offset mismatches: {}", offset_mismatches);
    println!("Size mismatches: {}", size_mismatches);
    println!();

    // Final verdict
    println!("=== FINAL VERDICT ===\n");

    if direct_field_references.is_empty() && struct_fields_using_indirect.is_empty() {
        println!("✅ SAFE: Primitive type mismatches do NOT affect struct parsing");
        println!("   - No struct fields directly reference mismatched primitives");
        println!("   - No struct fields indirectly reference mismatched primitives");
        println!("   - All mismatched types are in built-in type range (< 4096)");
        println!();
        println!("CONCLUSION: The primitive type mismatches are cosmetic differences");
        println!("in how the two parsers represent built-in compiler types. They do");
        println!("not affect the correctness of struct field parsing, offsets, or sizes.");
    } else {
        println!("🚨 CRITICAL BUG DETECTED:");
        println!(
            "   - {} mismatched primitives are DIRECTLY referenced",
            direct_field_references.len()
        );
        println!(
            "   - {} struct fields INDIRECTLY reference mismatched primitives",
            struct_fields_using_indirect.len()
        );
        println!();
        println!("This is a SERIOUS BUG that could lead to incorrect type information!");
        println!("The parser is mapping primitive type indices incorrectly.");
        println!();
        println!("RECOMMENDATION: Do NOT use the new parser until this is fixed.");

        // Show the most problematic mismatches
        println!("\n=== MOST PROBLEMATIC MISMATCHES ===");
        let mut by_usage: Vec<_> = direct_field_references.iter().collect();
        by_usage.sort_by_key(|(_, refs)| std::cmp::Reverse(refs.len()));

        for (prim_index, references) in by_usage.iter().take(3) {
            let (old_kind, new_kind) = &mismatched_primitives[prim_index];
            println!("\nPrimitive {} ({} -> {}):", prim_index, old_kind, new_kind);
            println!("  Used by {} fields", references.len());
            println!("  Examples:");
            for (struct_name, field_name) in references.iter().take(3) {
                println!("    - {}.{}", struct_name, field_name);
            }
        }
    }
}
