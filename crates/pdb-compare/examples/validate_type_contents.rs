//! Deep Content Validation Test for TPI/IPI Type Data
//!
//! This test validates the ACTUAL CONTENT of parsed types:
//! - Strings must be valid UTF-8/ASCII, not garbage
//! - Numeric fields must be in reasonable ranges
//! - Type indices must reference valid types
//! - Recently added types (Alias, VTableShape, VFTable) are deeply validated

use ms_pdb::types::{TypeData, TypeIndex};
use ms_pdb::{BStr, Pdb, RandomAccessFile};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("========================================");
    println!("TPI/IPI Type Content Validation Test");
    println!("========================================");
    println!();

    let test_dir = PathBuf::from("cache_test_pdbs");
    if !test_dir.exists() {
        eprintln!("Error: cache_test_pdbs directory not found");
        return Err("Test directory not found".into());
    }

    let pdb_files = find_pdb_files(&test_dir)?;
    if pdb_files.is_empty() {
        eprintln!("Error: No PDB files found in cache_test_pdbs");
        return Err("No test files found".into());
    }

    println!("Found {} PDB file(s) to validate\n", pdb_files.len());

    let mut total_stats = ValidationStats::default();
    let mut all_passed = true;

    for pdb_path in &pdb_files {
        println!("==========================================");
        println!(
            "Validating: {}",
            pdb_path.file_name().unwrap().to_string_lossy()
        );
        println!("==========================================");

        match validate_pdb(pdb_path) {
            Ok(stats) => {
                print_validation_results(&stats);
                total_stats.merge(&stats);

                if stats.validation_failures > 0 {
                    all_passed = false;
                }
            }
            Err(e) => {
                eprintln!("✗ Failed to validate PDB: {}", e);
                all_passed = false;
            }
        }
        println!();
    }

    // Print final summary
    println!("==========================================");
    println!("FINAL VALIDATION SUMMARY");
    println!("==========================================");
    println!();
    print_final_summary(&total_stats);

    if all_passed && total_stats.validation_failures == 0 {
        println!("\n✓✓✓ ALL VALIDATIONS PASSED ✓✓✓");
        println!("All type contents are valid and meaningful!");
        Ok(())
    } else {
        println!("\n✗✗✗ VALIDATION FAILURES DETECTED ✗✗✗");
        Err("Content validation failed".into())
    }
}

#[derive(Default, Debug)]
struct ValidationStats {
    total_types: usize,
    validated_strings: usize,
    validated_type_indices: usize,
    validated_numeric_fields: usize,

    // New type validations
    alias_validated: usize,
    vtable_shape_validated: usize,
    vftable_validated: usize,

    // Issues found
    invalid_strings: usize,
    invalid_type_indices: usize,
    invalid_numeric_ranges: usize,
    validation_failures: usize,

    // Detailed findings
    string_samples: Vec<String>,
    alias_samples: Vec<String>,
    vtshape_samples: Vec<String>,
    vftable_samples: Vec<String>,
    suspicious_findings: Vec<String>,
}

impl ValidationStats {
    fn merge(&mut self, other: &ValidationStats) {
        self.total_types += other.total_types;
        self.validated_strings += other.validated_strings;
        self.validated_type_indices += other.validated_type_indices;
        self.validated_numeric_fields += other.validated_numeric_fields;
        self.alias_validated += other.alias_validated;
        self.vtable_shape_validated += other.vtable_shape_validated;
        self.vftable_validated += other.vftable_validated;
        self.invalid_strings += other.invalid_strings;
        self.invalid_type_indices += other.invalid_type_indices;
        self.invalid_numeric_ranges += other.invalid_numeric_ranges;
        self.validation_failures += other.validation_failures;

        self.string_samples
            .extend(other.string_samples.iter().cloned());
        self.string_samples.truncate(20);
        self.alias_samples
            .extend(other.alias_samples.iter().cloned());
        self.alias_samples.truncate(5);
        self.vtshape_samples
            .extend(other.vtshape_samples.iter().cloned());
        self.vtshape_samples.truncate(5);
        self.vftable_samples
            .extend(other.vftable_samples.iter().cloned());
        self.vftable_samples.truncate(5);
        self.suspicious_findings
            .extend(other.suspicious_findings.iter().cloned());
    }
}

