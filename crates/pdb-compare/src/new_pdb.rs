use anyhow::{Context, Result};
use ezpdb;
use log::debug;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize)]
pub struct NewPdbData {
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
    pub fields: Vec<FieldInfo>,
    pub variants: Vec<VariantInfo>,
    pub base_classes: Vec<BaseClassInfo>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FieldInfo {
    pub name: String,
    pub offset: Option<u64>,
    pub type_index: Option<u32>,
    pub type_kind: Option<String>,
    pub bitfield_length: Option<usize>,
    pub bitfield_position: Option<usize>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct VariantInfo {
    pub name: String,
    pub value: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BaseClassInfo {
    pub name: Option<String>,
    pub offset: u64,
    pub type_index: Option<u32>,
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

pub fn parse_pdb_new(path: &Path) -> Result<NewPdbData> {
    debug!("Parsing PDB with ezpdb...");
    // ezpdb::parse_pdb takes a path and an optional base_address
    let pdb_info = ezpdb::parse_pdb(path, None).context("Failed to parse PDB with ezpdb")?;

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
        .filter_map(|(index, type_ref)| {
            // Borrow the TypeRef to access the Type
            let borrowed = type_ref.borrow();

            let mut fields = Vec::new();
            let mut variants = Vec::new();
            let mut base_classes = Vec::new();

            let (kind, name, size) = match &*borrowed {
                ezpdb::type_info::Type::Primitive(p) => {
                    (format!("Primitive({:?})", p.kind), None, None)
                }
                ezpdb::type_info::Type::Class(c) => {
                    // Extract fields from Class
                    for field_ref in &c.fields {
                        let field_borrowed = field_ref.borrow();
                        match &*field_borrowed {
                            ezpdb::type_info::Type::Member(m) => {
                                // Extract bitfield info if the member's type is a bitfield
                                let member_type_borrowed = m.underlying_type.borrow();
                                let (type_kind, bitfield_length, bitfield_position) =
                                    match &*member_type_borrowed {
                                        ezpdb::type_info::Type::Bitfield(bf) => (
                                            Some("Bitfield".to_string()),
                                            Some(bf.len),
                                            Some(bf.position),
                                        ),
                                        ezpdb::type_info::Type::Primitive(p) => {
                                            (Some(format!("Primitive({:?})", p.kind)), None, None)
                                        }
                                        ezpdb::type_info::Type::Pointer(_) => {
                                            (Some("Pointer".to_string()), None, None)
                                        }
                                        ezpdb::type_info::Type::Class(c) => {
                                            (Some(format!("Class({})", c.name)), None, None)
                                        }
                                        ezpdb::type_info::Type::Union(u) => {
                                            (Some(format!("Union({})", u.name)), None, None)
                                        }
                                        ezpdb::type_info::Type::Enumeration(e) => {
                                            (Some(format!("Enumeration({})", e.name)), None, None)
                                        }
                                        ezpdb::type_info::Type::Array(_) => {
                                            (Some("Array".to_string()), None, None)
                                        }
                                        _ => (Some("Other".to_string()), None, None),
                                    };

                                fields.push(FieldInfo {
                                    name: m.name.clone(),
                                    offset: Some(m.offset as u64),
                                    type_index: None,
                                    type_kind,
                                    bitfield_length,
                                    bitfield_position,
                                });
                            }
                            ezpdb::type_info::Type::BaseClass(b) => {
                                base_classes.push(BaseClassInfo {
                                    name: None,
                                    offset: b.offset as u64,
                                    type_index: None,
                                });
                            }
                            ezpdb::type_info::Type::VirtualBaseClass(vb) => {
                                base_classes.push(BaseClassInfo {
                                    name: None,
                                    offset: vb.base_pointer_offset as u64,
                                    type_index: None,
                                });
                            }
                            _ => {}
                        }
                    }
                    ("Class".to_string(), Some(c.name.clone()), Some(c.size))
                }
                ezpdb::type_info::Type::Union(u) => {
                    // Extract fields from Union
                    for field_ref in &u.fields {
                        let field_borrowed = field_ref.borrow();
                        if let ezpdb::type_info::Type::Member(m) = &*field_borrowed {
                            // Extract bitfield info if the member's type is a bitfield
                            let member_type_borrowed = m.underlying_type.borrow();
                            let (type_kind, bitfield_length, bitfield_position) =
                                match &*member_type_borrowed {
                                    ezpdb::type_info::Type::Bitfield(bf) => (
                                        Some("Bitfield".to_string()),
                                        Some(bf.len),
                                        Some(bf.position),
                                    ),
                                    ezpdb::type_info::Type::Primitive(p) => {
                                        (Some(format!("Primitive({:?})", p.kind)), None, None)
                                    }
                                    ezpdb::type_info::Type::Pointer(_) => {
                                        (Some("Pointer".to_string()), None, None)
                                    }
                                    ezpdb::type_info::Type::Class(c) => {
                                        (Some(format!("Class({})", c.name)), None, None)
                                    }
                                    ezpdb::type_info::Type::Union(u) => {
                                        (Some(format!("Union({})", u.name)), None, None)
                                    }
                                    ezpdb::type_info::Type::Enumeration(e) => {
                                        (Some(format!("Enumeration({})", e.name)), None, None)
                                    }
                                    ezpdb::type_info::Type::Array(_) => {
                                        (Some("Array".to_string()), None, None)
                                    }
                                    _ => (Some("Other".to_string()), None, None),
                                };

                            fields.push(FieldInfo {
                                name: m.name.clone(),
                                offset: Some(m.offset as u64),
                                type_index: None,
                                type_kind,
                                bitfield_length,
                                bitfield_position,
                            });
                        }
                    }
                    ("Union".to_string(), Some(u.name.clone()), Some(u.size))
                }
                ezpdb::type_info::Type::Enumeration(e) => {
                    // Extract variants from Enumeration
                    for variant in &e.variants {
                        // Convert VariantValue to i64 for comparison
                        let value = match variant.value {
                            ezpdb::type_info::VariantValue::U8(v) => v as i64,
                            ezpdb::type_info::VariantValue::U16(v) => v as i64,
                            ezpdb::type_info::VariantValue::U32(v) => v as i64,
                            ezpdb::type_info::VariantValue::U64(v) => v as i64,
                            ezpdb::type_info::VariantValue::I8(v) => v as i64,
                            ezpdb::type_info::VariantValue::I16(v) => v as i64,
                            ezpdb::type_info::VariantValue::I32(v) => v as i64,
                            ezpdb::type_info::VariantValue::I64(v) => v,
                        };
                        variants.push(VariantInfo {
                            name: variant.name.clone(),
                            value,
                        });
                    }
                    ("Enumeration".to_string(), Some(e.name.clone()), None)
                }
                ezpdb::type_info::Type::Pointer(_) => ("Pointer".to_string(), None, None),
                ezpdb::type_info::Type::Array(_) => ("Array".to_string(), None, None),
                ezpdb::type_info::Type::Procedure(_) => ("Procedure".to_string(), None, None),
                ezpdb::type_info::Type::MemberFunction(_) => {
                    ("MemberFunction".to_string(), None, None)
                }
                ezpdb::type_info::Type::BaseClass(_) => ("BaseClass".to_string(), None, None),
                ezpdb::type_info::Type::VirtualBaseClass(_) => {
                    ("VirtualBaseClass".to_string(), None, None)
                }
                ezpdb::type_info::Type::VTable(_) => ("VTable".to_string(), None, None),
                ezpdb::type_info::Type::Member(m) => {
                    ("Member".to_string(), Some(m.name.clone()), None)
                }
                ezpdb::type_info::Type::StaticMember(m) => {
                    ("StaticMember".to_string(), Some(m.name.clone()), None)
                }
                ezpdb::type_info::Type::Nested(n) => {
                    ("Nested".to_string(), Some(n.name.clone()), None)
                }
                ezpdb::type_info::Type::EnumVariant(v) => {
                    ("EnumVariant".to_string(), Some(v.name.clone()), None)
                }
                ezpdb::type_info::Type::OverloadedMethod(m) => {
                    ("OverloadedMethod".to_string(), Some(m.name.clone()), None)
                }
                ezpdb::type_info::Type::MethodListEntry(_) => {
                    ("MethodListEntry".to_string(), None, None)
                }
                ezpdb::type_info::Type::Method(m) => {
                    ("Method".to_string(), Some(m.name.clone()), None)
                }
                ezpdb::type_info::Type::Bitfield(_) => ("Bitfield".to_string(), None, None),
                // Filter out FieldList, MethodList, and ArgumentList as they are implementation details
                // that shouldn't be compared as top-level types. These are not meaningful standalone types.
                ezpdb::type_info::Type::FieldList(_) => return None,
                ezpdb::type_info::Type::ArgumentList(_) => return None,
                ezpdb::type_info::Type::MethodList(_) => return None,
                ezpdb::type_info::Type::Modifier(_) => ("Modifier".to_string(), None, None),
                // IPI stream types
                ezpdb::type_info::Type::FuncId(_) => ("FuncId".to_string(), None, None),
                ezpdb::type_info::Type::MFuncId(_) => ("MFuncId".to_string(), None, None),
                ezpdb::type_info::Type::StringId(_) => ("StringId".to_string(), None, None),
                ezpdb::type_info::Type::SubStrList(_) => ("SubStrList".to_string(), None, None),
                ezpdb::type_info::Type::BuildInfoType(_) => {
                    ("BuildInfoType".to_string(), None, None)
                }
                ezpdb::type_info::Type::UdtSrcLineType(_) => {
                    ("UdtSrcLineType".to_string(), None, None)
                }
                ezpdb::type_info::Type::Alias(_) => ("Alias".to_string(), None, None),
                ezpdb::type_info::Type::VTableShape(_) => ("VTableShape".to_string(), None, None),
                ezpdb::type_info::Type::VFTableType(_) => ("VFTableType".to_string(), None, None),
            };

            Some(TypeInfo {
                index: *index,
                kind,
                name,
                size: size.map(|s| s as u64),
                fields,
                variants,
                base_classes,
            })
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
            offset: s.rva.map(|o| o as u32),
            is_function: s.is_function,
        })
        .collect();

    let procedures = pdb_info
        .procedures
        .into_iter()
        .map(|p| ProcedureSymbol {
            name: p.name,
            offset: p.rva.map(|a| a as u32),
            len: p.len as u32,
        })
        .collect();

    let data_symbols = pdb_info
        .global_data
        .into_iter()
        .map(|d| DataSymbol {
            name: d.name,
            offset: d.rva.map(|o| o as u32),
        })
        .collect();

    let symbols = SymbolData {
        public_symbols,
        procedures,
        data_symbols,
    };

    // Extract modules
    debug!("Extracting {} modules", pdb_info.debug_modules.len());
    let modules = pdb_info
        .debug_modules
        .into_iter()
        .map(|m| ModuleInfo {
            name: m.name,
            object_file: m.object_file_name,
        })
        .collect();

    Ok(NewPdbData {
        header,
        types,
        symbols,
        modules,
    })
}
