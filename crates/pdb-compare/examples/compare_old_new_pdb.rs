/// Compare OLD pdb crate vs NEW ms-pdb parsing for the suspicious symbols
/// This will help us determine if WE are messing up or if it's really a PDB issue

use std::path::Path;
use anyhow::Result;

fn main() -> Result<()> {
    let pdb_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "cache_test_pdbs/ntdll.pdb".to_string());

    println!("═══════════════════════════════════════════════════════════════");
    println!("   COMPARING OLD PDB CRATE VS NEW MS-PDB PARSING");
    println!("═══════════════════════════════════════════════════════════════\n");
    println!("PDB File: {}\n", pdb_path);

    // Parse with OLD pdb crate (via ezpdb-old)
    println!("📦 Parsing with OLD pdb crate (pdb 0.8)...");
    let old_pdb = match ezpdb_old::parse_pdb(Path::new(&pdb_path), None) {
        Ok(pdb) => pdb,
        Err(e) => {
            eprintln!("❌ Failed to parse with old pdb crate: {}", e);
            return Ok(());
        }
    };
    
    // Parse with NEW ms-pdb (via ezpdb)
    println!("📦 Parsing with NEW ms-pdb...\n");
    let new_pdb = ezpdb::parse_pdb(Path::new(&pdb_path), None)?;
    
    println!("✅ Both parsers succeeded!\n");
    
    println!("═══════════════════════════════════════════════════════════════");
    println!("           ANALYZING SUSPICIOUS SYMBOLS");
    println!("═══════════════════════════════════════════════════════════════\n");
    
    // List of symbols that NEW parser thinks have IPI types
    let suspicious_symbols = vec![
        "LdrpMrdataLock",
        "RtlpDynamicFunctionTableLock",
        "LdrpInvertedFunctionTableSRWLock",
        "MuiCacheSWRLock",
        "RtlpMemoryZoneLock",
        "RtlpWnfInitOnce",
        "LdrpInitOnceLoadAsDataCrits",
    ];
    
    for symbol_name in suspicious_symbols {
        println!("═══════════════════════════════════════════════════════════════");
        println!("Symbol: {}", symbol_name);
        println!("═══════════════════════════════════════════════════════════════");
        
        // Find in OLD pdb
        let old_symbol = old_pdb.global_data.iter()
            .find(|s| s.name == symbol_name);
        
        // Find in NEW pdb
        let new_symbol = new_pdb.global_data.iter()
            .find(|s| s.name == symbol_name);
        
        match (old_symbol, new_symbol) {
            (Some(old), Some(new)) => {
                println!("\n📊 OLD PDB CRATE (pdb 0.8):");
                let old_ty = old.ty.borrow();
                let old_type_str = format!("{:?}", old_ty);
                
                // Show first few lines
                for (i, line) in old_type_str.lines().take(5).enumerate() {
                    if i == 0 {
                        println!("   Type: {}", line);
                    } else {
                        println!("        {}", line);
                    }
                }
                if old_type_str.lines().count() > 5 {
                    println!("        ... ({} more lines)", old_type_str.lines().count() - 5);
                }
                
                // Check if it's an IPI type
                let old_is_ipi = old_type_str.contains("FuncId") 
                    || old_type_str.contains("UdtSrcLine")
                    || old_type_str.contains("StringId");
                
                if old_is_ipi {
                    println!("   ⚠️  OLD PARSER ALSO GOT IPI TYPE!");
                } else {
                    println!("   ✅ OLD PARSER GOT PROPER TPI TYPE");
                }
                
                println!("\n📊 NEW MS-PDB:");
                let new_ty = new.ty.borrow();
                let new_type_str = format!("{:?}", new_ty);
                
                for (i, line) in new_type_str.lines().take(5).enumerate() {
                    if i == 0 {
                        println!("   Type: {}", line);
                    } else {
                        println!("        {}", line);
                    }
                }
                if new_type_str.lines().count() > 5 {
                    println!("        ... ({} more lines)", new_type_str.lines().count() - 5);
                }
                
                let new_is_ipi = new_type_str.contains("FuncId") 
                    || new_type_str.contains("UdtSrcLine")
                    || new_type_str.contains("StringId");
                
                if new_is_ipi {
                    println!("   ⚠️  NEW PARSER GOT IPI TYPE");
                } else {
                    println!("   ✅ NEW PARSER GOT PROPER TPI TYPE");
                }
                
                println!("\n🔍 COMPARISON:");
                if old_is_ipi && new_is_ipi {
                    println!("   ❌ BOTH parsers got IPI types - This IS a PDB issue!");
                } else if !old_is_ipi && new_is_ipi {
                    println!("   🚨 OLD parser got it RIGHT, NEW parser got it WRONG!");
                    println!("   🐛 BUG IN MS-PDB OR EZPDB!");
                } else if old_is_ipi && !new_is_ipi {
                    println!("   ✅ NEW parser FIXED something OLD parser got wrong!");
                } else {
                    println!("   ✅ Both parsers agree - proper TPI type");
                }
                
                // Try to get actual type name
                if !old_is_ipi {
                    if let Some(start) = old_type_str.find("name: \"") {
                        let rest = &old_type_str[start + 7..];
                        if let Some(end) = rest.find('"') {
                            let type_name = &rest[..end];
                            println!("   📝 Actual type name: {}", type_name);
                        }
                    }
                }
                
            },
            (None, Some(_)) => {
                println!("\n⚠️  Symbol only found in NEW parser");
            },
            (Some(_), None) => {
                println!("\n⚠️  Symbol only found in OLD parser");
            },
            (None, None) => {
                println!("\n❌ Symbol not found in either parser!");
            }
        }
        
        println!();
    }
    
    println!("═══════════════════════════════════════════════════════════════");
    println!("                        SUMMARY");
    println!("═══════════════════════════════════════════════════════════════\n");
    
    // Count how many have IPI types in each
    let old_ipi_count = old_pdb.global_data.iter()
        .filter(|s| {
            let ty = s.ty.borrow();
            let type_str = format!("{:?}", ty);
            type_str.contains("FuncId") || type_str.contains("UdtSrcLine")
        })
        .count();
    
    let new_ipi_count = new_pdb.global_data.iter()
        .filter(|s| {
            let ty = s.ty.borrow();
            let type_str = format!("{:?}", ty);
            type_str.contains("FuncId") || type_str.contains("UdtSrcLine")
        })
        .count();
    
    println!("OLD pdb crate: {} data symbols with IPI types", old_ipi_count);
    println!("NEW ms-pdb:    {} data symbols with IPI types", new_ipi_count);
    
    if old_ipi_count == new_ipi_count && new_ipi_count > 0 {
        println!("\n✅ BOTH parsers have the SAME number of IPI types!");
        println!("   This strongly suggests it's a PDB generation issue,");
        println!("   not a bug in our parsing code.");
    } else if old_ipi_count == 0 && new_ipi_count > 0 {
        println!("\n🚨 OLD parser has ZERO IPI types, NEW has {}!", new_ipi_count);
        println!("   This means WE ARE DOING SOMETHING WRONG!");
        println!("   The old pdb crate was handling this correctly.");
    } else if old_ipi_count > 0 && new_ipi_count == 0 {
        println!("\n✅ NEW parser FIXED the issue that OLD parser had!");
    } else {
        println!("\n🤔 Different counts - need more investigation");
        println!("   Difference: {} symbols", (old_ipi_count as i32 - new_ipi_count as i32).abs());
    }
    
    println!("\n═══════════════════════════════════════════════════════════════\n");
    
    Ok(())
}