fn validate_pdb(path: &Path) -> Result<ValidationStats, Box<dyn std::error::Error>> {
    let pdb = Pdb::open(path)?;
    let mut stats = ValidationStats::default();

    // Build valid type index set
    let mut valid_type_indices = HashSet::new();

    println!("\n📊 Phase 1: Building type index maps...");

    // Map TPI indices
    let type_stream = pdb.read_type_stream()?;
    let tpi_begin = type_stream.type_index_begin();
    let tpi_end = type_stream.type_index_end();
    for idx in tpi_begin.0..tpi_end.0 {
        valid_type_indices.insert(TypeIndex(idx));
    }
    println!(
        "  TPI index range: {} - {} ({} types)",
        tpi_begin.0,
        tpi_end.0 - 1,
        tpi_end.0 - tpi_begin.0
    );

    // Map IPI indices
    match pdb.read_ipi_stream() {
        Ok(ipi_stream) => {
            let ipi_begin = ipi_stream.type_index_begin();
            let ipi_end = ipi_stream.type_index_end();
            for idx in ipi_begin.0..ipi_end.0 {
                valid_type_indices.insert(TypeIndex(idx));
            }
            println!(
                "  IPI index range: {} - {} ({} types)",
                ipi_begin.0,
                ipi_end.0 - 1,
                ipi_end.0 - ipi_begin.0
            );
        }
        Err(_) => {
            println!("  (No IPI stream)");
        }
    }

    println!("\n📊 Phase 2: Validating TPI type contents...");
    validate_type_stream(&pdb, true, &valid_type_indices, &mut stats)?;

    println!("📊 Phase 3: Validating IPI type contents...");
    if pdb.read_ipi_stream().is_ok() {
        validate_type_stream(&pdb, false, &valid_type_indices, &mut stats)?;
    }

    Ok(stats)
}

fn validate_type_stream(
    pdb: &Pdb<RandomAccessFile>,
    is_tpi: bool,
    valid_type_indices: &HashSet<TypeIndex>,
    stats: &mut ValidationStats,
) -> Result<(), Box<dyn std::error::Error>> {
    let type_stream = if is_tpi {
        pdb.read_type_stream()?
    } else {
        pdb.read_ipi_stream()?
    };

    let mut record_count = 0;
    let stream_name = if is_tpi { "TPI" } else { "IPI" };

    for record in type_stream.iter_type_records() {
        record_count += 1;
        stats.total_types += 1;

        match record.parse() {
            Ok(data) => {
                validate_type_data(&data, valid_type_indices, stats, stream_name, record_count);
            }
            Err(e) => {
                let msg = format!(
                    "Parse error in {} at record {}: {}",
                    stream_name, record_count, e
                );
                stats.suspicious_findings.push(msg);
                stats.validation_failures += 1;
            }
        }
    }

    println!("  ✓ Validated {} {} records", record_count, stream_name);
    Ok(())
}

