/// Investigate what these "data" symbols with IPI types actually represent
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdb_path = std::env::args()
        .nth(1)
        .expect("Usage: investigate_ipi_data <pdb_path>");

    println!("Parsing PDB: {}", pdb_path);
    let parsed_pdb = ezpdb::parse_pdb(Path::new(&pdb_path), None)?;
    
    println!("Analyzing data symbols with IPI types...\n");
    
    for data in parsed_pdb.global_data.iter() {
        if let Some(offset) = data.offset {
            let borrowed_type = data.ty.borrow();
            let type_name = format!("{:?}", borrowed_type);
            
            // Check if this is an IPI type
            if type_name.contains("FuncId") 
                || type_name.contains("MFuncId") 
                || type_name.contains("StringId") 
                || type_name.contains("SubStrList") 
                || type_name.contains("BuildInfo")
                || type_name.contains("UdtSrcLine") {
                
                println!("=== Symbol: {} ===", data.name);
                println!("  Offset: 0x{:x}", offset);
                println!("  Type: {}", type_name.lines().next().unwrap_or(""));
                
                // Look at the actual type details
                if type_name.contains("FuncId") {
                    println!("  ⚠️  This is a FUNCTION ID - not a data type!");
                    println!("  Question: Why is a 'data symbol' pointing to a function?");
                } else if type_name.contains("UdtSrcLine") {
                    println!("  📝 This is source line info for a UDT");
                } else if type_name.contains("StringId") {
                    println!("  📝 This is a string literal");
                } else if type_name.contains("SubStrList") {
                    println!("  📝 This is a string list");
                } else if type_name.contains("BuildInfo") {
                    println!("  📝 This is build/compiler info");
                }
                
                // Try to extract more details from the type
                let type_details = format!("{:#?}", borrowed_type);
                let lines: Vec<&str> = type_details.lines().take(10).collect();
                println!("  First 10 lines of type data:");
                for line in lines {
                    println!("    {}", line);
                }
                println!();
            }
        }
    }
    
    Ok(())
}
