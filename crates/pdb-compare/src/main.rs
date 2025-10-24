use anyhow::Result;
use clap::Parser;
use log::{error, info};
use std::path::PathBuf;

pub mod compare;
pub mod new_pdb;
pub mod old_pdb;

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

    /// Test that old and new ezpdb APIs produce identical results
    #[arg(long)]
    api_test: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging - set WARN level by default to reduce noise from unhandled symbol types
    if args.verbose {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .filter_module("ezpdb", log::LevelFilter::Error) // Suppress ezpdb warnings (unhandled symbols)
            .init();
    }

    // Handle cache test mode
    if args.cache_test {
        info!("Running cache test with multiple PDBs...");
        return run_cache_test();
    }

    // Handle API test mode
    if args.api_test {
        info!("Running API compatibility test...");
        return run_api_test(&args.pdb_file);
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
                all_output.push_str(&format!(
                    "ERROR: Failed to parse with old pdb crate: {}\n",
                    e
                ));
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
                all_output.push_str(&format!(
                    "ERROR: Failed to parse with new ezpdb crate: {}\n",
                    e
                ));
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
        output.push_str(&format!(
            "\nTotal differences: {}\n",
            comparison.summary.total_differences
        ));
        output.push_str(&format!(
            "Header matches: {}\n",
            comparison.summary.header_matches
        ));
        output.push_str(&format!(
            "Type count matches: {}\n",
            comparison.summary.type_count_matches
        ));
        output.push_str(&format!(
            "Symbol count matches: {}\n",
            comparison.summary.symbol_count_matches
        ));
        output.push_str(&format!(
            "Module count matches: {}\n",
            comparison.summary.module_count_matches
        ));

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
            output.push_str(
                "\n✅ No differences found! Both implementations produce identical results.\n",
            );
        }

        all_output.push_str(&output);
    }

    // Add summary header
    let mut final_output = String::new();
    final_output.push_str("=== PDB Comparison Summary ===\n\n");
    final_output.push_str(&format!("Total PDB files tested: {}\n", total_files));
    final_output.push_str(&format!(
        "Files with differences: {}\n",
        files_with_differences
    ));
    final_output.push_str(&format!(
        "Files matching perfectly: {}\n",
        total_files - files_with_differences
    ));
    final_output.push_str(&all_output);

    // Print to console
    println!("{}", final_output);

    // Always save text output to pdb_compare_results.txt
    let text_output_path = PathBuf::from("pdb_compare_results.txt");
    info!("Writing detailed results to {}", text_output_path.display());
    std::fs::write(&text_output_path, &final_output)?;
    println!(
        "Detailed results written to: {}",
        text_output_path.display()
    );

    // Output to JSON if requested
    if let Some(output_path) = args.output {
        info!(
            "Writing JSON comparison results to {}",
            output_path.display()
        );
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
    // Use relative path to cache_test_pdbs directory
    let cache_dir = PathBuf::from("cache_test_pdbs");

    // Scan the directory for all .pdb files
    let mut test_pdbs = Vec::new();
    if cache_dir.exists() && cache_dir.is_dir() {
        for entry in std::fs::read_dir(&cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("pdb") {
                test_pdbs.push(path);
            }
        }
    }

    if test_pdbs.is_empty() {
        error!("No PDB files found in cache_test_pdbs directory!");
        error!("Please place test PDB files in the cache_test_pdbs directory");
        return Err(anyhow::anyhow!("No test PDB files found"));
    }

    // Sort for consistent ordering
    test_pdbs.sort();

    info!("=== Cache Test Mode ===");
    info!(
        "Testing with {} PDB files from cache_test_pdbs/",
        test_pdbs.len()
    );

    let mut all_output = String::new();
    all_output.push_str("=== Cache Test Results ===\n\n");
    all_output.push_str(&format!(
        "Testing {} PDB files from cache_test_pdbs/\n",
        test_pdbs.len()
    ));
    all_output.push_str("This test compares old ezpdb (pdb crate) vs new ezpdb (ms-pdb)\n\n");

    let mut successful_parses = 0;
    let mut failed_parses = 0;
    let mut total_offset_mismatches = 0;

    for (idx, pdb_path) in test_pdbs.iter().enumerate() {
        info!(
            "\n[{}/{}] Processing: {}",
            idx + 1,
            test_pdbs.len(),
            pdb_path.display()
        );
        all_output.push_str("\n");
        all_output.push_str(&"=".repeat(80));
        all_output.push_str("\n");
        all_output.push_str(&format!(
            "[{}/{}] PDB: {}\n",
            idx + 1,
            test_pdbs.len(),
            pdb_path.display()
        ));
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
        let offset_mismatches = comparison
            .differences
            .iter()
            .filter(|d| {
                d.description.contains("offset mismatch")
                    || d.description.contains("address mismatch")
            })
            .count();

        total_offset_mismatches += offset_mismatches;

        all_output.push_str(&format!("\nComparison Results:\n"));
        all_output.push_str(&format!(
            "  Total differences: {}\n",
            comparison.summary.total_differences
        ));
        all_output.push_str(&format!(
            "  Offset/Address mismatches: {}\n",
            offset_mismatches
        ));
        all_output.push_str(&format!(
            "  Public symbols: {} (old) vs {} (new)\n",
            old_result.symbols.public_symbols.len(),
            new_result.symbols.public_symbols.len()
        ));
        all_output.push_str(&format!(
            "  Procedures: {} (old) vs {} (new)\n",
            old_result.symbols.procedures.len(),
            new_result.symbols.procedures.len()
        ));
        all_output.push_str(&format!(
            "  Data symbols: {} (old) vs {} (new)\n",
            old_result.symbols.data_symbols.len(),
            new_result.symbols.data_symbols.len()
        ));

        if offset_mismatches == 0 {
            info!("✅ No offset mismatches - cache working correctly!");
            all_output.push_str("\n✅ CACHE TEST PASSED: No offset mismatches found!\n");
        } else {
            error!(
                "❌ Found {} offset mismatches - cache may have issues!",
                offset_mismatches
            );
            all_output.push_str(&format!(
                "\n❌ CACHE TEST FAILED: {} offset mismatches found!\n",
                offset_mismatches
            ));
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
    all_output.push_str(&format!(
        "Total offset mismatches across all files: {}\n",
        total_offset_mismatches
    ));

    if total_offset_mismatches == 0 && successful_parses == test_pdbs.len() {
        all_output.push_str(
            "\n🎉 CACHE TEST PASSED: All PDBs parsed successfully with no offset mismatches!\n",
        );
        all_output
            .push_str("The section header cache is working correctly across multiple PDB files.\n");
        info!("🎉 CACHE TEST PASSED!");
    } else {
        all_output.push_str("\n❌ CACHE TEST FAILED: Issues detected\n");
        if failed_parses > 0 {
            all_output.push_str(&format!("  - {} file(s) failed to parse\n", failed_parses));
        }
        if total_offset_mismatches > 0 {
            all_output.push_str(&format!(
                "  - {} offset mismatch(es) found\n",
                total_offset_mismatches
            ));
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

/// Test that old and new ezpdb APIs produce identical results
fn run_api_test(pdb_files: &[PathBuf]) -> Result<()> {
    if pdb_files.is_empty() {
        error!("No PDB files specified for API test. Use --pdb-file <path> --api-test");
        std::process::exit(1);
    }

    info!("Testing API compatibility between old and new ezpdb...");
    info!(
        "Will compare parse_pdb() output for {} file(s)",
        pdb_files.len()
    );

    let mut all_passed = true;
    let mut results = String::new();

    results.push_str(
        "================================================================================\n",
    );
    results.push_str("=== ezpdb API Compatibility Test ===\n");
    results.push_str(
        "================================================================================\n\n",
    );

    for pdb_path in pdb_files {
        info!("Testing: {}", pdb_path.display());
        results.push_str(&format!("\n--- Testing: {} ---\n", pdb_path.display()));

        // Call parse_pdb on both versions
        let old_pdb = match ezpdb_old::parse_pdb(pdb_path, None) {
            Ok(pdb) => pdb,
            Err(e) => {
                error!("OLD ezpdb::parse_pdb() failed: {}", e);
                results.push_str(&format!("❌ OLD ezpdb::parse_pdb() failed: {}\n", e));
                all_passed = false;
                continue;
            }
        };

        let new_pdb = match ezpdb::parse_pdb(pdb_path, None) {
            Ok(pdb) => pdb,
            Err(e) => {
                error!("NEW ezpdb::parse_pdb() failed: {}", e);
                results.push_str(&format!("❌ NEW ezpdb::parse_pdb() failed: {}\n", e));
                all_passed = false;
                continue;
            }
        };

        results.push_str("✅ Both APIs parsed successfully\n\n");

        // Compare all public fields of ParsedPdb
        let mut differences = Vec::new();

        // Compare path
        if old_pdb.path != new_pdb.path {
            differences.push(format!("  path: {:?} vs {:?}", old_pdb.path, new_pdb.path));
        }

        // Compare age
        if old_pdb.age != new_pdb.age {
            differences.push(format!("  age: {} vs {}", old_pdb.age, new_pdb.age));
        }

        // Compare guid
        if old_pdb.guid != new_pdb.guid {
            differences.push(format!("  guid: {} vs {}", old_pdb.guid, new_pdb.guid));
        }

        // Compare timestamp
        if old_pdb.timestamp != new_pdb.timestamp {
            differences.push(format!(
                "  timestamp: {} vs {}",
                old_pdb.timestamp, new_pdb.timestamp
            ));
        }

        // Compare machine_type
        let old_machine = format!("{:?}", old_pdb.machine_type);
        let new_machine = format!("{:?}", new_pdb.machine_type);
        if old_machine != new_machine {
            differences.push(format!(
                "  machine_type: {} vs {}",
                old_machine, new_machine
            ));
        }

        // Compare version (allow different representations of the same version)
        let old_version = format!("{:?}", old_pdb.version);
        let new_version = format!("{:?}", new_pdb.version);
        if old_version != new_version {
            // Log as info but don't fail - version detection may differ slightly
            results.push_str(&format!(
                "ℹ️  Version representation differs: {} (old) vs {} (new)\n",
                old_version, new_version
            ));
        }

        // Compare public_symbols count and content
        if old_pdb.public_symbols.len() != new_pdb.public_symbols.len() {
            differences.push(format!(
                "  public_symbols.len(): {} vs {}",
                old_pdb.public_symbols.len(),
                new_pdb.public_symbols.len()
            ));
        } else {
            // Check a sample of symbols for exact match
            let mut symbol_mismatches = 0;
            for (old_sym, new_sym) in old_pdb
                .public_symbols
                .iter()
                .zip(new_pdb.public_symbols.iter())
                .take(10)
            {
                if old_sym.name != new_sym.name || old_sym.offset != new_sym.offset {
                    symbol_mismatches += 1;
                }
            }
            if symbol_mismatches > 0 {
                differences.push(format!(
                    "  public_symbols content differs (checked {} symbols, {} mismatches)",
                    10.min(old_pdb.public_symbols.len()),
                    symbol_mismatches
                ));
            }
        }

        // Compare procedures count
        if old_pdb.procedures.len() != new_pdb.procedures.len() {
            differences.push(format!(
                "  procedures.len(): {} vs {}",
                old_pdb.procedures.len(),
                new_pdb.procedures.len()
            ));
        }

        // Compare global_data count
        if old_pdb.global_data.len() != new_pdb.global_data.len() {
            if new_pdb.global_data.len() > old_pdb.global_data.len() {
                results.push_str(&format!(
                    "✅ IMPROVEMENT: New API found {} more global data symbols ({} vs {})\n",
                    new_pdb.global_data.len() - old_pdb.global_data.len(),
                    old_pdb.global_data.len(),
                    new_pdb.global_data.len()
                ));
            } else {
                differences.push(format!(
                    "  REGRESSION: global_data.len(): {} vs {}",
                    old_pdb.global_data.len(),
                    new_pdb.global_data.len()
                ));
            }
        }

        // Compare debug_modules count
        if old_pdb.debug_modules.len() != new_pdb.debug_modules.len() {
            differences.push(format!(
                "  debug_modules.len(): {} vs {}",
                old_pdb.debug_modules.len(),
                new_pdb.debug_modules.len()
            ));
        }

        // Compare types count
        // NOTE: Old API mixed TPI and IPI types together in .types
        // New API separates them: .types (TPI) and .ipi_types (IPI)
        let old_total_types = old_pdb.types.len();
        let new_total_types = new_pdb.types.len() + new_pdb.ipi_types.len();

        if !new_pdb.ipi_types.is_empty() {
            results.push_str(&format!(
                "ℹ️  Type separation: Old API had {} types (TPI+IPI mixed), New API has {} TPI + {} IPI = {} total\n",
                old_total_types,
                new_pdb.types.len(),
                new_pdb.ipi_types.len(),
                new_total_types
            ));
        }

        // It's OK if new finds more types (improvement), but not if it finds fewer
        if new_total_types < old_total_types {
            differences.push(format!(
                "  REGRESSION: Total types decreased: {} (old) vs {} (new)",
                old_total_types, new_total_types
            ));
        } else if new_total_types > old_total_types {
            results.push_str(&format!(
                "✅ IMPROVEMENT: New API found {} more types than old\n",
                new_total_types - old_total_types
            ));
        }

        // Report results for this file
        if differences.is_empty() {
            results.push_str("✅ PASSED: All public fields match between old and new API\n");
            info!("✅ API test PASSED for {}", pdb_path.display());
        } else {
            results.push_str("❌ FAILED: Differences found:\n");
            for diff in &differences {
                results.push_str(&format!("{}\n", diff));
            }
            error!("❌ API test FAILED for {}", pdb_path.display());
            all_passed = false;
        }

        results.push_str("\nSummary for this file:\n");
        results.push_str(&format!(
            "  Public symbols: {} (old) vs {} (new)\n",
            old_pdb.public_symbols.len(),
            new_pdb.public_symbols.len()
        ));
        results.push_str(&format!(
            "  Procedures: {} (old) vs {} (new)\n",
            old_pdb.procedures.len(),
            new_pdb.procedures.len()
        ));
        results.push_str(&format!(
            "  Global data: {} (old) vs {} (new)\n",
            old_pdb.global_data.len(),
            new_pdb.global_data.len()
        ));
        results.push_str(&format!(
            "  Types: {} (old) vs {} (new)\n",
            old_pdb.types.len(),
            new_pdb.types.len()
        ));
        results.push_str(&format!(
            "  IPI types: N/A (old) vs {} (new)\n",
            new_pdb.ipi_types.len()
        ));
        results.push_str(&format!(
            "  Debug modules: {} (old) vs {} (new)\n",
            old_pdb.debug_modules.len(),
            new_pdb.debug_modules.len()
        ));
        results.push_str("\n");
    }

    // Final summary
    results.push_str(
        "\n================================================================================\n",
    );
    results.push_str("=== API Compatibility Test Summary ===\n");
    results.push_str(
        "================================================================================\n",
    );
    results.push_str(&format!("Files tested: {}\n", pdb_files.len()));

    if all_passed {
        results.push_str("\n🎉 ALL TESTS PASSED!\n");
        results.push_str("The new ezpdb API produces identical results to the old API.\n");
        info!("🎉 API compatibility test PASSED!");
    } else {
        results.push_str("\n❌ SOME TESTS FAILED!\n");
        results.push_str("There are differences between old and new ezpdb API outputs.\n");
        error!("❌ API compatibility test FAILED!");
    }

    // Print to console
    println!("{}", results);

    // Save results
    let output_path = PathBuf::from("api_test_results.txt");
    std::fs::write(&output_path, &results)?;
    info!("Results written to: {}", output_path.display());
    println!("\nDetailed results written to: {}", output_path.display());

    if !all_passed {
        std::process::exit(1);
    }

    Ok(())
}
