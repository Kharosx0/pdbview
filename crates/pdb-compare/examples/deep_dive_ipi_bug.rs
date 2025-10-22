/// Deep forensic analysis of IPI types appearing in data symbols

use std::path::Path;
use anyhow::Result;

fn main() -> Result<()> {
    let pdb_path = std::env::args()
        .nth(1)
        .expect("Usage: deep_dive_ipi_bug <pdb_path>");

    println!("═══════════════════════════════════════════════════════════════");
    println!("         DEEP DIVE: IPI Types in Data Symbols Bug");
    println!("═══════════════════════════════════════════════════════════════\n");
    println!("PDB File: {}\n", pdb_path);

    // Use ezpdb for parsing
    let parsed_pdb = ezpdb::parse_pdb(Path::new(&pdb_path), None)?;
    
    println!("📊 PARSED PDB STATISTICS:");
    println!("   Total types in TPI: {}", parsed_pdb.types.len());
    println!("   Total types in IPI: {}", parsed_pdb.ipi_types.len());
    println!("   Total global data symbols: {}", parsed_pdb.global_data.len());
    
    println!("\n🔍 ANALYZING DATA SYMBOLS WITH IPI TYPES...\n");
    
    let mut ipi_data_symbols = Vec::new();
    let mut tpi_data_symbols = 0;
    let mut unknown_symbols = 0;
    
    for data_symbol in parsed_pdb.global_data.iter() {
        let ty = data_symbol.ty.borrow();
        let type_name = format!("{:?}", ty);
        
        // Check if this is an IPI type
        let is_ipi = type_name.contains("FuncId") 
            || type_name.contains("MFuncId") 
            || type_name.contains("StringId") 
            || type_name.contains("SubStrList") 
            || type_name.contains("BuildInfo")
            || type_name.contains("UdtSrcLine");
        
        if is_ipi {
            ipi_data_symbols.push((data_symbol.name.clone(), type_name));
        } else if type_name.contains("Primitive") || type_name.contains("Class") || type_name.contains("Union") {
            tpi_data_symbols += 1;
        } else {
            unknown_symbols += 1;
        }
    }
    
    println!("═══════════════════════════════════════════════════════════════");
    println!("                        SUMMARY");
    println!("═══════════════════════════════════════════════════════════════\n");
    println!("Data symbols with TPI types: {}", tpi_data_symbols);
    println!("Data symbols with IPI types: {}", ipi_data_symbols.len());
    println!("Data symbols with unknown types: {}", unknown_symbols);
    
    let total = tpi_data_symbols + ipi_data_symbols.len() + unknown_symbols;
    if !ipi_data_symbols.is_empty() {
        let percentage = (ipi_data_symbols.len() as f64 / total as f64) * 100.0;
        println!("\n⚠️  {:.1}% of data symbols have IPI type indices!", percentage);
    }
    
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("           DETAILED EXAMPLES (first 15)");
    println!("═══════════════════════════════════════════════════════════════\n");
    
    for (i, (name, type_str)) in ipi_data_symbols.iter().take(15).enumerate() {
        println!("#{}: {}", i + 1, name);
        
        // Parse type kind from debug string
        let type_kind = if type_str.contains("FuncId") {
            "FuncId (Function ID)"
        } else if type_str.contains("UdtSrcLine") {
            "UdtSrcLine (Source Line Info)"
        } else if type_str.contains("StringId") {
            "StringId (String Literal)"
        } else if type_str.contains("BuildInfo") {
            "BuildInfo (Compiler Info)"
        } else {
            "Other IPI type"
        };
        
        println!("   Type: {}", type_kind);
        
        // Extract function name for FuncId
        if let Some(start) = type_str.find("name: \"") {
            let rest = &type_str[start + 7..];
            if let Some(end) = rest.find('"') {
                let func_name = &rest[..end];
                println!("   Function: {}", func_name);
            }
        }
        
        // Show first line of type debug
        if let Some(first_line) = type_str.lines().next() {
            println!("   Debug: {}...", &first_line[..first_line.len().min(80)]);
        }
        
        println!();
    }
    
    if ipi_data_symbols.len() > 15 {
        println!("... and {} more\n", ipi_data_symbols.len() - 15);
    }
    
    // Analysis of patterns
    println!("═══════════════════════════════════════════════════════════════");
    println!("                      PATTERN ANALYSIS");
    println!("═══════════════════════════════════════════════════════════════\n");
    
    let mut funcid_count = 0;
    let mut udtsrcline_count = 0;
    let mut other_count = 0;
    let mut unique_funcnames = std::collections::HashSet::new();
    
    for (_name, type_str) in &ipi_data_symbols {
        if type_str.contains("FuncId") {
            funcid_count += 1;
            // Extract function name
            if let Some(start) = type_str.find("name: \"") {
                let rest = &type_str[start + 7..];
                if let Some(end) = rest.find('"') {
                    unique_funcnames.insert(rest[..end].to_string());
                }
            }
        } else if type_str.contains("UdtSrcLine") {
            udtsrcline_count += 1;
        } else {
            other_count += 1;
        }
    }
    
    println!("IPI Type Breakdown:");
    println!("   FuncId: {} symbols", funcid_count);
    println!("   UdtSrcLine: {} symbols", udtsrcline_count);
    println!("   Other: {} symbols", other_count);
    
    println!("\nUnique function names in FuncId: {}", unique_funcnames.len());
    println!("Function names:");
    for (i, name) in unique_funcnames.iter().take(20).enumerate() {
        println!("   {}. {}", i + 1, name);
    }
    if unique_funcnames.len() > 20 {
        println!("   ... and {} more", unique_funcnames.len() - 20);
    }
    
    println!("\n═══════════════════════════════════════════════════════════════\n");
    
    Ok(())
}
