use anyhow::Result;
use clap::Parser;
use log::{error, info};
use std::path::PathBuf;

mod old_pdb;
mod new_pdb;
mod compare;

#[derive(Parser, Debug)]
#[command(name = "pdb-compare")]
#[command(about = "Compare PDB parsing between old ezpdb (pdb crate) and new ezpdb (ms-pdb)")]
struct Args {
    /// Path to the PDB file to analyze (can specify multiple times)
    #[arg(short, long)]
    pdb_file: Vec<PathBuf>,

    /// Output JSON comparison results
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Only run specific test categories (comma-separated: header,types,symbols,modules)
    #[arg(short, long)]
    categories: Option<String>,

    /// Run cache test with all available test PDBs
    #[arg(long)]
    cache_test: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    if args.verbose {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    // Handle cache test mode
    if args.cache_test {
        info!("Running cache test with multiple PDBs...");
        return run_cache_test();
    }

    // Validate that at least one PDB file is provided
    if args.pdb_file.is_empty() {
        error!("No PDB files specified. Use --pdb-file <path> or --cache-test");
        std::process::exit(1);
    }

    // Process each PDB file
    let mut all_output = String::new();
    let mut total_files = 0;
    let mut files_with_differences = 0;

    for pdb_path in &args.pdb_file {
        total_files += 1;
        info!("Starting PDB comparison: {}", pdb_path.display());

        // Parse which categories to test
        let categories = if let Some(ref cats) = args.categories {
            cats.split(',').map(|s| s.trim().to_string()).collect()
        } else {
            vec![
                "header".to_string(),
                "types".to_string(),
                "symbols".to_string(),
                "modules".to_string(),
            ]
        };

        // Parse with old pdb crate
        info!("Parsing with old 'pdb' crate...");
        let old_result = match old_pdb::parse_pdb(pdb_path) {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to parse with old pdb crate: {}", e);
                all_output.push_str(&format!("\n=== {} ===\n", pdb_path.display()));
                all_output.push_str(&format!("ERROR: Failed to parse with old pdb crate: {}\n", e));
                continue;
            }
        };
        info!("Old pdb crate parsed successfully");

        // Parse with new ezpdb crate
        info!("Parsing with new 'ezpdb' crate...");
        let new_result = match new_pdb::parse_pdb_new(pdb_path) {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to parse with new ezpdb crate: {}", e);
                all_output.push_str(&format!("\n=== {} ===\n", pdb_path.display()));
                all_output.push_str(&format!("ERROR: Failed to parse with new ezpdb crate: {}\n", e));
                continue;
            }
        };
        info!("New ezpdb crate parsed successfully");

        // Compare results
        info!("Comparing results...");
        let comparison = compare::compare_pdbs(&old_result, &new_result);

        // Build output text for this file
        let mut output = String::new();
        output.push_str(&"=".repeat(80));
        output.push_str("\n");
        output.push_str(&format!("=== PDB: {} ===\n", pdb_path.display()));
        output.push_str(&"=".repeat(80));
        output.push_str("\n\n");
        output.push_str(&format!("Categories tested: {}\n", categories.join(", ")));
        output.push_str(&format!("\nTotal differences: {}\n", comparison.summary.total_differences));
        output.push_str(&format!("Header matches: {}\n", comparison.summary.header_matches));
        output.push_str(&format!("Type count matches: {}\n", comparison.summary.type_count_matches));
        output.push_str(&format!("Symbol count matches: {}\n", comparison.summary.symbol_count_matches));
        output.push_str(&format!("Module count matches: {}\n", comparison.summary.module_count_matches));

        // Add detailed differences
        if !comparison.differences.is_empty() {
            files_with_differences += 1;
            output.push_str("\n=== Differences Found ===\n\n");
            for diff in &comparison.differences {
                output.push_str(&format!("  [{}] {}\n", diff.category, diff.description));
                if let Some(ref old) = diff.old_value {
                    output.push_str(&format!("    Old: {}\n", old));
                }
                if let Some(ref new) = diff.new_value {
                    output.push_str(&format!("    New: {}\n", new));
                }
                output.push_str("\n");
            }
        } else {
            output.push_str("\n✅ No differences found! Both implementations produce identical results.\n");
        }

