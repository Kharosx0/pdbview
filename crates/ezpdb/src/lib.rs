use crate::error::Error;
use crate::symbol_types::*;
use log::{debug, warn};
use ms_pdb::{
    codeview::{
        syms::{Sym, SymData},
        types::{TypeData, TypeIndex},
    },
    dbi::{optional_dbg::OptionalDebugHeaderStream, DbiStream},
    Pdb, Stream,
};
use std::cell::RefCell;
use std::convert::TryInto;
use std::fs::File;
use std::path::Path;
use std::rc::Rc;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

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

/// IMAGE_SECTION_HEADER from PE/COFF specification.
/// This is a 40-byte structure that describes a section in a PE file.
#[repr(C)]
#[derive(Debug, Clone, Copy, IntoBytes, FromBytes, KnownLayout, Immutable)]
struct ImageSectionHeader {
    name: [u8; 8],
    virtual_size: u32,
    virtual_address: u32,   // This is the RVA of the section
    size_of_raw_data: u32,
    pointer_to_raw_data: u32,
    pointer_to_relocations: u32,
    pointer_to_line_numbers: u32,
    number_of_relocations: u16,
    number_of_line_numbers: u16,
    characteristics: u32,
}

/// Converts a section:offset pair to an RVA (Relative Virtual Address).
/// 
/// This reads the IMAGE_SECTION_HEADER structures from the PDB's optional debug header
/// (stream index 5, `section_header_data`) to map section-relative offsets to RVAs.
/// 
/// Section indices in PDB symbols are 1-based.
fn section_offset_to_rva(pdb: &Pdb, section: u16, offset: u32) -> Option<usize> {
    // Read the DBI stream
    let dbi_data = pdb.read_stream_to_vec(Stream::DBI.into()).ok()?;
    let dbi = DbiStream::parse(dbi_data).ok()?;
    
    // Get the optional debug header
    let opt_dbg = dbi.optional_debug_header().ok()?;
    
    // Get the section header stream index
    let section_header_stream = opt_dbg.stream(OptionalDebugHeaderStream::section_header_data)?;
    
    // Read the section header stream
    let section_headers_data = pdb.read_stream_to_vec(section_header_stream).ok()?;
    
    // Parse as an array of IMAGE_SECTION_HEADER structures
    let section_headers = <[ImageSectionHeader]>::ref_from_bytes(&section_headers_data).ok()?;
    
    // Section indices are 1-based in PDB symbols
    if section == 0 || section as usize > section_headers.len() {
        return None;
    }
    
    let section_header = &section_headers[section as usize - 1];
    Some((section_header.virtual_address + offset) as usize)
}