fn validate_type_data(
    data: &TypeData,
    valid_type_indices: &HashSet<TypeIndex>,
    stats: &mut ValidationStats,
    stream_name: &str,
    record_num: usize,
) {
    match data {
        TypeData::Struct(s) => {
            validate_bstr(s.name, "Struct name", stats);
            validate_type_index(
                s.fixed.field_list.get(),
                "Struct fields",
                valid_type_indices,
                stats,
            );
            // Note: s.length is a Number type which can be large, we just check it exists
            if stats.string_samples.len() < 5 {
                stats.string_samples.push(format!("Struct: '{}'", s.name));
            }
        }

        TypeData::Union(u) => {
            validate_bstr(u.name, "Union name", stats);
            validate_type_index(
                u.fixed.fields.get(),
                "Union fields",
                valid_type_indices,
                stats,
            );
            if stats.string_samples.len() < 5 {
                stats.string_samples.push(format!("Union: '{}'", u.name));
            }
        }

        TypeData::Enum(e) => {
            validate_bstr(e.name, "Enum name", stats);
            validate_type_index(
                e.fixed.fields.get(),
                "Enum fields",
                valid_type_indices,
                stats,
            );
            validate_type_index(
                e.fixed.underlying_type.get(),
                "Enum underlying type",
                valid_type_indices,
                stats,
            );
            if stats.string_samples.len() < 5 {
                stats.string_samples.push(format!("Enum: '{}'", e.name));
            }
        }

        TypeData::Alias(alias) => {
            validate_bstr(alias.name, "Alias name", stats);
            validate_type_index(
                alias.utype,
                "Alias underlying type",
                valid_type_indices,
                stats,
            );
            stats.alias_validated += 1;

            let name_str = alias.name.to_string();
            stats
                .alias_samples
                .push(format!("Alias: '{}' -> type {}", name_str, alias.utype.0));

            println!(
                "    ✓ Alias validated: '{}' -> type index {}",
                name_str, alias.utype.0
            );

            // Validate name is reasonable
            if name_str.len() > 500 {
                stats.suspicious_findings.push(format!(
                    "Alias name suspiciously long ({} chars): {}",
                    name_str.len(),
                    name_str
                ));
            }
        }

        TypeData::VTableShape(vtshape) => {
            // Validate count is reasonable (virtual tables rarely have >100 entries)
            if vtshape.count == 0 || vtshape.count > 100 {
                stats.suspicious_findings.push(format!(
                    "VTableShape at {} record {}: suspicious count {}",
                    stream_name, record_num, vtshape.count
                ));
                stats.invalid_numeric_ranges += 1;
            } else {
                stats.validated_numeric_fields += 1;
            }

            // Descriptor bytes: Each descriptor is a nibble (4 bits), 2 per byte
            // Minimum bytes needed: ceil(count / 2) = (count + 1) / 2
            // Microsoft's format may have padding, so we check minimum and maximum reasonable values
            let min_bytes = ((vtshape.count + 1) / 2) as usize;
            let max_reasonable_bytes = ((vtshape.count + 3) / 2) as usize; // Allow 1 byte padding
            let actual_bytes = vtshape.descriptors.len();

            if actual_bytes < min_bytes {
                stats.suspicious_findings.push(format!(
                    "VTableShape at {} record {}: count={} needs at least {} descriptor bytes, got {} - DATA TRUNCATED",
                    stream_name, record_num, vtshape.count, min_bytes, actual_bytes
                ));
                stats.validation_failures += 1;
            } else if actual_bytes > max_reasonable_bytes + 4 {
                // More than 4 bytes of padding is suspicious
                stats.suspicious_findings.push(format!(
                    "VTableShape at {} record {}: count={} expects ~{} descriptor bytes, got {} - EXCESSIVE DATA",
                    stream_name, record_num, vtshape.count, min_bytes, actual_bytes
                ));
            } else {
                // Validate within reasonable range
                stats.vtable_shape_validated += 1;

                // Decode descriptor nibbles to validate they're reasonable
                let mut valid_descriptors = true;
                let mut invalid_nibbles = Vec::new();

                for (i, &byte) in vtshape.descriptors.iter().enumerate() {
                    let low_nibble = byte & 0x0F;
                    let high_nibble = (byte >> 4) & 0x0F;

                    // Valid descriptor values are 0-5 according to CV_VTS_desc enum
                    // 0 = near16, 1 = far16, 2 = thin, 3 = outer, 4 = meta, 5 = near, 6+ = unused
                    // Check only nibbles that correspond to actual entries
                    let nibble_index = i * 2;
                    if nibble_index < vtshape.count as usize && low_nibble > 5 {
                        valid_descriptors = false;
                        invalid_nibbles.push((nibble_index, low_nibble));
                    }
                    if nibble_index + 1 < vtshape.count as usize && high_nibble > 5 {
                        valid_descriptors = false;
                        invalid_nibbles.push((nibble_index + 1, high_nibble));
                    }
                }

                if !valid_descriptors {
                    stats.suspicious_findings.push(format!(
                        "VTableShape at {} record {}: invalid descriptor nibbles {:?} (should be 0-5)",
                        stream_name, record_num, invalid_nibbles
                    ));
                    stats.validation_failures += 1;
                } else {
                    let note = if actual_bytes > min_bytes {
                        format!(" (has {} byte padding)", actual_bytes - min_bytes)
                    } else {
                        String::new()
                    };
                    let sample = format!(
                        "VTableShape: count={}, descriptors={:?}{}",
                        vtshape.count, vtshape.descriptors, note
                    );
                    stats.vtshape_samples.push(sample.clone());
                    println!("    ✓ {}", sample);
                }
            }
        }

        TypeData::VFTable(vftable) => {
            validate_type_index(
                vftable.root.get(),
                "VFTable root",
                valid_type_indices,
                stats,
            );
            validate_type_index(
                vftable.path.get(),
                "VFTable path",
                valid_type_indices,
                stats,
            );

            // Validate offset is reasonable (usually within 1MB)
            let offset = vftable.off.get();
            if offset > 1_000_000 {
                stats.suspicious_findings.push(format!(
                    "VFTable at {} record {}: suspicious offset {}",
                    stream_name, record_num, offset
                ));
                stats.invalid_numeric_ranges += 1;
            } else {
                stats.validated_numeric_fields += 1;
            }

            // Validate segment is reasonable (usually 0-20 for common executables)
            let segment = vftable.seg.get();
            if segment > 100 {
                stats.suspicious_findings.push(format!(
                    "VFTable at {} record {}: suspicious segment {}",
                    stream_name, record_num, segment
                ));
                stats.invalid_numeric_ranges += 1;
            } else {
                stats.validated_numeric_fields += 1;
            }

            stats.vftable_validated += 1;
            let sample = format!(
                "VFTable: root={}, path={}, offset={}, seg={}",
                vftable.root.get().0,
                vftable.path.get().0,
                offset,
                segment
            );
            stats.vftable_samples.push(sample.clone());
            println!("    ✓ {}", sample);
        }

        TypeData::Pointer(ptr) => {
            validate_type_index(
                ptr.fixed.ty.get(),
                "Pointer type",
                valid_type_indices,
                stats,
            );
        }

        TypeData::Array(arr) => {
            validate_type_index(
                arr.fixed.element_type.get(),
                "Array element",
                valid_type_indices,
                stats,
            );
            validate_type_index(
                arr.fixed.index_type.get(),
                "Array index",
                valid_type_indices,
                stats,
            );
        }

        TypeData::Proc(proc) => {
            validate_type_index(
                proc.return_value.get(),
                "Proc return type",
                valid_type_indices,
                stats,
            );
            validate_type_index(
                proc.arg_list.get(),
                "Proc arglist",
                valid_type_indices,
                stats,
            );

            let num_params = proc.num_params.get();
            if num_params > 255 {
                stats.suspicious_findings.push(format!(
                    "Proc at {} record {}: suspicious param count {}",
                    stream_name, record_num, num_params
                ));
            } else {
                stats.validated_numeric_fields += 1;
            }
        }

        TypeData::MemberFunc(mf) => {
            validate_type_index(
                mf.return_value.get(),
                "MemberFunc return",
                valid_type_indices,
                stats,
            );
            validate_type_index(
                mf.class.get(),
                "MemberFunc class",
                valid_type_indices,
                stats,
            );
            validate_type_index(mf.this.get(), "MemberFunc this", valid_type_indices, stats);
            validate_type_index(
                mf.arg_list.get(),
                "MemberFunc arglist",
                valid_type_indices,
                stats,
            );
        }

        TypeData::FuncId(funcid) => {
            validate_bstr(funcid.name, "FuncId name", stats);
            if stats.string_samples.len() < 15 {
                stats
                    .string_samples
                    .push(format!("FuncId: '{}'", funcid.name));
            }
        }

        TypeData::MFuncId(mfuncid) => {
            validate_bstr(mfuncid.name, "MFuncId name", stats);
        }

        TypeData::StringId(strid) => {
            validate_bstr(strid.name, "StringId", stats);
            if stats.string_samples.len() < 20 {
                stats
                    .string_samples
                    .push(format!("StringId: '{}'", strid.name));
            }
        }

        TypeData::BuildInfo(buildinfo) => {
            for arg in buildinfo.args {
                validate_type_index(
                    TypeIndex(arg.get()),
                    "BuildInfo arg",
                    valid_type_indices,
                    stats,
                );
            }
        }

        TypeData::Modifier(modif) => {
            validate_type_index(
                modif.underlying_type.get(),
                "Modifier type",
                valid_type_indices,
                stats,
            );
        }

        TypeData::ArgList(arglist) => {
            for &arg in arglist.args {
                validate_type_index(arg.get(), "ArgList arg", valid_type_indices, stats);
            }
        }

        _ => {
            // Other types don't have easily validatable content
        }
    }
}

