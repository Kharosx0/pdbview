use anyhow::Result;
use ms_pdb::types::{fields::Field, TypeData};
use ms_pdb::Pdb;
use std::path::Path;

fn main() -> Result<()> {
    env_logger::init();

    let pdb_path = Path::new("cache_test_pdbs/ntkrnlmp.pdb");

    println!("Opening PDB: {}", pdb_path.display());
    let pdb = Pdb::open(pdb_path)?;

    let type_stream = pdb.read_type_stream()?;

    println!("Total types: {}", type_stream.num_types());

    // Find _EPROCESS - check both index 5761 (new parser) and 20653 (old parser)
    let indices_to_check = vec![5761, 20653];

    for check_index in indices_to_check {
        println!("\n===========================================");
        println!("Checking _EPROCESS at index {}", check_index);
        println!("===========================================");

        let ti = ms_pdb::types::TypeIndex(check_index);
        if let Ok(record) = type_stream.record(ti) {
            if let Ok(TypeData::Struct(data)) = record.parse() {
                let name = data.name.to_string();
                let field_list = data.fixed.field_list.get();

                println!("Name: {}", name);
                println!("Size: {}", data.length);
                println!("Field list index: {}", field_list.0);

                if field_list.0 == 0 {
                    println!("SKIPPING: Forward reference");
                    continue;
                }

                analyze_eprocess(check_index, ti, field_list, &type_stream, pdb_path)?;
            }
        }
    }

    return Ok(());
}

