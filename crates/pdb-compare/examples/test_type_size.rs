/// Test program to verify that calling type_size() on IPI types doesn't panic
use ezpdb::type_info::Typed;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdb_path = std::env::args()
        .nth(1)
        .expect("Usage: test_type_size <pdb_path>");

    println!("Parsing PDB: {}", pdb_path);
    let parsed_pdb = ezpdb::parse_pdb(Path::new(&pdb_path), None)?;
    
    println!("Found {} global data symbols", parsed_pdb.global_data.len());
    println!("Found {} types", parsed_pdb.types.len());
    println!("Found {} IPI types", parsed_pdb.ipi_types.len());
    
    let mut ipi_type_count = 0;
    let mut total_with_offset = 0;
    
    // This is the user's code pattern that was failing
    for data in parsed_pdb.global_data.iter() {
        if let Some(offset) = data.offset {
            total_with_offset += 1;
            
            // Try to get the size - this should NOT panic even for IPI types
            let borrowed_type = data.ty.borrow();
            let data_type_size = borrowed_type.type_size(&parsed_pdb);
            
            if data_type_size == 0 {
                // Check if this is an IPI type
                let type_name = format!("{:?}", data.ty.borrow());
                if type_name.contains("FuncId") 
                    || type_name.contains("MFuncId") 
                    || type_name.contains("StringId") 
                    || type_name.contains("SubStrList") 
                    || type_name.contains("BuildInfo")
                    || type_name.contains("UdtSrcLine") {
                    ipi_type_count += 1;
                    println!("  Data symbol '{}' at offset 0x{:x} has IPI type (size=0): {}", 
                        data.name, offset, type_name.lines().next().unwrap_or(""));
                }
            }
        }
    }
    
    println!("\nSummary:");
    println!("  Data symbols with offsets: {}", total_with_offset);
    println!("  Data symbols with IPI types: {}", ipi_type_count);
    
    if ipi_type_count > 0 {
        println!("\n✅ SUCCESS: IPI types returned size=0 instead of panicking!");
    } else {
        println!("\n✅ No IPI types found in data symbols (this is expected for most PDBs)");
    }
    
    Ok(())
}
