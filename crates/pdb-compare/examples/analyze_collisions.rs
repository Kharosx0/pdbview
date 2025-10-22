/// Check what types of records exist at colliding indices
use std::path::Path;
use ezpdb::type_info::Type;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdb_path = std::env::args()
        .nth(1)
        .expect("Usage: analyze_collisions <pdb_path>");

    println!("Parsing PDB: {}", pdb_path);
    let parsed_pdb = ezpdb::parse_pdb(Path::new(&pdb_path), None)?;

    println!("Checking for index collisions and their types...\n");
    
    let mut collision_count = 0;
    let mut same_type_count = 0;
    let mut different_type_count = 0;
    
    for (idx, tpi_type) in parsed_pdb.types.iter() {
        if let Some(ipi_type) = parsed_pdb.ipi_types.get(idx) {
            collision_count += 1;
            
            let tpi_borrowed = tpi_type.borrow();
            let ipi_borrowed = ipi_type.borrow();
            
            // Check if they're the same type variant
            let tpi_discriminant = std::mem::discriminant(&*tpi_borrowed);
            let ipi_discriminant = std::mem::discriminant(&*ipi_borrowed);
            
            if tpi_discriminant == ipi_discriminant {
                same_type_count += 1;
                if collision_count <= 5 {
                    println!("Index 0x{:08x}: SAME type variant", idx);
                    println!("  TPI: {:?}", format!("{:?}", &*tpi_borrowed).lines().next().unwrap_or(""));
                    println!("  IPI: {:?}", format!("{:?}", &*ipi_borrowed).lines().next().unwrap_or(""));
                    println!();
                }
            } else {
                different_type_count += 1;
                if different_type_count <= 10 {
                    println!("Index 0x{:08x}: DIFFERENT type variants!", idx);
                    println!("  TPI: {:?}", format!("{:?}", &*tpi_borrowed).lines().next().unwrap_or(""));
                    println!("  IPI: {:?}", format!("{:?}", &*ipi_borrowed).lines().next().unwrap_or(""));
                    
                    // Check if IPI is one of the IPI-only types
                    match &*ipi_borrowed {
                        Type::FuncId(_) => println!("  → IPI is FuncId (IPI-only type)"),
                        Type::MFuncId(_) => println!("  → IPI is MFuncId (IPI-only type)"),
                        Type::StringId(_) => println!("  → IPI is StringId (IPI-only type)"),
                        Type::SubStrList(_) => println!("  → IPI is SubStrList (IPI-only type)"),
                        Type::BuildInfoType(_) => println!("  → IPI is BuildInfo (IPI-only type)"),
                        Type::UdtSrcLineType(_) => println!("  → IPI is UdtSrcLine (IPI-only type)"),
                        _ => println!("  → IPI is NOT an IPI-only type (collision error!)"),
                    }
                    println!();
                }
            }
        }
    }
    
    println!("=== SUMMARY ===");
    println!("Total collisions: {}", collision_count);
    println!("Same type variant: {} ({:.1}%)", same_type_count, (same_type_count as f64 / collision_count as f64) * 100.0);
    println!("Different type variant: {} ({:.1}%)", different_type_count, (different_type_count as f64 / collision_count as f64) * 100.0);
    
    if different_type_count > 0 {
        println!("\n⚠️ Found {} collisions with DIFFERENT types - this explains the problem!", different_type_count);
        println!("We need to determine which stream based on the record type, not just check TPI first!");
    }
    
    Ok(())
}
