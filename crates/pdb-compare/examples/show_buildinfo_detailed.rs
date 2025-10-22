/// Detailed BuildInfo viewer - shows complete contents without truncation
use anyhow::Result;
use std::path::Path;

fn main() -> Result<()> {
    let pdb_path = std::env::args().nth(1).unwrap_or_else(|| {
        "./crates/ms-pdb/target/debug/build/anyhow-11a718d5af00b923/build_script_build.pdb"
            .to_string()
    });

    println!("═══════════════════════════════════════════════════════════════");
    println!("           DETAILED BUILDINFO CONTENTS VIEWER");
    println!("═══════════════════════════════════════════════════════════════\n");
    println!("PDB File: {}\n", pdb_path);

    // Parse with ezpdb
    let parsed_pdb = ezpdb::parse_pdb(Path::new(&pdb_path), None)?;

    println!("📋 BUILD INFO FROM SYMBOLS (Parsed from S_BUILDINFO record):\n");
    if let Some(ref build_info) = parsed_pdb.assembly_info.build_info {
        println!("   Found {} arguments:\n", build_info.arguments.len());

        let labels = [
            "Current Directory",
            "Compiler Path",
            "Source File",
            "PDB Path",
            "Command Line Arguments",
        ];

        for (i, arg) in build_info.arguments.iter().enumerate() {
            let label = labels.get(i).unwrap_or(&"Extra Argument");
            println!("   ┌─ [{}] {}:", i, label);

            if arg.is_empty() {
                println!("   │  <empty string>");
            } else {
                // Print long strings with wrapping
                let max_width = 100;
                let mut pos = 0;
                while pos < arg.len() {
                    let end = (pos + max_width).min(arg.len());
                    println!("   │  {}", &arg[pos..end]);
                    pos = end;
                }
            }
            println!("   └─");
            println!();
        }
    } else {
        println!("   ⚠️  No build info found in symbol records\n");
    }

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("🔨 BUILD INFO TYPES FROM IPI STREAM (LF_BUILDINFO records):\n");

    let mut buildinfo_count = 0;

    for (idx, type_ref) in parsed_pdb.ipi_types.iter() {
        let ty = type_ref.borrow();

        if let ezpdb::type_info::Type::BuildInfoType(build_info) = &*ty {
            buildinfo_count += 1;

            println!("┌─────────────────────────────────────────────────────────────────");
            println!(
                "│ BuildInfo #{} at IPI index 0x{:08x}",
                buildinfo_count, idx
            );
            println!(
                "│ Contains {} items (should all be StringIds):",
                build_info.items.len()
            );
            println!("└─────────────────────────────────────────────────────────────────\n");

            let labels = [
                "Current Working Directory",
                "Compiler Executable Path",
                "Source File Path",
                "PDB File Path",
                "Full Command Line",
            ];

            for (i, item) in build_info.items.iter().enumerate() {
                let item_ty = item.borrow();
                let label = labels.get(i).unwrap_or(&"Extra Item");

                print!("   [{}] {}: ", i, label);

                match &*item_ty {
                    ezpdb::type_info::Type::StringId(string_id) => {
                        println!("StringId");
                        if string_id.id.is_empty() {
                            println!("       Value: <empty string>");
                        } else {
                            println!("       Value:");
                            // Wrap long strings
                            let max_width = 90;
                            let mut pos = 0;
                            while pos < string_id.id.len() {
                                let end = (pos + max_width).min(string_id.id.len());
                                println!("       {}", &string_id.id[pos..end]);
                                pos = end;
                            }
                        }

                        if let Some(ref substr) = string_id.substring {
                            let substr_ty = substr.borrow();
                            println!("       Substring: {:?}", substr_ty);
                        }
                    }
                    other => {
                        // This should NOT happen after our fix!
                        let type_str = format!("{:?}", other);
                        let variant = type_str.split('(').next().unwrap_or("Unknown");
                        println!("⚠️  UNEXPECTED TYPE: {}", variant);
                        println!("       THIS IS A BUG! BuildInfo should only contain StringIds!");
                        let preview = if type_str.len() > 100 {
                            format!("{}...", &type_str[..97])
                        } else {
                            type_str
                        };
                        println!("       {}", preview);
                    }
                }
                println!();
            }

            println!();
        }
    }

    println!("═══════════════════════════════════════════════════════════════");
    println!("                      SUMMARY");
    println!("═══════════════════════════════════════════════════════════════\n");

    println!(
        "Total BuildInfo types found in IPI stream: {}",
        buildinfo_count
    );

    if buildinfo_count == 0 {
        println!("⚠️  No BuildInfo types found - this PDB may not contain build information");
    } else {
        println!("✅ Found {} BuildInfo records", buildinfo_count);

        // Verify all items are StringIds
        let mut all_correct = true;
        for type_ref in parsed_pdb.ipi_types.values() {
            let ty = type_ref.borrow();
            if let ezpdb::type_info::Type::BuildInfoType(build_info) = &*ty {
                for item in &build_info.items {
                    let item_ty = item.borrow();
                    if !matches!(&*item_ty, ezpdb::type_info::Type::StringId(_)) {
                        all_correct = false;
                        break;
                    }
                }
            }
        }

        if all_correct {
            println!("✅ All BuildInfo items are StringIds (correct!)");
        } else {
            println!("❌ Some BuildInfo items are NOT StringIds (BUG!)");
        }
    }

    println!("\n═══════════════════════════════════════════════════════════════\n");

    Ok(())
}
