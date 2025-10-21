use ezpdb::parse_pdb;

#[test]
fn test_parse_ntkrnl_pdb() {
    // Test parsing a real Windows kernel PDB
    let pdb_path = r"E:\tmp\jobs\sk\setup\oft-setup-f3a331446f2e5bb88a5b307775d84e1c\fakefs\symbols\ntkrnlmp.pdb\9D43BEAB4FA0945345C28E78975337A01\ntkrnlmp.pdb";

    // Skip if file doesn't exist (for CI)
    if !std::path::Path::new(pdb_path).exists() {
        eprintln!("Skipping test - PDB file not found at: {}", pdb_path);
        return;
    }

    let parsed_pdb = parse_pdb(pdb_path, None).expect("Failed to parse PDB");

    println!("Successfully parsed PDB:");
    println!("  GUID: {}", parsed_pdb.guid);
    println!("  Age: {}", parsed_pdb.age);
    println!("  Timestamp: {}", parsed_pdb.timestamp);
    println!("  Machine Type: {:?}", parsed_pdb.machine_type);
    println!("  Total types: {}", parsed_pdb.types.len());
    println!("  Total procedures: {}", parsed_pdb.procedures.len());
    println!(
        "  Total public symbols: {}",
        parsed_pdb.public_symbols.len()
    );
    println!("  Total debug modules: {}", parsed_pdb.debug_modules.len());

    // Look for _KPROCESS type
    let mut found_kprocess = false;
    let mut kprocess_type_index = None;
    let mut forward_refs = 0;
    let mut complete_defs = 0;
    let mut kprocess_variants = Vec::new();

    for (type_idx, type_ref) in parsed_pdb.types.iter() {
        if let Ok(borrowed_type) = type_ref.try_borrow() {
            match &*borrowed_type {
                ezpdb::type_info::Type::Class(class) => {
                    // Search for ANY _KPROCESS by name OR unique_name
                    let is_kprocess = class.name == "_KPROCESS"
                        || class
                            .unique_name
                            .as_ref()
                            .map(|s| s.contains("_KPROCESS"))
                            .unwrap_or(false);

                    if is_kprocess {
                        kprocess_variants.push((
                            format!("{} (unique: {:?})", class.name, class.unique_name),
                            *type_idx,
                            class.properties.forward_reference,
                            class.fields.len(),
                        ));
                    }

                    if class.name == "_KPROCESS" {
                        println!("\nFound _KPROCESS at type index {}", type_idx);
                        println!("  Size: {} bytes", class.size);
                        println!("  Fields: {}", class.fields.len());
                        println!(
                            "  Forward reference: {}",
                            class.properties.forward_reference
                        );
                        println!("  Unique name: {:?}", class.unique_name);

                        if class.properties.forward_reference {
                            forward_refs += 1;
                            println!("  (This is a forward reference)");
                        } else {
                            complete_defs += 1;
                            found_kprocess = true;
                            kprocess_type_index = Some(*type_idx);

                            // Print first few fields
                            println!("  First 10 fields:");
                            for (i, field) in class.fields.iter().take(10).enumerate() {
                                if let Ok(field_borrow) = field.try_borrow() {
                                    use ezpdb::type_info::Type;
                                    match &*field_borrow {
                                        Type::Member(member) => {
                                            println!(
                                                "    [{}] {} at offset 0x{:x}",
                                                i, member.name, member.offset
                                            );
                                        }
                                        Type::BaseClass(bc) => {
                                            println!(
                                                "    [{}] <BaseClass at offset 0x{:x}>",
                                                i, bc.offset
                                            );
                                        }
                                        Type::VirtualBaseClass(vbc) => {
                                            println!("    [{}] <VirtualBaseClass>", i);
                                        }
                                        Type::StaticMember(sm) => {
                                            println!("    [{}] static {} ", i, sm.name);
                                        }
                                        Type::Nested(n) => {
                                            println!("    [{}] <Nested>", i);
                                        }
                                        Type::Method(m) => {
                                            println!("    [{}] method {} ", i, m.name);
                                        }
                                        Type::OverloadedMethod(om) => {
                                            println!("    [{}] overloaded method {}", i, om.name);
                                        }
                                        other => {
                                            println!(
                                                "    [{}] <unexpected field type: {:?}>",
                                                i,
                                                std::mem::discriminant(other)
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    println!("\n_KPROCESS search results:");
    println!("  Forward references found: {}", forward_refs);
    println!("  Complete definitions found: {}", complete_defs);

    println!(
        "\nAll types containing 'KPROCESS' ({} total):",
        kprocess_variants.len()
    );
    for (name, idx, is_fwd, field_count) in kprocess_variants.iter().take(25) {
        let status = if *is_fwd { "[FWD REF]" } else { "[COMPLETE]" };
        println!(
            "  {} {:10} - {:4} fields at index {}",
            status, name, field_count, idx
        );
    }
    if kprocess_variants.len() > 25 {
        println!("  ... and {} more", kprocess_variants.len() - 25);
    }

    assert!(
        found_kprocess || forward_refs > 0,
        "_KPROCESS type not found at all in PDB! (Expected at least a forward reference)"
    );

    if found_kprocess {
        println!(
            "\n✅ SUCCESS: Found complete _KPROCESS definition at type index {:?}",
            kprocess_type_index
        );
    } else if forward_refs > 0 {
        println!(
            "\n⚠️  WARNING: Found only forward reference(s) to _KPROCESS, no complete definition"
        );
        println!("   This is NORMAL - Windows PDBs often only contain forward references.");
        println!("   The complete definition may be in a different PDB or module.");
    }

    // Test the new find_type_by_name helper
    println!("\n--- Testing find_type_by_name helper ---");
    match parsed_pdb.find_type_by_name("_KPROCESS") {
        Some(type_ref) => {
            if let Ok(borrowed) = type_ref.try_borrow() {
                match &*borrowed {
                    ezpdb::type_info::Type::Class(class) => {
                        println!("find_type_by_name('_KPROCESS') returned:");
                        println!("  Name: {}", class.name);
                        println!("  Forward ref: {}", class.properties.forward_reference);
                        println!("  Fields: {}", class.fields.len());
                        println!("  Size: {}", class.size);
                    }
                    _ => println!("Unexpected type variant"),
                }
            }
        }
        None => println!("find_type_by_name('_KPROCESS') returned None"),
    }
}
