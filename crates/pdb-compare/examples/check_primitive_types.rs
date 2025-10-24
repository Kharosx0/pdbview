use pdb_compare::{new_pdb, old_pdb};
use std::path::PathBuf;

fn main() {
    let pdb_path = PathBuf::from("cache_test_pdbs/ntkrnlmp.pdb");

    println!("Parsing PDB with old parser...");
    let old_data = old_pdb::parse_pdb(&pdb_path).expect("Failed to parse with old parser");

    println!("Parsing PDB with new parser...");
    let new_data = new_pdb::parse_pdb_new(&pdb_path).expect("Failed to parse with new parser");

    // Find the problematic type
    let type_name = "_RTLP_CPU_FEATURE_ENTRY";

    println!("\nLooking for ALL instances of type: {}", type_name);

    // Find ALL instances in old parser
    let old_types: Vec<_> = old_data
        .types
        .iter()
        .filter(|t| t.name.as_ref().map(|n| n.as_str()) == Some(type_name))
        .collect();

    println!("\n=== OLD PARSER - Found {} instances ===", old_types.len());
    for old_type in &old_types {
        println!("\nType {} (index {}):", type_name, old_type.index);
        println!("  Kind: {}", old_type.kind);
        println!("  Fields: {}", old_type.fields.len());

        for field in &old_type.fields {
            println!("    - {} @ offset {:?}", field.name, field.offset);
        }
    }

    // Find ALL instances in new parser
    let new_types: Vec<_> = new_data
        .types
        .iter()
        .filter(|t| t.name.as_ref().map(|n| n.as_str()) == Some(type_name))
        .collect();

    println!("\n=== NEW PARSER - Found {} instances ===", new_types.len());
    for new_type in &new_types {
        println!("\nType {} (index {}):", type_name, new_type.index);
        println!("  Kind: {}", new_type.kind);
        println!("  Fields: {}", new_type.fields.len());

        for field in &new_type.fields {
            println!("    - {} @ offset {:?}", field.name, field.offset);
        }
    }

    // Find fullest definitions
    let old_fullest = old_types.iter().max_by_key(|t| t.fields.len());
    let new_fullest = new_types.iter().max_by_key(|t| t.fields.len());

    println!("\n=== FULLEST DEFINITIONS ===");
    if let Some(old_type) = old_fullest {
        println!(
            "Old parser fullest: index {} with {} fields",
            old_type.index,
            old_type.fields.len()
        );
    }
    if let Some(new_type) = new_fullest {
        println!(
            "New parser fullest: index {} with {} fields",
            new_type.index,
            new_type.fields.len()
        );
    }
}
