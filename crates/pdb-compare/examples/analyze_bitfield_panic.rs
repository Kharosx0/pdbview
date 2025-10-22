/// This program analyzes the bitfield panic in ms-pdb to understand what values are causing it
use std::panic::{catch_unwind, AssertUnwindSafe};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <pdb_file>", args[0]);
        std::process::exit(1);
    }

    let pdb_path = &args[1];
    println!("Analyzing bitfield panics in: {}\n", pdb_path);

    // We'll use ezpdb but directly access the parsed PDB to inspect UdtProperties
    // For now, let's just trigger the parsing and see the error
    println!("Attempting to parse with ezpdb...\n");
    let result = ezpdb::parse_pdb(pdb_path);
    
    match result {
        Ok(_pdb) => {
            println!("\nPDB parsed successfully without visible panics!");
        }
        Err(e) => {
            println!("\nPDB parsing failed: {}", e);
        }
    }
    
    // Now let's directly use ms-pdb to inspect the raw properties
    println!("\n=== Direct ms-pdb inspection ===\n");
    
    let file = std::fs::File::open(pdb_path)?;
    let mut pdb = ms_pdb::PDB::open(file)?;
    
    // Get TPI stream
    let tpi = pdb.type_information()?;
    let mut iter = tpi.iter();
    
    let mut panic_count = 0;
    let mut success_count = 0;
    let mut problematic_types = Vec::new();
    
    println!("Scanning all types for bitfield panics...\n");
    
    while let Some(typ) = iter.next()? {
        let index = typ.index();
        
        // Try to access the type data
        match typ.parse() {
            Ok(data) => {
                // Check if this is a type with UdtProperties
                match data {
                    Type::Class(ref class_type) => {
                        let props = class_type.properties;
                        
                        // Try to call each bitfield accessor
                        let hfa_result = catch_unwind(AssertUnwindSafe(|| props.hfa()));
                        let mocom_result = catch_unwind(AssertUnwindSafe(|| props.mocom()));
                        
                        if hfa_result.is_err() || mocom_result.is_err() {
                            panic_count += 1;
                            problematic_types.push((
                                index,
                                "Class",
                                class_type.name.as_str(),
                                props,
                                hfa_result.is_err(),
                                mocom_result.is_err()
                            ));
                            
                            if panic_count <= 10 {
                                println!("PANIC #{}: Class '{}' (index: 0x{:x})", 
                                    panic_count, class_type.name.as_str(), index.0);
                                println!("  Raw properties value: 0x{:04x}", props.0);
                                println!("  HFA panic: {}, MOCOM panic: {}", 
                                    hfa_result.is_err(), mocom_result.is_err());
                                
                                // Try to decode the bitfield manually to understand what's wrong
                                let raw = props.0;
                                println!("  Binary: {:016b}", raw);
                                println!("  Trying to extract HFA field (bits 11-12)...");
                                println!("  Trying to extract MOCOM field (bits 14-15)...");
                                println!();
                            }
                        } else {
                            success_count += 1;
                        }
                    }
                    Type::Structure(ref struct_type) => {
                        let props = struct_type.properties;
                        
                        let hfa_result = catch_unwind(AssertUnwindSafe(|| props.hfa()));
                        let mocom_result = catch_unwind(AssertUnwindSafe(|| props.mocom()));
                        
                        if hfa_result.is_err() || mocom_result.is_err() {
                            panic_count += 1;
                            problematic_types.push((
                                index,
                                "Structure",
                                struct_type.name.as_str(),
                                props,
                                hfa_result.is_err(),
                                mocom_result.is_err()
                            ));
                            
                            if panic_count <= 10 {
                                println!("PANIC #{}: Structure '{}' (index: 0x{:x})", 
                                    panic_count, struct_type.name.as_str(), index.0);
                                println!("  Raw properties value: 0x{:04x}", props.0);
                                println!("  HFA panic: {}, MOCOM panic: {}", 
                                    hfa_result.is_err(), mocom_result.is_err());
                                println!("  Binary: {:016b}", props.0);
                                println!();
                            }
                        } else {
                            success_count += 1;
                        }
                    }
                    Type::Enum(ref enum_type) => {
                        let props = enum_type.properties;
                        
                        let hfa_result = catch_unwind(AssertUnwindSafe(|| props.hfa()));
                        let mocom_result = catch_unwind(AssertUnwindSafe(|| props.mocom()));
                        
                        if hfa_result.is_err() || mocom_result.is_err() {
                            panic_count += 1;
                            problematic_types.push((
                                index,
                                "Enum",
                                enum_type.name.as_str(),
                                props,
                                hfa_result.is_err(),
                                mocom_result.is_err()
                            ));
                            
                            if panic_count <= 10 {
                                println!("PANIC #{}: Enum '{}' (index: 0x{:x})", 
                                    panic_count, enum_type.name.as_str(), index.0);
                                println!("  Raw properties value: 0x{:04x}", props.0);
                                println!("  HFA panic: {}, MOCOM panic: {}", 
                                    hfa_result.is_err(), mocom_result.is_err());
                                println!("  Binary: {:016b}", props.0);
                                println!();
                            }
                        } else {
                            success_count += 1;
                        }
                    }
                    Type::Union(ref union_type) => {
                        let props = union_type.properties;
                        
                        let hfa_result = catch_unwind(AssertUnwindSafe(|| props.hfa()));
                        let mocom_result = catch_unwind(AssertUnwindSafe(|| props.mocom()));
                        
                        if hfa_result.is_err() || mocom_result.is_err() {
                            panic_count += 1;
                            problematic_types.push((
                                index,
                                "Union",
                                union_type.name.as_str(),
                                props,
                                hfa_result.is_err(),
                                mocom_result.is_err()
                            ));
                            
                            if panic_count <= 10 {
                                println!("PANIC #{}: Union '{}' (index: 0x{:x})", 
                                    panic_count, union_type.name.as_str(), index.0);
                                println!("  Raw properties value: 0x{:04x}", props.0);
                                println!("  HFA panic: {}, MOCOM panic: {}", 
                                    hfa_result.is_err(), mocom_result.is_err());
                                println!("  Binary: {:016b}", props.0);
                                println!();
                            }
                        } else {
                            success_count += 1;
                        }
                    }
                    _ => {
                        // Other types don't have UdtProperties
                    }
                }
            }
            Err(_e) => {
                // Skip types we can't parse
            }
        }
    }
    
    println!("\n=== SUMMARY ===");
    println!("Total types with UdtProperties that panicked: {}", panic_count);
    println!("Total types with UdtProperties that succeeded: {}", success_count);
    
    if !problematic_types.is_empty() {
        println!("\n=== ANALYZING PROBLEMATIC PROPERTY VALUES ===");
        
        // Collect all unique raw property values that caused panics
        let mut unique_values: Vec<u16> = problematic_types.iter()
            .map(|t| t.3.0)
            .collect();
        unique_values.sort_unstable();
        unique_values.dedup();
        
        println!("Unique problematic raw values:");
        for val in unique_values.iter().take(20) {
            println!("  0x{:04x} = {:016b}", val, val);
        }
    }
    
    Ok(())
}
