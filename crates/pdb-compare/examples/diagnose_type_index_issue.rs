/// Diagnose the root cause: Do these type indices exist in both TPI and IPI?

use std::path::Path;
use std::fs::File;
use anyhow::Result;
use ms_pdb::{Pdb, codeview::types::TypeIndex};

fn main() -> Result<()> {
    let pdb_path = std::env::args()
        .nth(1)
        .expect("Usage: diagnose_type_index_issue <pdb_path>");

    println!("═══════════════════════════════════════════════════════════════");
    println!("    DIAGNOSING TYPE INDEX ISSUE: TPI vs IPI Resolution");
    println!("═══════════════════════════════════════════════════════════════\n");
    println!("PDB File: {}\n", pdb_path);

    // Open PDB with ms-pdb
    let file = File::open(&pdb_path)?;
    let pdb = Pdb::open_from_file(file)?;
    
    // Get type streams
    let type_stream = pdb.read_type_stream()?;
    let ipi_stream = pdb.read_ipi_stream()?;
    
    println!("📊 STREAM INFO:");
    println!("   TPI range: 0x{:08x} - 0x{:08x} ({} types)", 
        type_stream.type_index_begin().0,
        type_stream.type_index_end().0,
        type_stream.num_types());
    println!("   IPI range: 0x{:08x} - 0x{:08x} ({} types)", 
        ipi_stream.type_index_begin().0,
        ipi_stream.type_index_end().0,
        ipi_stream.num_types());
    
    // Read symbol stream to get data symbol type indices
    let gss = pdb.read_gss()?;
    
    println!("\n🔍 ANALYZING DATA SYMBOL TYPE INDICES...\n");
    
    let mut symbols_with_collisions = Vec::new();
    let mut symbols_ipi_only = Vec::new();
    let mut symbols_tpi_only = Vec::new();
    
    for sym in gss.iter_syms() {
        if let Ok(ms_pdb::codeview::syms::SymData::Data(data)) = sym.parse() {
            let type_idx = data.header.type_.get();
            
            // Check if this index exists in TPI
            let in_tpi = type_idx.0 >= type_stream.type_index_begin().0 
                && type_idx.0 < type_stream.type_index_end().0;
            
            // Check if this index exists in IPI
            let in_ipi = type_idx.0 >= ipi_stream.type_index_begin().0 
                && type_idx.0 < ipi_stream.type_index_end().0;
            
            if in_tpi && in_ipi {
                // Collision - exists in BOTH streams
                symbols_with_collisions.push((data.name.to_string(), type_idx));
            } else if in_ipi && !in_tpi {
                // IPI only
                symbols_ipi_only.push((data.name.to_string(), type_idx));
            } else if in_tpi && !in_ipi {
                // TPI only (normal case)
                symbols_tpi_only.push((data.name.to_string(), type_idx));
            }
        }
    }
    
    println!("═══════════════════════════════════════════════════════════════");
    println!("                        RESULTS");
    println!("═══════════════════════════════════════════════════════════════\n");
    println!("Data symbols with type in TPI only (CORRECT): {}", symbols_tpi_only.len());
    println!("Data symbols with type in IPI only (BUG): {}", symbols_ipi_only.len());
    println!("Data symbols with type in BOTH TPI and IPI (COLLISION): {}", symbols_with_collisions.len());
    
    if !symbols_with_collisions.is_empty() {
        println!("\n🚨 COLLISIONS DETECTED!");
        println!("   These indices exist in BOTH TPI and IPI with different data.\n");
        
        for (name, type_idx) in symbols_with_collisions.iter().take(5) {
            println!("Symbol: {}", name);
            println!("  TypeIndex: 0x{:08x} ({})", type_idx.0, type_idx.0);
            
            // Show what's in TPI
            if let Ok(tpi_record) = type_stream.record(*type_idx) {
                if let Ok(tpi_data) = tpi_record.parse() {
                    println!("  TPI: {:?}", tpi_data);
                }
            }
            
            // Show what's in IPI
            if let Ok(ipi_record) = ipi_stream.record(*type_idx) {
                if let Ok(ipi_data) = ipi_record.parse() {
                    println!("  IPI: {:?}", ipi_data);
                }
            }
            println!();
        }
        
        if symbols_with_collisions.len() > 5 {
            println!("... and {} more collisions\n", symbols_with_collisions.len() - 5);
        }
    }
    
    if !symbols_ipi_only.is_empty() {
        println!("\n❌ IPI-ONLY INDICES (These should NOT happen!)");
        println!("   These type indices DON'T exist in TPI, only in IPI.\n");
        
        for (name, type_idx) in symbols_ipi_only.iter().take(10) {
            println!("Symbol: {}", name);
            println!("  TypeIndex: 0x{:08x} ({})", type_idx.0, type_idx.0);
            
            // Show what's in IPI
            if let Ok(ipi_record) = ipi_stream.record(*type_idx) {
                if let Ok(ipi_data) = ipi_record.parse() {
                    let type_name = match ipi_data {
                        ms_pdb::codeview::types::TypeData::FuncId(ref f) => format!("FuncId({})", f.name),
                        ms_pdb::codeview::types::TypeData::UdtSrcLine(ref u) => format!("UdtSrcLine(line {})", u.line.get()),
                        _ => format!("{:?}", ipi_data),
                    };
                    println!("  IPI: {}", type_name);
                }
            }
            println!();
        }
        
        if symbols_ipi_only.len() > 10 {
            println!("... and {} more IPI-only symbols\n", symbols_ipi_only.len() - 10);
        }
    }
    
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("                      CONCLUSION");
    println!("═══════════════════════════════════════════════════════════════\n");
    
    if symbols_with_collisions.is_empty() && symbols_ipi_only.is_empty() {
        println!("✅ NO ISSUES DETECTED!");
        println!("   All data symbols correctly reference TPI types.");
    } else if !symbols_with_collisions.is_empty() {
        println!("🔧 SOLUTION: Index Collision");
        println!("   The type indices exist in BOTH TPI and IPI streams.");
        println!("   When resolving types for data symbols, always prefer TPI over IPI.");
        println!("   The old pdb crate likely tries TPI first, which is why it works.");
    } else if !symbols_ipi_only.is_empty() {
        println!("🔧 SOLUTION: IPI-Only Indices");
        println!("   The type indices ONLY exist in IPI, not in TPI.");
        println!("   This is a PDB generation bug - data symbols should never");
        println!("   reference IPI types. However, we need a fallback to IPI");
        println!("   for compatibility with buggy PDB files.");
    }
    
    println!("\n═══════════════════════════════════════════════════════════════\n");
    
    Ok(())
}
