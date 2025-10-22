/// Simple test to display BuildInfo contents from a PDB file
use anyhow::Result;
use std::path::Path;

fn main() -> Result<()> {
    let pdb_path = std::env::args().nth(1).unwrap_or_else(|| {
        "./crates/ms-pdb/target/debug/build/anyhow-11a718d5af00b923/build_script_build.pdb"
            .to_string()
    });

    println!("═══════════════════════════════════════════════════════════════");
    println!("               BUILD INFO DISPLAY TEST");
    println!("═══════════════════════════════════════════════════════════════\n");
    println!("PDB File: {}\n", pdb_path);

    // Parse with ezpdb
    let parsed_pdb = ezpdb::parse_pdb(Path::new(&pdb_path), None)?;

    // Display BuildInfo from assembly_info (parsed from symbols)
    println!("📋 BUILD INFO FROM SYMBOLS:");
    if let Some(ref build_info) = parsed_pdb.assembly_info.build_info {
        println!("   Found {} arguments:\n", build_info.arguments.len());

        let labels = [
            "Current Directory",
            "Compiler Path",
            "Source File",
            "PDB Path",
            "Command Line Args",
        ];

        for (i, arg) in build_info.arguments.iter().enumerate() {
            let label = labels.get(i).unwrap_or(&"Extra Arg");

            if arg.is_empty() {
                println!("   [{}] {}: <empty>", i, label);
            } else {
                println!("   [{}] {}:", i, label);
                if arg.len() > 120 {
                    // Split long args into multiple lines for readability
                    let mut pos = 0;
                    while pos < arg.len() {
                        let end = (pos + 100).min(arg.len());
                        println!("       {}", &arg[pos..end]);
                        pos = end;
                    }
                } else {
                    println!("       {}", arg);
                }
            }
            println!();
        }
    } else {
        println!("   ⚠️  No build info found in this PDB\n");
    }

    // Display sample BuildInfo types from IPI stream
    println!("🔨 BUILD INFO TYPES FROM IPI STREAM:");
    let mut found_count = 0;

    for (idx, type_ref) in parsed_pdb.ipi_types.iter() {
        let ty = type_ref.borrow();

        if let ezpdb::type_info::Type::BuildInfoType(build_info) = &*ty {
            found_count += 1;

            if found_count <= 5 {
                println!("\n   BuildInfo #{} at index 0x{:08x}:", found_count, idx);
                println!("   Contains {} items:\n", build_info.items.len());

                for (i, item) in build_info.items.iter().enumerate() {
                    let item_ty = item.borrow();

                    // Try to extract meaningful info based on type
                    match &*item_ty {
                        ezpdb::type_info::Type::StringId(string_id) => {
                            let preview = if string_id.id.len() > 80 {
                                format!("{}...", &string_id.id[..77])
                            } else {
                                string_id.id.clone()
                            };
                            println!("      [{}] StringId: \"{}\"", i, preview);
                        }
                        ezpdb::type_info::Type::Procedure(proc) => {
                            let ret_type = if let Some(ref ret) = proc.return_type {
                                let ret_ty = ret.borrow();
                                format!("{:?}", ret_ty)
                            } else {
                                "void".to_string()
                            };
                            let preview = if ret_type.len() > 60 {
                                format!("{}...", &ret_type[..57])
                            } else {
                                ret_type
                            };
                            println!("      [{}] Procedure: returns {}", i, preview);
                        }
                        ezpdb::type_info::Type::ArgumentList(args) => {
                            println!("      [{}] ArgumentList: {} args", i, args.0.len());
                        }
                        other => {
                            let type_str = format!("{:?}", other);
                            let variant =
                                type_str.split('(').next().unwrap_or("Unknown").to_string();
                            let preview = if type_str.len() > 70 {
                                format!("{}...", &type_str[..67])
                            } else {
                                type_str.clone()
                            };
                            println!("      [{}] {}: {}", i, variant, preview);
                        }
                    }
                }
            }
        }
    }

    println!("\n   Total BuildInfo types found: {}", found_count);

    // Display some FuncId samples
    println!("\n🔧 SAMPLE FUNCID TYPES FROM IPI STREAM:");
    let mut funcid_count = 0;

    for (idx, type_ref) in parsed_pdb.ipi_types.iter() {
        let ty = type_ref.borrow();

        if let ezpdb::type_info::Type::FuncId(func_id) = &*ty {
            funcid_count += 1;

            if funcid_count <= 5 {
                println!("\n   FuncId at 0x{:08x}:", idx);
                println!("      name: \"{}\"", func_id.name);

                if let Some(ref parent_scope) = func_id.parent_scope {
                    let scope_ty = parent_scope.borrow();
                    match &*scope_ty {
                        ezpdb::type_info::Type::StringId(s) => {
                            println!("      parent_scope: \"{}\"", s.id);
                        }
                        _ => {
                            println!("      parent_scope: {:?}", scope_ty);
                        }
                    }
                }
            }
        }
    }

    println!("\n   Total FuncId types found: {}", funcid_count);

    // Summary statistics
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("                      SUMMARY");
    println!("═══════════════════════════════════════════════════════════════\n");

    println!("IPI Stream Statistics:");
    let mut type_counts = std::collections::HashMap::new();
    for type_ref in parsed_pdb.ipi_types.values() {
        let ty = type_ref.borrow();
        let type_name = format!("{:?}", ty);
        let variant = type_name.split('(').next().unwrap_or("Unknown");
        *type_counts.entry(variant.to_string()).or_insert(0) += 1;
    }

    let mut types_vec: Vec<_> = type_counts.iter().collect();
    types_vec.sort_by_key(|(_, count)| std::cmp::Reverse(**count));

    for (type_name, count) in types_vec.iter().take(10) {
        println!("   {}: {}", type_name, count);
    }

    println!("\n✅ Verification complete!");
    println!(
        "   - Build info symbols: {}",
        if parsed_pdb.assembly_info.build_info.is_some() {
            "Present"
        } else {
            "Not found"
        }
    );
    println!("   - BuildInfo IPI types: {}", found_count);
    println!("   - FuncId IPI types: {}", funcid_count);
    println!("   - Total IPI types: {}", parsed_pdb.ipi_types.len());
    println!("   - Total TPI types: {}", parsed_pdb.types.len());

    println!("\n═══════════════════════════════════════════════════════════════\n");

    Ok(())
}
