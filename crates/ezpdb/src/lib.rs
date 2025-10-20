use crate::error::Error;
use crate::symbol_types::*;
use log::{debug, warn};
use ms_pdb::{
    Pdb,
    codeview::{
        syms::{Sym, SymData},
        types::{TypeData, TypeIndex},
    },
};
use std::cell::RefCell;
use std::convert::TryInto;
use std::fs::File;
use std::path::Path;
use std::rc::Rc;

pub mod error;
pub mod symbol_types;
pub mod type_info;

pub use crate::symbol_types::ParsedPdb;

// Helper function to convert version
fn convert_version(version: u32) -> Version {
    match version {
        20000404 => Version::V41,
        20040203 => Version::V50,
        // Add other version mappings as needed
        _ => Version::Other(version),
    }
}

pub fn parse_pdb<P: AsRef<Path>>(
    path: P,
    base_address: Option<usize>,
) -> Result<ParsedPdb, crate::error::Error> {
    let file = File::open(path.as_ref())?;
    debug!("opening PDB");
    let pdb = Pdb::open_from_file(file)?;

    let mut output_pdb = ParsedPdb::new(path.as_ref().to_owned());
    let dbi_header = pdb.dbi_header();
    let pdbi = pdb.pdbi();
    
    // Machine type handling
    output_pdb.machine_type = match dbi_header.machine.get() {
        0 => Some(MachineType::Unknown),
        0x14c => Some(MachineType::X86),
        0x8664 => Some(MachineType::Amd64),
        0xAA64 => Some(MachineType::Arm64),
        0x1c0 => Some(MachineType::Arm),
        0x1c4 => Some(MachineType::ArmNT),
        _ => None,
    };

    output_pdb.age = dbi_header.age.get();
    output_pdb.guid = pdbi.binding_key().guid;
    output_pdb.timestamp = pdbi.signature;  // Field, not method
    output_pdb.version = convert_version(pdbi.version());

    debug!("fetching ID information");
    // Read IPI stream for ID information (build info, etc.)
    let ipi_stream = match pdb.read_ipi_stream() {
        Ok(stream) => {
            debug!("ID information header was valid");
            Some(stream)
        }
        Err(e) => {
            warn!("error when fetching ipi stream: {}. ID information and symbols depending on such will not be loaded", e);
            None
        }
    };

    debug!("grabbing type information");
    // Parse type information first. Some symbol info (such as function signatures) depends
    // upon type information, but not vice versa
    let type_stream = pdb.read_type_stream()?;
    let type_index_begin = type_stream.type_index_begin();
    let _type_index_end = type_stream.type_index_end();
    
    let mut discovered_types = vec![];
    for _type_record in type_stream.iter_type_records() {
        let type_index = TypeIndex(type_index_begin.0 + discovered_types.len() as u32);
        discovered_types.push(type_index);
    }

    for typ_idx in discovered_types.iter() {
        let _typ = match handle_type(*typ_idx, &mut output_pdb, &type_stream) {
            Ok(typ) => typ,
            Err(e) => {
                // Log but continue - some types may be unimplemented
                warn!("Could not parse type {:?}: {}", typ_idx, e);
                continue;
            }
        };
    }

    // Iterate through all of the parsed types once just to update any necessary info
    for typ in output_pdb.types.values() {
        use crate::type_info::Typed;
        typ.as_ref().borrow_mut().on_complete(&output_pdb);
    }

    debug!("grabbing public symbols");
    // Parse public symbols
    let gss = pdb.read_gss()?;
    for sym in gss.iter_syms() {
        if let Err(e) = handle_symbol(
            sym,
            &mut output_pdb,
            &type_stream,
            ipi_stream.as_ref(),
            base_address,
        ) {
            warn!("Error handling symbol: {}", e);
        }
    }

    debug!("grabbing debug modules");
    // Parse private symbols from modules
    let dbi_stream = pdb.read_dbi_stream()?;
    
    for module in dbi_stream.iter_modules() {
        let module_name = module.module_name;
        
        // Store module info
        output_pdb.debug_modules.push(DebugModule {
            name: String::from_utf8_lossy(module.module_name).into_owned(),
            object_file_name: String::from_utf8_lossy(module.obj_file).into_owned(),
            source_files: None, // TODO: Parse source files if needed
        });
        
        // Read module symbols
        if let Ok(Some(modi_stream)) = pdb.read_module_stream(&module) {
            debug!("grabbing symbols for module: {}", String::from_utf8_lossy(module_name));
            
            for sym in modi_stream.iter_syms() {
                if let Err(e) = handle_symbol(
                    sym,
                    &mut output_pdb,
                    &type_stream,
                    ipi_stream.as_ref(),
                    base_address,
                ) {
                    warn!("Error handling symbol in module: {}", e);
                }
            }
        }
    }

    Ok(output_pdb)
}

