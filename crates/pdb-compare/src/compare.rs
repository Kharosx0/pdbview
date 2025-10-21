use crate::old_pdb::OldPdbData;
use crate::new_pdb::NewPdbData;
use log::debug;

#[derive(Debug, serde::Serialize)]
pub struct ComparisonResult {
    pub summary: ComparisonSummary,
    pub differences: Vec<Difference>,
}

#[derive(Debug, serde::Serialize)]
pub struct ComparisonSummary {
    pub total_differences: usize,
    pub header_matches: bool,
    pub type_count_matches: bool,
    pub symbol_count_matches: bool,
    pub module_count_matches: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Difference {
    pub category: String,
    pub description: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

impl Difference {
    pub fn new(category: &str, description: &str, old_value: Option<String>, new_value: Option<String>) -> Self {
        Difference {
            category: category.to_string(),
            description: description.to_string(),
            old_value,
            new_value,
        }
    }
}

pub fn compare_pdbs(old: &OldPdbData, new: &NewPdbData) -> ComparisonResult {
    debug!("Comparing PDB data from old and new parsers...");
    let mut differences = Vec::new();

    // Compare headers
    let header_matches = compare_headers(&old.header, &new.header, &mut differences);

    // Compare type counts
    let type_count_matches = compare_type_counts(old, new, &mut differences);

    // Compare symbol counts
    let symbol_count_matches = compare_symbol_counts(old, new, &mut differences);

    // Compare module counts
    let module_count_matches = compare_module_counts(old, new, &mut differences);

    // Compare types in detail
    compare_types(old, new, &mut differences);

    // Compare symbols in detail
    compare_symbols(old, new, &mut differences);

    // Compare modules in detail
    compare_modules(old, new, &mut differences);

    let summary = ComparisonSummary {
        total_differences: differences.len(),
        header_matches,
        type_count_matches,
        symbol_count_matches,
        module_count_matches,
    };

    debug!("Comparison complete: {} differences found", differences.len());

    ComparisonResult {
        summary,
        differences,
    }
}

fn compare_headers(
    old: &crate::old_pdb::HeaderInfo,
    new: &crate::new_pdb::HeaderInfo,
    differences: &mut Vec<Difference>,
) -> bool {
    let mut matches = true;

    if old.age != new.age {
        differences.push(Difference::new(
            "header",
            "Age mismatch",
            Some(old.age.to_string()),
            Some(new.age.to_string()),
        ));
        matches = false;
    }

    if old.guid != new.guid {
        differences.push(Difference::new(
            "header",
            "GUID mismatch",
            Some(old.guid.clone()),
            Some(new.guid.clone()),
        ));
        matches = false;
    }

    matches
}

fn compare_type_counts(
    old: &OldPdbData,
    new: &NewPdbData,
    differences: &mut Vec<Difference>,
) -> bool {
    let old_count = old.types.len();
    let new_count = new.types.len();

    if old_count != new_count {
        differences.push(Difference::new(
            "types",
            "Type count mismatch",
            Some(old_count.to_string()),
            Some(new_count.to_string()),
        ));
        false
    } else {
        true
    }
}

fn compare_symbol_counts(
    old: &OldPdbData,
    new: &NewPdbData,
    differences: &mut Vec<Difference>,
) -> bool {
    let mut matches = true;

    let old_public = old.symbols.public_symbols.len();
    let new_public = new.symbols.public_symbols.len();
    if old_public != new_public {
        differences.push(Difference::new(
            "symbols",
            "Public symbol count mismatch",
            Some(old_public.to_string()),
            Some(new_public.to_string()),
        ));
        matches = false;
    }

    let old_procs = old.symbols.procedures.len();
    let new_procs = new.symbols.procedures.len();
    if old_procs != new_procs {
        differences.push(Difference::new(
            "symbols",
            "Procedure count mismatch",
            Some(old_procs.to_string()),
            Some(new_procs.to_string()),
        ));
        matches = false;
    }

    let old_data = old.symbols.data_symbols.len();
    let new_data = new.symbols.data_symbols.len();
    if old_data != new_data {
        differences.push(Difference::new(
            "symbols",
            "Data symbol count mismatch",
            Some(old_data.to_string()),
            Some(new_data.to_string()),
        ));
        matches = false;
    }

    matches
}

fn compare_module_counts(
    old: &OldPdbData,
    new: &NewPdbData,
    differences: &mut Vec<Difference>,
) -> bool {
    let old_count = old.modules.len();
    let new_count = new.modules.len();

    if old_count != new_count {
        differences.push(Difference::new(
            "modules",
            "Module count mismatch",
            Some(old_count.to_string()),
            Some(new_count.to_string()),
        ));
        false
    } else {
        true
    }
}

fn compare_types(
    old: &OldPdbData,
    new: &NewPdbData,
    differences: &mut Vec<Difference>,
) {
    debug!("Comparing types in detail...");

    // Create indices for quick lookup
    use std::collections::HashMap;
    let old_by_index: HashMap<u32, &crate::old_pdb::TypeInfo> =
        old.types.iter().map(|t| (t.index, t)).collect();
    let new_by_index: HashMap<u32, &crate::new_pdb::TypeInfo> =
        new.types.iter().map(|t| (t.index, t)).collect();

    // Check for types in old but not in new
    for old_type in &old.types {
        if let Some(new_type) = new_by_index.get(&old_type.index) {
            // Compare kind
            if old_type.kind != new_type.kind {
                differences.push(Difference::new(
                    "types",
                    &format!("Type {} kind mismatch", old_type.index),
                    Some(old_type.kind.clone()),
                    Some(new_type.kind.clone()),
                ));
            }

            // Compare name
            if old_type.name != new_type.name {
                differences.push(Difference::new(
                    "types",
                    &format!("Type {} name mismatch", old_type.index),
                    Some(format!("{:?}", old_type.name)),
                    Some(format!("{:?}", new_type.name)),
                ));
            }

            // Compare size
            if old_type.size != new_type.size {
                differences.push(Difference::new(
                    "types",
                    &format!("Type {} size mismatch", old_type.index),
                    Some(format!("{:?}", old_type.size)),
                    Some(format!("{:?}", new_type.size)),
                ));
            }
        } else {
            differences.push(Difference::new(
                "types",
                &format!("Type {} missing in new parser", old_type.index),
                Some(format!("{:?}", old_type)),
                None,
            ));
        }
    }

    // Check for types in new but not in old
    for new_type in &new.types {
        if !old_by_index.contains_key(&new_type.index) {
            differences.push(Difference::new(
                "types",
                &format!("Type {} only in new parser", new_type.index),
                None,
                Some(format!("{:?}", new_type)),
            ));
        }
    }
}

fn compare_symbols(
    old: &OldPdbData,
    new: &NewPdbData,
    differences: &mut Vec<Difference>,
) {
    debug!("Comparing symbols in detail...");

    // Compare public symbols
    use std::collections::HashMap;
    let old_public: HashMap<&str, &crate::old_pdb::PublicSymbol> =
        old.symbols.public_symbols.iter().map(|s| (s.name.as_str(), s)).collect();
    let new_public: HashMap<&str, &crate::new_pdb::PublicSymbol> =
        new.symbols.public_symbols.iter().map(|s| (s.name.as_str(), s)).collect();

    for (name, old_sym) in &old_public {
        if let Some(new_sym) = new_public.get(name) {
            if old_sym.offset != new_sym.offset {
                differences.push(Difference::new(
                    "symbols",
                    &format!("Public symbol '{}' offset mismatch", name),
                    Some(format!("{:?}", old_sym.offset)),
                    Some(format!("{:?}", new_sym.offset)),
                ));
            }

            if old_sym.is_function != new_sym.is_function {
                differences.push(Difference::new(
                    "symbols",
                    &format!("Public symbol '{}' is_function mismatch", name),
                    Some(old_sym.is_function.to_string()),
                    Some(new_sym.is_function.to_string()),
                ));
            }
        } else {
            differences.push(Difference::new(
                "symbols",
                &format!("Public symbol '{}' missing in new parser", name),
                Some(format!("{:?}", old_sym)),
                None,
            ));
        }
    }

    for (name, new_sym) in &new_public {
        if !old_public.contains_key(name) {
            differences.push(Difference::new(
                "symbols",
                &format!("Public symbol '{}' only in new parser", name),
                None,
                Some(format!("{:?}", new_sym)),
            ));
        }
    }

    // Compare procedures
    let old_procs: HashMap<&str, &crate::old_pdb::ProcedureSymbol> =
        old.symbols.procedures.iter().map(|p| (p.name.as_str(), p)).collect();
    let new_procs: HashMap<&str, &crate::new_pdb::ProcedureSymbol> =
        new.symbols.procedures.iter().map(|p| (p.name.as_str(), p)).collect();

    for (name, old_proc) in &old_procs {
        if let Some(new_proc) = new_procs.get(name) {
            if old_proc.offset != new_proc.offset {
                differences.push(Difference::new(
                    "symbols",
                    &format!("Procedure '{}' offset mismatch", name),
                    Some(format!("{:?}", old_proc.offset)),
                    Some(format!("{:?}", new_proc.offset)),
                ));
            }

            if old_proc.len != new_proc.len {
                differences.push(Difference::new(
                    "symbols",
                    &format!("Procedure '{}' length mismatch", name),
                    Some(old_proc.len.to_string()),
                    Some(new_proc.len.to_string()),
                ));
            }
        } else {
            differences.push(Difference::new(
                "symbols",
                &format!("Procedure '{}' missing in new parser", name),
                Some(format!("{:?}", old_proc)),
                None,
            ));
        }
    }

    for (name, new_proc) in &new_procs {
        if !old_procs.contains_key(name) {
            differences.push(Difference::new(
                "symbols",
                &format!("Procedure '{}' only in new parser", name),
                None,
                Some(format!("{:?}", new_proc)),
            ));
        }
    }

    // Compare data symbols
    let old_data: HashMap<&str, &crate::old_pdb::DataSymbol> =
        old.symbols.data_symbols.iter().map(|d| (d.name.as_str(), d)).collect();
    let new_data: HashMap<&str, &crate::new_pdb::DataSymbol> =
        new.symbols.data_symbols.iter().map(|d| (d.name.as_str(), d)).collect();

    for (name, old_dat) in &old_data {
        if let Some(new_dat) = new_data.get(name) {
            if old_dat.offset != new_dat.offset {
                differences.push(Difference::new(
                    "symbols",
                    &format!("Data symbol '{}' offset mismatch", name),
                    Some(format!("{:?}", old_dat.offset)),
                    Some(format!("{:?}", new_dat.offset)),
                ));
            }
        } else {
            differences.push(Difference::new(
                "symbols",
                &format!("Data symbol '{}' missing in new parser", name),
                Some(format!("{:?}", old_dat)),
                None,
            ));
        }
    }

    for (name, new_dat) in &new_data {
        if !old_data.contains_key(name) {
            differences.push(Difference::new(
                "symbols",
                &format!("Data symbol '{}' only in new parser", name),
                None,
                Some(format!("{:?}", new_dat)),
            ));
        }
    }
}

fn compare_modules(
    old: &OldPdbData,
    new: &NewPdbData,
    differences: &mut Vec<Difference>,
) {
    debug!("Comparing modules in detail...");

    use std::collections::HashMap;
    let old_modules: HashMap<&str, &crate::old_pdb::ModuleInfo> =
        old.modules.iter().map(|m| (m.name.as_str(), m)).collect();
    let new_modules: HashMap<&str, &crate::new_pdb::ModuleInfo> =
        new.modules.iter().map(|m| (m.name.as_str(), m)).collect();

    for (name, old_mod) in &old_modules {
        if let Some(new_mod) = new_modules.get(name) {
            if old_mod.object_file != new_mod.object_file {
                differences.push(Difference::new(
                    "modules",
                    &format!("Module '{}' object file mismatch", name),
                    Some(old_mod.object_file.clone()),
                    Some(new_mod.object_file.clone()),
                ));
            }
        } else {
            differences.push(Difference::new(
                "modules",
                &format!("Module '{}' missing in new parser", name),
                Some(format!("{:?}", old_mod)),
                None,
            ));
        }
    }

    for (name, new_mod) in &new_modules {
        if !old_modules.contains_key(name) {
            differences.push(Difference::new(
                "modules",
                &format!("Module '{}' only in new parser", name),
                None,
                Some(format!("{:?}", new_mod)),
            ));
        }
    }
}
