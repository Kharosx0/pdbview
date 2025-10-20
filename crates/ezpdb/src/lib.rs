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
    let type_index_end = type_stream.type_index_end();
    
    eprintln!("🔍 Type stream range: {} to {} ({} types expected)", 
           type_index_begin.0, type_index_end.0, type_index_end.0 - type_index_begin.0);
    
    let mut discovered_types = vec![];
    let mut current_index = type_index_begin.0;
    for _type_record in type_stream.iter_type_records() {
        let type_index = TypeIndex(current_index);
        discovered_types.push(type_index);
        current_index += 1;
    }
    
    eprintln!("🔍 Actually iterated {} type records (expected {})", 
          discovered_types.len(), type_index_end.0 - type_index_begin.0);

    let mut parse_errors = 0;
    let mut primitive_skipped = 0;
    for typ_idx in discovered_types.iter() {
        // Skip primitive types (< type_index_begin) - these are built-in types like int, char, etc.
        // ms-pdb will error if we try to get a record for these
        if typ_idx.0 < type_index_begin.0 {
            primitive_skipped += 1;
            continue;
        }
        
        let _typ = match handle_type(*typ_idx, &mut output_pdb, &type_stream) {
            Ok(typ) => typ,
            Err(e) => {
                // Log errors
                parse_errors += 1;
                if parse_errors <= 10 || typ_idx.0 == 6219 {
                    eprintln!("⚠️  Could not parse type {:?}: {}", typ_idx, e);
                } else if parse_errors == 11 {
                    eprintln!("⚠️  (suppressing further error messages, total will be shown at end)");
                }
                continue;
            }
        };
    }
    
    eprintln!("🔍 Successfully parsed {} types, {} primitives skipped, {} failed", 
              output_pdb.types.len(), primitive_skipped, parse_errors);
    

    // Parse IPI (ID Program Information) stream into separate HashMap
    // IPI contains FuncId, StringId, BuildInfo, etc. - metadata not in TPI
    // Store in ipi_types to prevent collision with TPI types (both use same TypeIndex range)
    if let Some(ref ipi_stream) = ipi_stream {
        let mut ipi_discovered_types = vec![];
        let ipi_type_index_begin = ipi_stream.type_index_begin();
        
        // Discover all IPI type indices
        for type_index in ipi_type_index_begin.0..ipi_stream.type_index_end().0 {
            ipi_discovered_types.push(TypeIndex(type_index));
        }
        
        eprintln!("🔍 Parsing {} IPI types...", ipi_discovered_types.len());
        
        let mut ipi_parse_errors = 0;
        let mut ipi_primitive_skipped = 0;
        for typ_idx in ipi_discovered_types.iter() {
            // Skip primitive types
            if typ_idx.0 < ipi_type_index_begin.0 {
                ipi_primitive_skipped += 1;
                continue;
            }
            
            // Parse IPI type and store in ipi_types HashMap
            let result = handle_ipi_type(*typ_idx, &mut output_pdb, ipi_stream);
            match result {
                Ok(_typ) => {},
                Err(e) => {
                    ipi_parse_errors += 1;
                    if ipi_parse_errors <= 10 {
                        eprintln!("⚠️  Could not parse IPI type {:?}: {}", typ_idx, e);
                    } else if ipi_parse_errors == 11 {
                        eprintln!("⚠️  (suppressing further IPI error messages)");
                    }
                    continue;
                }
            }
        }
        
        eprintln!("🔍 Successfully parsed {} IPI types, {} primitives skipped, {} failed", 
                  output_pdb.ipi_types.len(), ipi_primitive_skipped, ipi_parse_errors);
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

    // Check if this is a primitive type (built-in type like int, char, etc.)
    if type_stream.is_primitive(idx) {
        // For primitive types, we create a simple placeholder type
        // TODO: Properly decode primitive type information from TypeIndex encoding
        use crate::type_info::{Type, Primitive, PrimitiveKind};
        let primitive = Primitive {
            kind: PrimitiveKind::Void,  // Placeholder - should decode from idx
            indirection: None,
        };
        let typ = Rc::new(RefCell::new(Type::Primitive(primitive)));
        output_pdb.types.insert(idx.0, Rc::clone(&typ));
        return Ok(typ);
    }

    // Get the type record from the stream
    let type_record = type_stream.record(idx)?;
    
    let parsed_type = type_record.parse().map_err(|e| anyhow::anyhow!("Failed to parse type record: {:?}", e))?;
    
    let typ = handle_type_data(&parsed_type, output_pdb, type_stream)?;

    output_pdb.types.insert(idx.0, Rc::clone(&typ));

    Ok(typ)
}

/// Converts an IPI type index to our internal type representation (stored in ipi_types HashMap)
pub(crate) fn handle_ipi_type(
    idx: TypeIndex,
    output_pdb: &mut ParsedPdb,
    ipi_stream: &ms_pdb::tpi::TypeStream<Vec<u8>>,
) -> Result<TypeRef, Error> {
    // Check if already parsed
    if let Some(typ) = output_pdb.ipi_types.get(&idx.0) {
        return Ok(Rc::clone(typ));
    }

    // Check if this is a primitive type
    if ipi_stream.is_primitive(idx) {
        use crate::type_info::{Type, Primitive, PrimitiveKind};
        let primitive = Primitive {
            kind: PrimitiveKind::Void,
            indirection: None,
        };
        let typ = Rc::new(RefCell::new(Type::Primitive(primitive)));
        output_pdb.ipi_types.insert(idx.0, Rc::clone(&typ));
        return Ok(typ);
    }

    // Get the type record from the IPI stream
    let type_record = ipi_stream.record(idx)?;
    
    let parsed_type = type_record.parse().map_err(|e| anyhow::anyhow!("Failed to parse IPI type record: {:?}", e))?;
    
    // Convert using handle_type_data (same conversion logic, just store in ipi_types)
    let typ = handle_type_data(&parsed_type, output_pdb, ipi_stream)?;

    output_pdb.ipi_types.insert(idx.0, Rc::clone(&typ));

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
        // IPI stream types
        TypeData::FuncId(data) => {
            let typ = (data, type_stream, output_pdb).try_into()?;
            Type::FuncId(typ)
        }
        TypeData::MFuncId(data) => {
            let typ = (data, type_stream, output_pdb).try_into()?;
            Type::MFuncId(typ)
        }
        TypeData::StringId(data) => {
            let typ = (data, type_stream, output_pdb).try_into()?;
            Type::StringId(typ)
        }
        TypeData::SubStrList(data) => {
            let typ = (data, type_stream, output_pdb).try_into()?;
            Type::SubStrList(typ)
        }
        TypeData::BuildInfo(data) => {
            let typ = (data, type_stream, output_pdb).try_into()?;
            Type::BuildInfoType(typ)
        }
        TypeData::UdtSrcLine(data) => {
            let typ = (*data, type_stream, output_pdb).try_into()?;
            Type::UdtSrcLineType(typ)
        }
        TypeData::Unknown => {
            // Unknown types are not supported by ms-codeview - they represent type kinds
            // that the library doesn't recognize. Create a placeholder.
            warn!("Encountered Unknown type - creating placeholder");
            use crate::type_info::{Primitive, PrimitiveKind};
            let primitive = Primitive {
                kind: PrimitiveKind::Void,
                indirection: None,
            };
            Type::Primitive(primitive)
        }
        _ => {
            warn!("Unhandled type variant: {:?}", typ);
            // Return a placeholder for unhandled types
            return Err(Error::UnhandledType(format!("{:?}", typ)));
        }
    };

    Ok(Rc::new(RefCell::new(result_typ)))
}