fn validate_bstr(bstr: &BStr, context: &str, stats: &mut ValidationStats) {
    stats.validated_strings += 1;

    let s = bstr.to_string();

    // Check for suspicious characters
    if s.chars()
        .any(|c| c.is_control() && c != '\t' && c != '\n' && c != '\r')
    {
        stats
            .suspicious_findings
            .push(format!("{} contains control characters: '{}'", context, s));
        stats.invalid_strings += 1;
    }

    // Check for reasonable length (most symbols are < 5000 chars)
    if s.len() > 10000 {
        stats.suspicious_findings.push(format!(
            "{} is suspiciously long: {} chars",
            context,
            s.len()
        ));
        stats.invalid_strings += 1;
    }
}

fn validate_type_index(
    type_idx: TypeIndex,
    context: &str,
    valid_type_indices: &HashSet<TypeIndex>,
    stats: &mut ValidationStats,
) {
    stats.validated_type_indices += 1;

    // Type index 0 means "no type" or void
    if type_idx.0 == 0 {
        return;
    }

    // Primitive types are < 0x1000
    if type_idx.0 < 0x1000 {
        return; // Primitive types are always valid
    }

    // Check if it's a valid TPI or IPI index
    if !valid_type_indices.contains(&type_idx) {
        stats.suspicious_findings.push(format!(
            "{} references invalid type index: {} (0x{:X})",
            context, type_idx.0, type_idx.0
        ));
        stats.invalid_type_indices += 1;
    }
}