        all_output.push_str(&output);
    }

    // Add summary header
    let mut final_output = String::new();
    final_output.push_str("=== PDB Comparison Summary ===\n\n");
    final_output.push_str(&format!("Total PDB files tested: {}\n", total_files));
    final_output.push_str(&format!("Files with differences: {}\n", files_with_differences));
    final_output.push_str(&format!("Files matching perfectly: {}\n", total_files - files_with_differences));
    final_output.push_str(&all_output);

    // Print to console
    println!("{}", final_output);

    // Always save text output to pdb_compare_results.txt
    let text_output_path = PathBuf::from("pdb_compare_results.txt");
    info!("Writing detailed results to {}", text_output_path.display());
    std::fs::write(&text_output_path, &final_output)?;
    println!("Detailed results written to: {}", text_output_path.display());

    // Output to JSON if requested
    if let Some(output_path) = args.output {
        info!("Writing JSON comparison results to {}", output_path.display());
        // For multiple files, we'd need to adjust the JSON structure, but for now just note it
        println!("Note: JSON output not fully implemented for multiple files yet");
    }

    // Exit with error code if differences found
    if files_with_differences > 0 {
        std::process::exit(1);
    }

    Ok(())
}

/// Run cache test with multiple predefined PDB files
fn run_cache_test() -> Result<()> {
    let test_pdbs = vec![
        PathBuf::from("E:\\pdbview\\target\\debug\\pdb_compare.pdb"),
        PathBuf::from("E:\\tmp\\jobs\\sk\\setup\\oft-setup-f3a331446f2e5bb88a5b307775d84e1c\\fakefs\\symbols\\ntdll.pdb\\569C1118589A215D0BA886504157B8AF1\\ntdll.pdb"),
        PathBuf::from("E:\\tmp\\jobs\\sk\\setup\\oft-setup-f3a331446f2e5bb88a5b307775d84e1c\\fakefs\\symbols\\hvix64.pdb\\BEB089CD3966319528A589D97B5DB4701\\hvix64.pdb"),
    ];

    info!("=== Cache Test Mode ===");
    info!("Testing cache functionality with {} PDB files", test_pdbs.len());
    
    let mut all_output = String::new();
    all_output.push_str("=== Cache Test Results ===\n\n");
    all_output.push_str(&format!("Testing {} PDB files to verify cache functionality\n", test_pdbs.len()));
    all_output.push_str("This test ensures section header cache works correctly across multiple PDBs\n\n");

    let mut successful_parses = 0;
    let mut failed_parses = 0;
    let mut total_offset_mismatches = 0;

    for (idx, pdb_path) in test_pdbs.iter().enumerate() {
        info!("\n[{}/{}] Processing: {}", idx + 1, test_pdbs.len(), pdb_path.display());
        all_output.push_str("\n");
        all_output.push_str(&"=".repeat(80));
        all_output.push_str("\n");
        all_output.push_str(&format!("[{}/{}] PDB: {}\n", idx + 1, test_pdbs.len(), pdb_path.display()));
        all_output.push_str(&"=".repeat(80));
        all_output.push_str("\n");

        // Check if file exists
        if !pdb_path.exists() {
            error!("File not found: {}", pdb_path.display());
            all_output.push_str(&format!("❌ ERROR: File not found\n"));
            failed_parses += 1;
            continue;
        }

        // Parse with old pdb crate
        let old_result = match old_pdb::parse_pdb(pdb_path) {
            Ok(result) => {
                info!("✅ Old pdb crate parsed successfully");
                all_output.push_str("✅ Old pdb crate: SUCCESS\n");
                result
            }
            Err(e) => {
                error!("❌ Failed to parse with old pdb crate: {}", e);
                all_output.push_str(&format!("❌ Old pdb crate: FAILED - {}\n", e));
                failed_parses += 1;
                continue;
            }
        };

        // Parse with new ezpdb crate (this will use the cache)
        let new_result = match new_pdb::parse_pdb_new(pdb_path) {
            Ok(result) => {
                info!("✅ New ezpdb crate parsed successfully");
                all_output.push_str("✅ New ezpdb crate: SUCCESS\n");
                result
            }
            Err(e) => {
                error!("❌ Failed to parse with new ezpdb crate: {}", e);
                all_output.push_str(&format!("❌ New ezpdb crate: FAILED - {}\n", e));
                failed_parses += 1;
                continue;
            }
        };

        // Compare results
        let comparison = compare::compare_pdbs(&old_result, &new_result);
        
        // Count offset mismatches specifically
        let offset_mismatches = comparison.differences.iter()
            .filter(|d| d.description.contains("offset mismatch") || d.description.contains("address mismatch"))
            .count();
        
        total_offset_mismatches += offset_mismatches;

        all_output.push_str(&format!("\nComparison Results:\n"));
        all_output.push_str(&format!("  Total differences: {}\n", comparison.summary.total_differences));
        all_output.push_str(&format!("  Offset/Address mismatches: {}\n", offset_mismatches));
        all_output.push_str(&format!("  Public symbols: {} (old) vs {} (new)\n", 
            old_result.symbols.public_symbols.len(), new_result.symbols.public_symbols.len()));
        all_output.push_str(&format!("  Procedures: {} (old) vs {} (new)\n", 
            old_result.symbols.procedures.len(), new_result.symbols.procedures.len()));
        all_output.push_str(&format!("  Data symbols: {} (old) vs {} (new)\n", 
            old_result.symbols.data_symbols.len(), new_result.symbols.data_symbols.len()));

        if offset_mismatches == 0 {
            info!("✅ No offset mismatches - cache working correctly!");
            all_output.push_str("\n✅ CACHE TEST PASSED: No offset mismatches found!\n");
        } else {
            error!("❌ Found {} offset mismatches - cache may have issues!", offset_mismatches);
            all_output.push_str(&format!("\n❌ CACHE TEST FAILED: {} offset mismatches found!\n", offset_mismatches));
        }

        successful_parses += 1;
    }

    // Summary
    all_output.push_str("\n\n");
    all_output.push_str(&"=".repeat(80));
    all_output.push_str("\n");
    all_output.push_str("=== Cache Test Summary ===\n");
    all_output.push_str(&"=".repeat(80));
    all_output.push_str("\n");
    all_output.push_str(&format!("Total PDB files tested: {}\n", test_pdbs.len()));
    all_output.push_str(&format!("Successfully parsed: {}\n", successful_parses));
    all_output.push_str(&format!("Failed to parse: {}\n", failed_parses));
    all_output.push_str(&format!("Total offset mismatches across all files: {}\n", total_offset_mismatches));

    if total_offset_mismatches == 0 && successful_parses == test_pdbs.len() {
        all_output.push_str("\n🎉 CACHE TEST PASSED: All PDBs parsed successfully with no offset mismatches!\n");
        all_output.push_str("The section header cache is working correctly across multiple PDB files.\n");
        info!("🎉 CACHE TEST PASSED!");
    } else {
        all_output.push_str("\n❌ CACHE TEST FAILED: Issues detected\n");
        if failed_parses > 0 {
            all_output.push_str(&format!("  - {} file(s) failed to parse\n", failed_parses));
        }
        if total_offset_mismatches > 0 {
            all_output.push_str(&format!("  - {} offset mismatch(es) found\n", total_offset_mismatches));
        }
        error!("❌ CACHE TEST FAILED");
    }

    // Print to console
    println!("{}", all_output);

    // Save results
    let output_path = PathBuf::from("cache_test_results.txt");
    std::fs::write(&output_path, &all_output)?;
    info!("Results written to: {}", output_path.display());
    println!("\nDetailed results written to: {}", output_path.display());

    // Exit with error if test failed
    if total_offset_mismatches > 0 || failed_parses > 0 {
        std::process::exit(1);
    }

    Ok(())
}