/// Decodes a primitive TypeIndex into a Primitive type
/// TypeIndex encodes primitive types in the lower bits:
/// - Bits 0-7: Type kind (T_VOID, T_CHAR, T_INT4, etc.)
/// - Bits 8-11: Indirection mode (near16, far16, near32, ptr64, etc.)
fn decode_primitive_type(
    idx: ms_pdb::codeview::types::TypeIndex,
) -> Result<crate::type_info::Primitive, Error> {
    use crate::type_info::{Indirection, Primitive, PrimitiveKind};

    let type_value = idx.0;
    let kind_value = type_value & 0xFF;
    let mode_value = (type_value >> 8) & 0xF;

    // Decode primitive kind from CV_typ_e enum
    // See cvinfo.h from Microsoft's debug interface access SDK
    let kind = match kind_value {
        0x0000 => PrimitiveKind::NoType,
        0x0003 => PrimitiveKind::Void,
        0x0008 => PrimitiveKind::HRESULT,

        // Character types
        0x0010 => PrimitiveKind::Char,
        0x0020 => PrimitiveKind::Short,
        0x0021 => PrimitiveKind::UShort,
        0x0022 => PrimitiveKind::I16,
        0x0023 => PrimitiveKind::U16,

        // Boolean
        0x0030 => PrimitiveKind::Bool8,
        0x0031 => PrimitiveKind::Bool16,
        0x0033 => PrimitiveKind::Bool32,
        0x0034 => PrimitiveKind::Bool64,

        // Floating point
        0x0040 => PrimitiveKind::F32,
        0x0041 => PrimitiveKind::F32PP,
        0x0042 => PrimitiveKind::F64,
        0x0043 => PrimitiveKind::F128,
        0x0044 => PrimitiveKind::F48,
        0x0045 => PrimitiveKind::F80,
        0x0046 => PrimitiveKind::F16,

        // Complex
        0x0050 => PrimitiveKind::Complex32,
        0x0051 => PrimitiveKind::Complex64,
        0x0052 => PrimitiveKind::Complex128,
        0x0053 => PrimitiveKind::Complex80,

        // 8-bit types
        0x0068 => PrimitiveKind::I8,
        0x0069 => PrimitiveKind::U8,

        // Wide character types
        0x0070 => PrimitiveKind::RChar,
        0x0071 => PrimitiveKind::WChar,

        // 32-bit integers
        0x0072 => PrimitiveKind::I32,
        0x0073 => PrimitiveKind::U32,

        // Long types (platform-dependent)
        0x0074 => PrimitiveKind::Long,
        0x0075 => PrimitiveKind::ULong,

        // 64-bit integers
        0x0076 => PrimitiveKind::Quad,
        0x0077 => PrimitiveKind::UQuad,

        // Unicode character types
        0x007a => PrimitiveKind::RChar16,
        0x007b => PrimitiveKind::RChar32,

        // Explicit sized types
        0x0013 => PrimitiveKind::I64,
        0x0014 => PrimitiveKind::Octa,
        0x0015 => PrimitiveKind::UOcta,
        0x0017 => PrimitiveKind::I128,
        0x0018 => PrimitiveKind::U128,

        _ => {
            warn!(
                "Unknown primitive type kind: 0x{:04x}, defaulting to Void",
                kind_value
            );
            PrimitiveKind::Void
        }
    };

    // Decode indirection mode
    let indirection = match mode_value {
        0x0 => None, // Direct
        0x1 => Some(Indirection::Near16),
        0x2 => Some(Indirection::Far16),
        0x3 => Some(Indirection::Huge16),
        0x4 => Some(Indirection::Near32),
        0x5 => Some(Indirection::Far32),
        0x6 => Some(Indirection::Near64),
        0x7 => Some(Indirection::Near128),
        _ => {
            warn!(
                "Unknown indirection mode: 0x{:x}, defaulting to None",
                mode_value
            );
            None
        }
    };

    Ok(Primitive { kind, indirection })
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
    output_pdb.timestamp = pdbi.signature; // Field, not method
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
    let mut current_index = type_index_begin.0;
    for _type_record in type_stream.iter_type_records() {
        let type_index = TypeIndex(current_index);
        discovered_types.push(type_index);
        current_index += 1;
    }

    let mut parse_errors = 0;
    let mut _primitive_skipped = 0;
    for typ_idx in discovered_types.iter() {
        // Skip primitive types (< type_index_begin) - these are built-in types like int, char, etc.
        // ms-pdb will error if we try to get a record for these
        if typ_idx.0 < type_index_begin.0 {
            _primitive_skipped += 1;
            continue;
        }

        let _typ = match handle_type(*typ_idx, &mut output_pdb, &type_stream) {
            Ok(typ) => typ,
            Err(e) => {
                // Log errors
                parse_errors += 1;
                if parse_errors <= 10 || typ_idx.0 == 6219 {
                    warn!("Could not parse type {:?}: {}", typ_idx, e);
                } else if parse_errors == 11 {
                    warn!("(suppressing further error messages, total will be shown at end)");
                }
                continue;
            }
        };
    }

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

        let mut ipi_parse_errors = 0;
        let mut _ipi_primitive_skipped = 0;
        for typ_idx in ipi_discovered_types.iter() {
            // Skip primitive types
            if typ_idx.0 < ipi_type_index_begin.0 {
                _ipi_primitive_skipped += 1;
                continue;
            }

            // Parse IPI type and store in ipi_types HashMap
            let result = handle_ipi_type(*typ_idx, &mut output_pdb, ipi_stream);
            match result {
                Ok(_typ) => {}
                Err(e) => {
                    ipi_parse_errors += 1;
                    if ipi_parse_errors <= 10 {
                        warn!("Could not parse IPI type {:?}: {}", typ_idx, e);
                    } else if ipi_parse_errors == 11 {
                        warn!("(suppressing further IPI error messages)");
                    }
                    continue;
                }
            }
        }
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
            &pdb,
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

        // Extract source files from module stream
        // Note: This requires parsing C13 line data and resolving name indexes through the
        // names stream. The ms-pdb library doesn't currently provide a high-level API for this,
        // so we'd need to manually parse FILE_CHECKSUMS subsection and resolve names.
        // For now, we leave source_files as None to avoid incomplete implementation.
        let source_files = None;

        // Store module info
        output_pdb.debug_modules.push(DebugModule {
            name: String::from_utf8_lossy(module.module_name).into_owned(),
            object_file_name: String::from_utf8_lossy(module.obj_file).into_owned(),
            source_files,
        });

        // Read module symbols
        if let Ok(Some(modi_stream)) = pdb.read_module_stream(&module) {
            debug!(
                "grabbing symbols for module: {}",
                String::from_utf8_lossy(module_name)
            );

            for sym in modi_stream.iter_syms() {
                if let Err(e) = handle_symbol(
                    &pdb,
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
    pdb: &Pdb,
    sym: Sym<'a>,
    output_pdb: &mut ParsedPdb,
    type_stream: &ms_pdb::tpi::TypeStream<Vec<u8>>,
    ipi_stream: Option<&ms_pdb::tpi::TypeStream<Vec<u8>>>,
    base_address: Option<usize>,
) -> Result<(), Error> {
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
                (pdb, &data, base_address, type_stream).into();
            output_pdb.public_symbols.push(converted_symbol);
        }
        SymData::Proc(data) => {
            debug!("procedure: {:?}", data);
            let is_global = matches!(sym.kind, ms_pdb::codeview::syms::SymKind::S_GPROC32);
            let is_dpc = matches!(sym.kind, ms_pdb::codeview::syms::SymKind::S_LPROC32_DPC);
            let mut converted_symbol: crate::symbol_types::Procedure =
                (pdb, &data, base_address, type_stream).into();
            converted_symbol.is_global = is_global;
            converted_symbol.is_dpc = is_dpc;
            output_pdb.procedures.push(converted_symbol);
        }
        SymData::BuildInfo(data) => {
            debug!("build info: {:?}", data);
            if let Some(ipi) = ipi_stream {
                let converted_symbol: crate::symbol_types::BuildInfo = (&data, ipi).try_into()?;
                output_pdb.assembly_info.build_info = Some(converted_symbol);
            }
        }
        // Note: CompileFlags not available in ms-pdb, skipping
        SymData::Data(data) => {
            let is_global = matches!(
                sym.kind,
                ms_pdb::codeview::syms::SymKind::S_GDATA32
                    | ms_pdb::codeview::syms::SymKind::S_GMANDATA
            );
            let is_managed = matches!(
                sym.kind,
                ms_pdb::codeview::syms::SymKind::S_GMANDATA
                    | ms_pdb::codeview::syms::SymKind::S_LMANDATA
            );
            let mut sym: crate::symbol_types::Data =
                (pdb, &data, base_address, type_stream, &output_pdb.types).try_into()?;
            sym.is_global = is_global;
            sym.is_managed = is_managed;
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
        // For primitive types, decode the TypeIndex to extract primitive information
        use crate::type_info::Type;
        let primitive = decode_primitive_type(idx)?;
        let typ = Rc::new(RefCell::new(Type::Primitive(primitive)));
        output_pdb.types.insert(idx.0, Rc::clone(&typ));
        return Ok(typ);
    }

    // Get the type record from the stream
    let type_record = type_stream.record(idx)?;

    let parsed_type = type_record
        .parse()
        .map_err(|e| anyhow::anyhow!("Failed to parse type record: {:?}", e))?;

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
        use crate::type_info::Type;
        let primitive = decode_primitive_type(idx)?;
        let typ = Rc::new(RefCell::new(Type::Primitive(primitive)));
        output_pdb.ipi_types.insert(idx.0, Rc::clone(&typ));
        return Ok(typ);
    }

    // Get the type record from the IPI stream
    let type_record = ipi_stream.record(idx)?;

    let parsed_type = type_record
        .parse()
        .map_err(|e| anyhow::anyhow!("Failed to parse IPI type record: {:?}", e))?;

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
        TypeData::UdtModSrcLine(data) => {
            // UdtModSrcLine is similar to UdtSrcLine but includes module index
            // We'll convert it to UdtSrcLineType (ignoring module index for now)
            let typ = (*data, type_stream, output_pdb).try_into()?;
            Type::UdtSrcLineType(typ)
        }
        TypeData::Unknown => {
            // Unknown types represent type kinds that the library doesn't recognize.
            // Try to get more information from the raw record if possible.
            warn!("Encountered Unknown type - this may indicate an unsupported type variant");
            return Err(Error::UnhandledType("Unknown type variant".to_string()));
        }
        _ => {
            warn!("Unhandled type variant: {:?}", typ);
            return Err(Error::UnhandledType(format!("{:?}", typ)));
        }
    };

    Ok(Rc::new(RefCell::new(result_typ)))
}