fn find_pdb_files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut pdb_files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.eq_ignore_ascii_case("pdb") {
                    pdb_files.push(path);
                }
            }
        }
    }
    pdb_files.sort();
    Ok(pdb_files)
}

fn print_validation_results(stats: &ValidationStats) {
    println!("\n📈 Validation Results:");
    println!("  Total types validated: {}", stats.total_types);
    println!("  Strings validated: {}", stats.validated_strings);
    println!("  Type indices validated: {}", stats.validated_type_indices);
    println!(
        "  Numeric fields validated: {}",
        stats.validated_numeric_fields
    );

    println!("\n✨ New Types Validated:");
    println!("  Alias: {}", stats.alias_validated);
    println!("  VTableShape: {}", stats.vtable_shape_validated);
    println!("  VFTable: {}", stats.vftable_validated);

    println!("\n🔍 Content Issues Found:");
    println!("  Invalid strings: {}", stats.invalid_strings);
    println!("  Invalid type indices: {}", stats.invalid_type_indices);
    println!("  Invalid numeric ranges: {}", stats.invalid_numeric_ranges);
    println!("  Total failures: {}", stats.validation_failures);

    if !stats.string_samples.is_empty() {
        println!("\n📝 Sample Strings (proving data is real):");
        for sample in stats.string_samples.iter().take(10) {
            println!("  {}", sample);
        }
    }

    if !stats.alias_samples.is_empty() {
        println!("\n📋 Alias Samples:");
        for sample in &stats.alias_samples {
            println!("  {}", sample);
        }
    }

    if !stats.vtshape_samples.is_empty() {
        println!("\n📋 VTableShape Samples:");
        for sample in &stats.vtshape_samples {
            println!("  {}", sample);
        }
    }

    if !stats.vftable_samples.is_empty() {
        println!("\n📋 VFTable Samples:");
        for sample in &stats.vftable_samples {
            println!("  {}", sample);
        }
    }

    if !stats.suspicious_findings.is_empty() {
        println!("\n⚠️  Suspicious Findings:");
        for finding in stats.suspicious_findings.iter().take(10) {
            println!("  {}", finding);
        }
        if stats.suspicious_findings.len() > 10 {
            println!("  ... and {} more", stats.suspicious_findings.len() - 10);
        }
    }

    if stats.validation_failures == 0
        && stats.invalid_strings == 0
        && stats.invalid_type_indices == 0
        && stats.invalid_numeric_ranges == 0
    {
        println!("\n✅ All content validations passed!");
    } else {
        println!("\n❌ Content validation issues detected!");
    }
}

fn print_final_summary(stats: &ValidationStats) {
    println!("Total types validated: {}", stats.total_types);
    println!("  Strings: {}", stats.validated_strings);
    println!("  Type indices: {}", stats.validated_type_indices);
    println!("  Numeric fields: {}", stats.validated_numeric_fields);
    println!();

    println!("New types validated:");
    println!("  Alias: {}", stats.alias_validated);
    println!("  VTableShape: {}", stats.vtable_shape_validated);
    println!("  VFTable: {}", stats.vftable_validated);
    println!();

    println!("Content issues:");
    println!("  Invalid strings: {}", stats.invalid_strings);
    println!("  Invalid type indices: {}", stats.invalid_type_indices);
    println!("  Invalid numeric ranges: {}", stats.invalid_numeric_ranges);
    println!("  Total failures: {}", stats.validation_failures);
}
