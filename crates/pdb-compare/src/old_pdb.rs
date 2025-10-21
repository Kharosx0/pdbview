use anyhow::{Context, Result};
use ezpdb_old;
use log::debug;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize)]
pub struct OldPdbData {
    pub header: HeaderInfo,
    pub types: Vec<TypeInfo>,
    pub symbols: SymbolData,
    pub modules: Vec<ModuleInfo>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HeaderInfo {
    pub age: u32,
    pub guid: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TypeInfo {
    pub index: u32,
    pub kind: String,
    pub name: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SymbolData {
    pub public_symbols: Vec<PublicSymbol>,
    pub procedures: Vec<ProcedureSymbol>,
    pub data_symbols: Vec<DataSymbol>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PublicSymbol {
    pub name: String,
    pub offset: Option<u32>,
    pub is_function: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProcedureSymbol {
    pub name: String,
    pub offset: Option<u32>,
    pub len: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DataSymbol {
    pub name: String,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ModuleInfo {
    pub name: String,
    pub object_file: String,
}

pub fn parse_pdb(path: &Path) -> Result<OldPdbData> {
    debug!("Parsing PDB with ezpdb-old (pdb crate 0.8)...");
    // ezpdb::parse_pdb takes a path and an optional base_address
    let pdb_info = ezpdb_old::parse_pdb(path, None).context("Failed to parse PDB with ezpdb-old")?;

    // Extract header information
    let header = HeaderInfo {
        age: pdb_info.age,
        guid: format!("{:?}", pdb_info.guid),
    };

    // Extract type information
    debug!("Extracting {} types", pdb_info.types.len());
    let types = pdb_info
        .types
        .iter()
        .map(|(index, type_ref)| {
            // Borrow the TypeRef to access the Type
            let borrowed = type_ref.borrow();
            let (kind, name, size) = match &*borrowed {
                ezpdb_old::type_info::Type::Primitive(p) => {
                    (format!("Primitive({:?})", p.kind), None, None)
                }
                ezpdb_old::type_info::Type::Class(c) => {
                    ("Class".to_string(), Some(c.name.clone()), Some(c.size))
                }
                ezpdb_old::type_info::Type::Union(u) => {
                    ("Union".to_string(), Some(u.name.clone()), Some(u.size))
                }
                ezpdb_old::type_info::Type::Enumeration(e) => {
                    ("Enumeration".to_string(), Some(e.name.clone()), None)
                }
                ezpdb_old::type_info::Type::Pointer(_) => ("Pointer".to_string(), None, None),
                ezpdb_old::type_info::Type::Array(_) => ("Array".to_string(), None, None),
                ezpdb_old::type_info::Type::Procedure(_) => ("Procedure".to_string(), None, None),
                ezpdb_old::type_info::Type::MemberFunction(_) => ("MemberFunction".to_string(), None, None),
                ezpdb_old::type_info::Type::BaseClass(_) => ("BaseClass".to_string(), None, None),
                ezpdb_old::type_info::Type::VirtualBaseClass(_) => {
                    ("VirtualBaseClass".to_string(), None, None)
                }
                ezpdb_old::type_info::Type::VTable(_) => {
                    ("VTable".to_string(), None, None)
                }
                ezpdb_old::type_info::Type::Member(m) => {
                    ("Member".to_string(), Some(m.name.clone()), None)
                }
                ezpdb_old::type_info::Type::StaticMember(m) => {
                    ("StaticMember".to_string(), Some(m.name.clone()), None)
                }
                ezpdb_old::type_info::Type::Nested(n) => {
                    ("Nested".to_string(), Some(n.name.clone()), None)
                }
                ezpdb_old::type_info::Type::EnumVariant(v) => {
                    ("EnumVariant".to_string(), Some(v.name.clone()), None)
                }
                ezpdb_old::type_info::Type::OverloadedMethod(m) => {
                    ("OverloadedMethod".to_string(), Some(m.name.clone()), None)
                }
                ezpdb_old::type_info::Type::MethodListEntry(_) => {
                    ("MethodListEntry".to_string(), None, None)
                }
                ezpdb_old::type_info::Type::Method(m) => {
                    ("Method".to_string(), Some(m.name.clone()), None)
                }
                ezpdb_old::type_info::Type::Bitfield(_) => ("Bitfield".to_string(), None, None),
                ezpdb_old::type_info::Type::FieldList(_) => ("FieldList".to_string(), None, None),
                ezpdb_old::type_info::Type::ArgumentList(_) => ("ArgumentList".to_string(), None, None),
                ezpdb_old::type_info::Type::MethodList(_) => ("MethodList".to_string(), None, None),
                ezpdb_old::type_info::Type::Modifier(_) => ("Modifier".to_string(), None, None),
            };

            TypeInfo {
                index: *index,
                kind,
                name,
                size: size.map(|s| s as u64),
            }
        })
        .collect();

    // Extract symbols
    debug!(
        "Extracting {} public symbols, {} procedures, {} data symbols",
        pdb_info.public_symbols.len(),
        pdb_info.procedures.len(),
        pdb_info.global_data.len()
    );

    let public_symbols = pdb_info
        .public_symbols
        .into_iter()
        .map(|s| PublicSymbol {
            name: s.name,
            offset: s.offset.map(|o| o as u32),
            is_function: s.is_function,
        })
        .collect();

    let procedures = pdb_info
        .procedures
        .into_iter()
        .map(|p| ProcedureSymbol {
            name: p.name,
            offset: p.address.map(|a| a as u32),
            len: p.len as u32,
        })
        .collect();

    let data_symbols = pdb_info
        .global_data
        .into_iter()
        .map(|d| DataSymbol {
            name: d.name,
            offset: d.offset.map(|o| o as u32),
        })
        .collect();

    let symbols = SymbolData {
        public_symbols,
        procedures,
        data_symbols,
    };

    // Extract modules
    // Note: The old ezpdb version on GitHub has private fields for DebugModule,
    // so we extract just the count for now. The real comparison is about symbols/types anyway.
    debug!("Extracting {} modules (limited info due to private fields)", pdb_info.debug_modules.len());
    
    // Since we can't access the name/object_file_name fields (they're private in old ezpdb),
    // we'll create placeholder entries. This won't give us detailed module comparison,
    // but the important comparisons are symbols and types.
    let modules = pdb_info
        .debug_modules
        .iter()
        .enumerate()
        .map(|(i, _m)| ModuleInfo {
            name: format!("<module_{}>", i),
            object_file: format!("<object_{}>", i),
        })
        .collect();

    Ok(OldPdbData {
        header,
        types,
        symbols,
        modules,
    })
}