fn analyze_eprocess(
    struct_index: u32,
    ti: ms_pdb::types::TypeIndex,
    field_list_index: ms_pdb::types::TypeIndex,
    type_stream: &ms_pdb::tpi::TypeStream<Vec<u8>>,
    pdb_path: &Path,
) -> Result<()> {
    println!("\n=== Analyzing field list {} ===", field_list_index.0);

    // Collect all fields using iter_fields
    println!("\n--- Using iter_fields() ---");
    let mut ms_pdb_fields = Vec::new();

    for field in type_stream.iter_fields(field_list_index) {
        match field {
            Field::Member(m) => {
                let offset = u64::try_from(m.offset).unwrap_or(0);
                ms_pdb_fields.push((m.name.to_string(), offset, m.ty.0));
            }
            Field::BaseClass(bc) => {
                let offset = u64::try_from(bc.offset).unwrap_or(0);
                ms_pdb_fields.push((format!("<base class {}>", bc.ty.0), offset, bc.ty.0));
            }
            Field::DirectVirtualBaseClass(vb) => {
                let offset = u64::try_from(vb.vbpoff).unwrap_or(0);
                ms_pdb_fields.push((
                    format!("<direct vbase {}>", vb.fixed.btype.get().0),
                    offset,
                    vb.fixed.btype.get().0,
                ));
            }
            Field::IndirectVirtualBaseClass(vb) => {
                let offset = u64::try_from(vb.vbpoff).unwrap_or(0);
                ms_pdb_fields.push((
                    format!("<indirect vbase {}>", vb.fixed.btype.get().0),
                    offset,
                    vb.fixed.btype.get().0,
                ));
            }
            Field::StaticMember(sm) => {
                println!("StaticMember: '{}', type {}", sm.name, sm.ty.0);
            }
            Field::NestedType(nt) => {
                println!("NestedType: '{}', type {}", nt.name, nt.nested_ty.0);
            }
            Field::VFuncTable(vft) => {
                ms_pdb_fields.push((format!("<vftable {}>", vft.0), 0, vft.0));
            }
            _ => {}
        }
    }

    println!("\nms-pdb found {} data fields", ms_pdb_fields.len());

    // Now check what ezpdb parses
    println!("\n--- Checking NEW ezpdb parsing ---");
    let ezpdb_info = ezpdb::parse_pdb(pdb_path, None)?;

    let mut ezpdb_fields = Vec::new();
    for (idx, type_ref) in &ezpdb_info.types {
        if *idx == ti.0 {
            let borrowed = type_ref.borrow();
            if let ezpdb::type_info::Type::Class(c) = &*borrowed {
                println!("NEW ezpdb found {} fields for {}", c.fields.len(), c.name);
                for field_ref in &c.fields {
                    let field_borrowed = field_ref.borrow();
                    match &*field_borrowed {
                        ezpdb::type_info::Type::Member(m) => {
                            ezpdb_fields.push((m.name.clone(), m.offset as u64));
                        }
                        ezpdb::type_info::Type::BaseClass(b) => {
                            ezpdb_fields.push((format!("<base class>"), b.offset as u64));
                        }
                        ezpdb::type_info::Type::VirtualBaseClass(vb) => {
                            ezpdb_fields.push((format!("<vbase>"), vb.base_pointer_offset as u64));
                        }
                        ezpdb::type_info::Type::VTable(_) => {
                            ezpdb_fields.push((format!("<vtable>"), 0));
                        }
                        ezpdb::type_info::Type::StaticMember(sm) => {
                            ezpdb_fields.push((format!("<static: {}>", sm.name), 0));
                        }
                        ezpdb::type_info::Type::Nested(n) => {
                            ezpdb_fields.push((format!("<nested: {}>", n.name), 0));
                        }
                        _ => {}
                    }
                }
            }
            break;
        }
    }

    println!("\nNEW ezpdb collected {} data fields", ezpdb_fields.len());

    // Check what old ezpdb found
    println!("\n--- Checking OLD ezpdb parsing ---");
    let old_ezpdb_info = ezpdb_old::parse_pdb(pdb_path, None)?;

    let mut old_ezpdb_fields = Vec::new();
    for (idx, type_ref) in &old_ezpdb_info.types {
        if *idx == ti.0 {
            let borrowed = type_ref.borrow();
            if let ezpdb_old::type_info::Type::Class(c) = &*borrowed {
                println!("OLD ezpdb found {} fields for {}", c.fields.len(), c.name);
                for field_ref in &c.fields {
                    let field_borrowed = field_ref.borrow();
                    match &*field_borrowed {
                        ezpdb_old::type_info::Type::Member(m) => {
                            old_ezpdb_fields.push((m.name.clone(), m.offset as u64));
                        }
                        ezpdb_old::type_info::Type::BaseClass(b) => {
                            old_ezpdb_fields.push((format!("<base class>"), b.offset as u64));
                        }
                        ezpdb_old::type_info::Type::VirtualBaseClass(vb) => {
                            old_ezpdb_fields
                                .push((format!("<vbase>"), vb.base_pointer_offset as u64));
                        }
                        ezpdb_old::type_info::Type::VTable(_) => {
                            old_ezpdb_fields.push((format!("<vtable>"), 0));
                        }
                        ezpdb_old::type_info::Type::StaticMember(sm) => {
                            old_ezpdb_fields.push((format!("<static: {}>", sm.name), 0));
                        }
                        ezpdb_old::type_info::Type::Nested(n) => {
                            old_ezpdb_fields.push((format!("<nested: {}>", n.name), 0));
                        }
                        _ => {}
                    }
                }
            }
            break;
        }
    }

    println!(
        "\nOLD ezpdb collected {} data fields",
        old_ezpdb_fields.len()
    );

    // Compare new vs old
    println!("\n=== Fields in OLD but not in NEW ===");
    let mut missing_count = 0;
    for (old_name, old_offset) in &old_ezpdb_fields {
        let found = ezpdb_fields
            .iter()
            .any(|(new_name, new_offset)| old_name == new_name && *old_offset == *new_offset);
        if !found {
            println!("  - '{}' at offset {}", old_name, old_offset);
            missing_count += 1;
        }
    }

    if missing_count == 0 {
        println!("  (none - all fields match!)");
    }

    Ok(())
}