/// Helper to convert symbol data to parsed representation
fn handle_symbol<'a>(
    sym: Sym<'a>,
    output_pdb: &mut ParsedPdb,
    type_stream: &ms_pdb::tpi::TypeStream<Vec<u8>>,
    ipi_stream: Option<&ms_pdb::tpi::TypeStream<Vec<u8>>>,
    base_address: Option<usize>,
) -> Result<(), Error> {
    let base_address = base_address.unwrap_or(0);
    
    // Parse the symbol
    let sym_data = match sym.parse() {
        Ok(data) => data,
        Err(e) => {
            warn!("Failed to parse symbol: {}", e);
            return Ok(()); // Non-fatal
        }
    };

    match sym_data {
        SymData::Pub(data) => {
            debug!("public symbol: {:?}", data);
            let converted_symbol: crate::symbol_types::PublicSymbol =
                (&data, base_address, type_stream).into();
            output_pdb.public_symbols.push(converted_symbol);
        }
        SymData::Proc(data) => {
            debug!("procedure: {:?}", data);
            let is_global = matches!(sym.kind, ms_pdb::codeview::syms::SymKind::S_GPROC32);
            let is_dpc = matches!(sym.kind, ms_pdb::codeview::syms::SymKind::S_LPROC32_DPC);
            let mut converted_symbol: crate::symbol_types::Procedure =
                (&data, base_address, type_stream).into();
            converted_symbol.is_global = is_global;
            converted_symbol.is_dpc = is_dpc;
            output_pdb.procedures.push(converted_symbol);
        }
        SymData::BuildInfo(data) => {
            debug!("build info: {:?}", data);
            if let Some(ipi) = ipi_stream {
                let converted_symbol: crate::symbol_types::BuildInfo = 
                    (&data, ipi).try_into()?;
                output_pdb.assembly_info.build_info = Some(converted_symbol);
            }
        }
        // Note: CompileFlags not available in ms-pdb, skipping
        SymData::Data(data) => {
            let is_global = matches!(sym.kind, ms_pdb::codeview::syms::SymKind::S_GDATA32 | ms_pdb::codeview::syms::SymKind::S_GMANDATA);
            let mut sym: crate::symbol_types::Data =
                (&data, base_address, type_stream, &output_pdb.types).try_into()?;
            sym.is_global = is_global;
            if sym.is_global {
                output_pdb.global_data.push(sym);
            }
        }
        _ => {
            // Many symbol types we don't handle yet
        }
    }

    Ok(())
}

/// Converts a type index to our internal type representation
pub(crate) fn handle_type(
    idx: TypeIndex,
    output_pdb: &mut ParsedPdb,
    type_stream: &ms_pdb::tpi::TypeStream<Vec<u8>>,
) -> Result<TypeRef, Error> {
    if let Some(typ) = output_pdb.types.get(&idx.0) {
        return Ok(Rc::clone(typ));
    }

    // Get the type record from the stream
    let type_record = type_stream.record(idx)?;
    let parsed_type = type_record.parse().map_err(|e| anyhow::anyhow!("Failed to parse type record: {:?}", e))?;
    let typ = handle_type_data(&parsed_type, output_pdb, type_stream)?;

    output_pdb.types.insert(idx.0, Rc::clone(&typ));

    Ok(typ)
}

pub(crate) fn handle_type_data(
    typ: &TypeData,
    output_pdb: &mut ParsedPdb,
    type_stream: &ms_pdb::tpi::TypeStream<Vec<u8>>,
) -> Result<TypeRef, Error> {
    use crate::type_info::Type;
    
    let result_typ = match typ {
        TypeData::Struct(data) => {
            let typ = (data, type_stream, output_pdb).try_into()?;
            Type::Class(typ)
        }
        TypeData::Union(data) => {
            let typ = (data, type_stream, output_pdb).try_into()?;
            Type::Union(typ)
        }
        TypeData::Array(data) => {
            let typ = (data, type_stream, output_pdb).try_into()?;
            Type::Array(typ)
        }
        TypeData::Enum(data) => {
            let typ = (data, type_stream, output_pdb).try_into()?;
            Type::Enumeration(typ)
        }
        TypeData::Pointer(data) => {
            let typ = (data, type_stream, output_pdb).try_into()?;
            Type::Pointer(typ)
        }
        TypeData::Modifier(data) => {
            let typ = (data, type_stream, output_pdb).try_into()?;
            Type::Modifier(typ)
        }
        TypeData::FieldList(data) => {
            let typ = (data, type_stream, output_pdb).try_into()?;
            Type::FieldList(typ)
        }
        TypeData::ArgList(data) => {
            let typ = (data, type_stream, output_pdb).try_into()?;
            Type::ArgumentList(typ)
        }
        TypeData::Proc(data) => {
            let typ = (*data, type_stream, output_pdb).try_into()?;
            Type::Procedure(typ)
        }
        TypeData::MemberFunc(data) => {
            let typ = (*data, type_stream, output_pdb).try_into()?;
            Type::MemberFunction(typ)
        }
        TypeData::MethodList(data) => {
            let typ = (data, type_stream, output_pdb).try_into()?;
            Type::MethodList(typ)
        }
        _ => {
            warn!("Unhandled type: {:?}", typ);
            // Return a placeholder for unhandled types
            return Err(Error::UnhandledType(format!("{:?}", typ)));
        }
    };

    Ok(Rc::new(RefCell::new(result_typ)))
}
