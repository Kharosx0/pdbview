use anyhow::Result;
use ms_pdb::types::{fields::Field, TypeData};
use ms_pdb::Pdb;
use std::path::Path;

fn main() -> Result<()> {
    env_logger::init();

    let pdb_path = Path::new("cache_test_pdbs/ntkrnlmp.pdb");

    println!("Opening PDB: {}", pdb_path.display());
    let mut pdb = Pdb::open(pdb_path)?;

    let type_stream = pdb.read_type_stream()?;

    println!("Total types: {}", type_stream.num_types());

    // Find _HEAP_EXECUTED_BLOCK (index 62929 from test failure)
    let target_type_index = ms_pdb::types::TypeIndex(62929);

    println!(
        "\n=== Analyzing _HEAP_EXECUTED_BLOCK at index {} ===",
        target_type_index.0
    );

    let record = type_stream.record(target_type_index)?;
    match record.parse()? {
        TypeData::Struct(data) => {
            let name = data.name.to_string();
            println!("Struct name: {}", name);
            println!("Size: {}", data.length);
            let field_list_index = data.fixed.field_list.get();
            println!("Field list index: {}", field_list_index.0);

            println!("\n--- Method 1: Direct record inspection ---");
            let mut current_index = field_list_index;
            let mut record_count = 0;
            let mut total_fields_direct = 0;

            loop {
                record_count += 1;
                println!(
                    "\nFieldList Record #{} (TypeIndex: {})",
                    record_count, current_index.0
                );

                let fl_record = type_stream.record(current_index)?;
                match fl_record.parse()? {
                    TypeData::FieldList(fl) => {
                        let mut field_count_in_record = 0;
                        let mut has_continuation = false;

                        for field in fl.iter() {
                            match field {
                                Field::Member(m) => {
                                    field_count_in_record += 1;
                                    total_fields_direct += 1;
                                    println!(
                                        "  Field #{}: Member '{}' at offset {}, type {}",
                                        total_fields_direct, m.name, m.offset, m.ty.0
                                    );
                                }
                                Field::StaticMember(sm) => {
                                    field_count_in_record += 1;
                                    total_fields_direct += 1;
                                    println!(
                                        "  Field #{}: StaticMember '{}', type {}",
                                        total_fields_direct, sm.name, sm.ty.0
                                    );
                                }
                                Field::BaseClass(bc) => {
                                    field_count_in_record += 1;
                                    total_fields_direct += 1;
                                    println!(
                                        "  Field #{}: BaseClass at offset {}, type {}",
                                        total_fields_direct, bc.offset, bc.ty.0
                                    );
                                }
                                Field::Index(idx) => {
                                    has_continuation = true;
                                    println!("  -> Continuation pointer to TypeIndex {}", idx.0);
                                    current_index = idx;
                                }
                                Field::OneMethod(m) => {
                                    println!("  Method: '{}' (not counted as field)", m.name);
                                }
                                Field::Method(m) => {
                                    println!("  Method list: '{}' (not counted as field)", m.name);
                                }
                                Field::NestedType(nt) => {
                                    field_count_in_record += 1;
                                    total_fields_direct += 1;
                                    println!(
                                        "  Field #{}: NestedType '{}', type {}",
                                        total_fields_direct, nt.name, nt.nested_ty.0
                                    );
                                }
                                Field::VFuncTable(vft) => {
                                    field_count_in_record += 1;
                                    total_fields_direct += 1;
                                    println!(
                                        "  Field #{}: VFuncTable, type {}",
                                        total_fields_direct, vft.0
                                    );
                                }
                                Field::DirectVirtualBaseClass(vb) => {
                                    field_count_in_record += 1;
                                    total_fields_direct += 1;
                                    println!(
                                        "  Field #{}: DirectVirtualBaseClass at offset {}",
                                        total_fields_direct, vb.vbpoff
                                    );
                                }
                                Field::IndirectVirtualBaseClass(vb) => {
                                    field_count_in_record += 1;
                                    total_fields_direct += 1;
                                    println!(
                                        "  Field #{}: IndirectVirtualBaseClass at offset {}",
                                        total_fields_direct, vb.vbpoff
                                    );
                                }
                                _ => {
                                    println!("  Other field type: {:?}", field);
                                }
                            }
                        }

                        println!("  Fields in this record: {}", field_count_in_record);

                        if !has_continuation {
                            break;
                        }
                    }
                    _ => {
                        println!("  ERROR: Expected FieldList, got different type!");
                        break;
                    }
                }
            }

            println!("\n=== Summary (Direct) ===");
            println!(
                "Total fields found via direct inspection: {}",
                total_fields_direct
            );

            // Check what type 4533 is (the bInLRU field type)
            println!("\n--- Checking type 4533 (bInLRU field type) ---");
            let type_4533 = ms_pdb::types::TypeIndex(4533);
            match type_stream.record(type_4533) {
                Ok(rec) => {
                    println!("Type 4533 Leaf kind: {:?}", rec.kind);
                    match rec.parse() {
                        Ok(type_data) => {
                            println!("Type 4533 data: {:?}", type_data);
                        }
                        Err(e) => {
                            println!("Failed to parse type 4533: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to read type 4533: {}", e);
                }
            }

            // Method 2: Using iter_fields (the way ezpdb does it)
            println!("\n--- Method 2: Using iter_fields() ---");
            let mut total_fields_iter = 0;

            for field in type_stream.iter_fields(field_list_index) {
                match field {
                    Field::Member(m) => {
                        total_fields_iter += 1;
                        println!(
                            "Field #{}: Member '{}' at offset {}, type {}",
                            total_fields_iter, m.name, m.offset, m.ty.0
                        );
                    }
                    Field::StaticMember(sm) => {
                        total_fields_iter += 1;
                        println!(
                            "Field #{}: StaticMember '{}', type {}",
                            total_fields_iter, sm.name, sm.ty.0
                        );
                    }
                    Field::BaseClass(bc) => {
                        total_fields_iter += 1;
                        println!(
                            "Field #{}: BaseClass at offset {}, type {}",
                            total_fields_iter, bc.offset, bc.ty.0
                        );
                    }
                    Field::Index(idx) => {
                        println!(
                            "ERROR: Index should not be returned by iter_fields! Index: {}",
                            idx.0
                        );
                    }
                    Field::OneMethod(m) => {
                        println!("Method: '{}' (not counted as field)", m.name);
                    }
                    Field::Method(m) => {
                        println!("Method list: '{}' (not counted as field)", m.name);
                    }
                    Field::NestedType(nt) => {
                        total_fields_iter += 1;
                        println!(
                            "Field #{}: NestedType '{}', type {}",
                            total_fields_iter, nt.name, nt.nested_ty.0
                        );
                    }
                    Field::VFuncTable(vft) => {
                        total_fields_iter += 1;
                        println!("Field #{}: VFuncTable, type {}", total_fields_iter, vft.0);
                    }
                    Field::DirectVirtualBaseClass(vb) => {
                        total_fields_iter += 1;
                        println!(
                            "Field #{}: DirectVirtualBaseClass at offset {}",
                            total_fields_iter, vb.vbpoff
                        );
                    }
                    Field::IndirectVirtualBaseClass(vb) => {
                        total_fields_iter += 1;
                        println!(
                            "Field #{}: IndirectVirtualBaseClass at offset {}",
                            total_fields_iter, vb.vbpoff
                        );
                    }
                    _ => {
                        println!("Other field type: {:?}", field);
                    }
                }
            }

            println!("\n=== Summary (iter_fields) ===");
            println!(
                "Total fields found via iter_fields(): {}",
                total_fields_iter
            );

            // Now check what ezpdb parses
            println!("\n--- Checking ezpdb parsing ---");
            let ezpdb_info = ezpdb::parse_pdb(pdb_path, None)?;

            for (idx, type_ref) in &ezpdb_info.types {
                if *idx == target_type_index.0 {
                    let borrowed = type_ref.borrow();
                    if let ezpdb::type_info::Type::Class(c) = &*borrowed {
                        println!("ezpdb found {} fields for {}", c.fields.len(), c.name);
                        for (i, field_ref) in c.fields.iter().enumerate() {
                            let field_borrowed = field_ref.borrow();
                            match &*field_borrowed {
                                ezpdb::type_info::Type::Member(m) => {
                                    println!(
                                        "  Field {}: Member '{}' at offset {}",
                                        i + 1,
                                        m.name,
                                        m.offset
                                    );
                                }
                                ezpdb::type_info::Type::BaseClass(b) => {
                                    println!("  Field {}: BaseClass at offset {}", i + 1, b.offset);
                                }
                                ezpdb::type_info::Type::VirtualBaseClass(vb) => {
                                    println!(
                                        "  Field {}: VirtualBaseClass at offset {}",
                                        i + 1,
                                        vb.base_pointer_offset
                                    );
                                }
                                ezpdb::type_info::Type::StaticMember(sm) => {
                                    println!("  Field {}: StaticMember '{}'", i + 1, sm.name);
                                }
                                ezpdb::type_info::Type::Nested(n) => {
                                    println!("  Field {}: Nested '{}'", i + 1, n.name);
                                }
                                ezpdb::type_info::Type::VTable(_) => {
                                    println!("  Field {}: VTable", i + 1);
                                }
                                _ => {
                                    println!("  Field {}: Other type", i + 1);
                                }
                            }
                        }
                    }
                    break;
                }
            }
        }
        _ => {
            println!("Type at index {} is not a struct!", target_type_index.0);
        }
    }

    Ok(())
}
