/// Check the actual TypeIndex values to see if high bit is set for IPI types
use ezpdb::type_info::Typed;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdb_path = std::env::args()
        .nth(1)
        .expect("Usage: check_type_indices <pdb_path>");

    println!("Parsing PDB: {}", pdb_path);
    let parsed_pdb = ezpdb::parse_pdb(Path::new(&pdb_path), None)?;
    
    println!("TPI types: {}", parsed_pdb.types.len());
    println!("IPI types: {}", parsed_pdb.ipi_types.len());
    
    // Show a few type indices from each collection
    println!("\nSample TPI type indices:");
    for (idx, _) in parsed_pdb.types.iter().take(5) {
        println!("  0x{:08x} (high_bit={})", idx, (idx & 0x80000000) != 0);
    }
    
    println!("\nSample IPI type indices:");
    for (idx, _) in parsed_pdb.ipi_types.iter().take(5) {
        println!("  0x{:08x} (high_bit={})", idx, (idx & 0x80000000) != 0);
    }
    
    println!("\nChecking data symbols for IPI types:");
    let mut count = 0;
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
                println!("  Found IPI type in data symbol '{}'", data.name);
                count += 1;
                if count >= 10 {
                    break;
                }
            }
        }
    }
    
    Ok(())
}
