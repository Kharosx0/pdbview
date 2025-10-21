use crate::type_info::Type;
use log::warn;

#[cfg(feature = "serde")]
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::{From, TryFrom};
use std::path::PathBuf;
use std::rc::Rc;

pub type TypeRef = Rc<RefCell<Type>>;
pub type TypeIndexNumber = u32;
/// Represents a PDB that has been fully parsed
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct ParsedPdb {
    pub path: PathBuf,
    pub assembly_info: AssemblyInfo,
    pub public_symbols: Vec<PublicSymbol>,
    /// TPI stream types (structs, unions, enums, etc.)
    pub types: HashMap<TypeIndexNumber, TypeRef>,
    /// IPI stream types (FuncId, StringId, BuildInfo, etc.)
    pub ipi_types: HashMap<TypeIndexNumber, TypeRef>,
    pub procedures: Vec<Procedure>,
    pub global_data: Vec<Data>,
    pub debug_modules: Vec<DebugModule>,
    pub version: Version,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_uuid"))]
    pub guid: uuid::Uuid,
    pub age: u32,
    pub timestamp: u32,
    pub machine_type: Option<MachineType>,
}

impl ParsedPdb {
    /// Constructs a new [ParsedPdb] with the corresponding path
    pub fn new(path: PathBuf) -> Self {
        ParsedPdb {
            path,
            assembly_info: AssemblyInfo::default(),
            public_symbols: vec![],
            types: Default::default(),
            ipi_types: Default::default(),
            procedures: vec![],
            global_data: vec![],
            debug_modules: vec![],
            version: Version::Other(0),
            guid: uuid::Uuid::nil(),
            age: 0,
            timestamp: 0,
            machine_type: None,
        }
    }

    /// Find a type by name, automatically resolving forward references to complete definitions.
    ///
    /// If the type is found as a forward reference, this method searches for a complete definition
    /// with the same unique_name. This is necessary because Windows PDBs often contain forward
    /// references without fields.
    ///
    /// # Arguments
    /// * `name` - The type name to search for (e.g., "_KPROCESS")
    ///
    /// # Returns
    /// * `Some(TypeRef)` - The complete type definition if found
    /// * `None` - If no type with that name exists (forward ref or complete)
    pub fn find_type_by_name(&self, name: &str) -> Option<TypeRef> {
        use crate::type_info::Type;

        // First pass: find any type with this name
        let mut forward_ref = None;
        for type_ref in self.types.values() {
            if let Ok(borrowed) = type_ref.try_borrow() {
                match &*borrowed {
                    Type::Class(class) if class.name == name => {
                        if !class.properties.forward_reference {
                            // Found complete definition! Return immediately
                            return Some(Rc::clone(type_ref));
                        } else {
                            // Store forward ref and keep searching
                            forward_ref = Some((Rc::clone(type_ref), class.unique_name.clone()));
                        }
                    }
                    Type::Union(union) if union.name == name => {
                        if !union.properties.forward_reference {
                            return Some(Rc::clone(type_ref));
                        } else {
                            forward_ref = Some((Rc::clone(type_ref), union.unique_name.clone()));
                        }
                    }
                    Type::Enumeration(enum_type) if enum_type.name == name => {
                        return Some(Rc::clone(type_ref));
                    }
                    _ => {}
                }
            }
        }

        // If we only found a forward reference, try to find the complete definition by unique_name
        if let Some((fwd_ref, Some(unique_name))) = forward_ref {
            for type_ref in self.types.values() {
                if let Ok(borrowed) = type_ref.try_borrow() {
                    match &*borrowed {
                        Type::Class(class) => {
                            if !class.properties.forward_reference
                                && class.unique_name.as_ref() == Some(&unique_name)
                            {
                                warn!(
                                    "Resolved forward reference for {} to complete definition",
                                    name
                                );
                                return Some(Rc::clone(type_ref));
                            }
                        }
                        Type::Union(union) => {
                            if !union.properties.forward_reference
                                && union.unique_name.as_ref() == Some(&unique_name)
                            {
                                warn!(
                                    "Resolved forward reference for {} to complete definition",
                                    name
                                );
                                return Some(Rc::clone(type_ref));
                            }
                        }
                        _ => {}
                    }
                }
            }

            // No complete definition found, return the forward reference
            warn!(
                "Type {} only has forward reference, no complete definition found",
                name
            );
            return Some(fwd_ref);
        }

        None
    }
}

#[cfg(feature = "serde")]
fn serialize_uuid<S: serde::Serializer>(uuid: &uuid::Uuid, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(uuid.to_string().as_ref())
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum MachineType {
    /// The contents of this field are assumed to be applicable to any machine type.
    Unknown,
    /// Matsushita AM33
    Am33,
    /// x64
    Amd64,
    /// ARM little endian
    Arm,
    /// ARM64 little endian
    Arm64,
    /// ARM Thumb-2 little endian
    ArmNT,
    /// EFI byte code
    Ebc,
    /// Intel 386 or later processors and compatible processors
    X86,
    /// Intel Itanium processor family
    Ia64,
    /// Mitsubishi M32R little endian
    M32R,
    /// MIPS16
    Mips16,
    /// MIPS with FPU
    MipsFpu,
    /// MIPS16 with FPU
    MipsFpu16,
    /// Power PC little endian
    PowerPC,
    /// Power PC with floating point support
    PowerPCFP,
    /// MIPS little endian
    R4000,
    /// RISC-V 32-bit address space
    RiscV32,
    /// RISC-V 64-bit address space
    RiscV64,
    /// RISC-V 128-bit address space
    RiscV128,
    /// Hitachi SH3
    SH3,
    /// Hitachi SH3 DSP
    SH3DSP,
    /// Hitachi SH4
    SH4,
    /// Hitachi SH5
    SH5,
    /// Thumb
    Thumb,
    /// MIPS little-endian WCE v2
    WceMipsV2,
    /// Invalid value
    Invalid,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum Version {
    V41,
    V50,
    V60,
    V70,
    V110,
    Other(u32),
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct AssemblyInfo {
    pub build_info: Option<BuildInfo>,
    pub compiler_info: Option<CompilerInfo>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct BuildInfo {
    pub arguments: Vec<String>,
}

impl
    TryFrom<(
        &ms_pdb::codeview::syms::BuildInfo,
        &ms_pdb::tpi::TypeStream<Vec<u8>>,
    )> for BuildInfo
{
    type Error = crate::error::Error;

    fn try_from(
        info: (
            &ms_pdb::codeview::syms::BuildInfo,
            &ms_pdb::tpi::TypeStream<Vec<u8>>,
        ),
    ) -> Result<Self, Self::Error> {
        let (symbol, ipi_stream) = info;

        // BuildInfo.item is an ItemId pointing to an LF_BUILDINFO record in IPI stream
        let type_index = ms_pdb::codeview::types::TypeIndex(symbol.item);
        let mut arguments = Vec::new();

        if let Ok(type_record) = ipi_stream.record(type_index) {
            // Parse the LF_BUILDINFO record
            match type_record.parse() {
                Ok(ms_pdb::codeview::types::TypeData::BuildInfo(build_info_data)) => {
                    // BuildInfo contains TypeIndex references to StringId records
                    // Each StringId contains the actual string (current dir, compiler, source, pdb, cmdline)
                    for arg_idx in build_info_data.args.iter() {
                        let arg_type_index = ms_pdb::codeview::types::TypeIndex(arg_idx.get());
                        if arg_type_index.0 == 0 {
                            continue;
                        }

                        if let Ok(arg_record) = ipi_stream.record(arg_type_index) {
                            if let Ok(ms_pdb::codeview::types::TypeData::StringId(string_id)) =
                                arg_record.parse()
                            {
                                arguments.push(string_id.name.to_string());
                            }
                        }
                    }
                }
                Ok(other) => {
                    // Unexpected type - store debug representation
                    arguments.push(format!("{:?}", other));
                }
                Err(e) => {
                    warn!("Failed to parse BuildInfo type record: {}", e);
                }
            }
        }

        Ok(BuildInfo { arguments })
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct CompilerInfo {
    pub language: String,
    pub flags: CompileFlags,
    pub cpu_type: String,
    pub frontend_version: CompilerVersion,
    pub backend_version: CompilerVersion,
    pub version_string: String,
}

// Note: CompileFlags symbol (S_COMPILE2, S_COMPILE3) is not currently exposed in ms-pdb
// When ms-pdb adds support for parsing compiler flags symbols, this conversion can be implemented:
// impl From<&ms_pdb::codeview::syms::CompileFlags> for CompilerInfo {
//     fn from(flags: &ms_pdb::codeview::syms::CompileFlags) -> Self {
//         CompilerInfo {
//             language: String::from_utf8_lossy(flags.language).into_owned(),
//             flags: CompileFlags {
//                 edit_and_continue: flags.flags.edit_and_continue,
//                 no_debug_info: flags.flags.no_debug_info,
//                 link_time_codegen: flags.flags.link_time_codegen,
//                 no_data_align: flags.flags.no_data_align,
//                 managed: flags.flags.managed,
//                 security_checks: flags.flags.security_checks,
//                 hot_patch: flags.flags.hot_patch,
//                 cvtcil: flags.flags.cvtcil,
//                 msil_module: flags.flags.msil_module,
//                 sdl: flags.flags.sdl,
//                 pgo: flags.flags.pgo,
//                 exp_module: flags.flags.exp_module,
//             },
//             cpu_type: String::from_utf8_lossy(flags.cpu_type).into_owned(),
//             frontend_version: CompilerVersion {
//                 major: flags.frontend_version.major.get(),
//                 minor: flags.frontend_version.minor.get(),
//                 build: flags.frontend_version.build.get(),
//                 qfe: flags.frontend_version.qfe.map(|q| q.get()),
//             },
//             backend_version: CompilerVersion {
//                 major: flags.backend_version.major.get(),
//                 minor: flags.backend_version.minor.get(),
//                 build: flags.backend_version.build.get(),
//                 qfe: flags.backend_version.qfe.map(|q| q.get()),
//             },
//             version_string: String::from_utf8_lossy(flags.version_string).into_owned(),
//         }
//     }
// }

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct CompileFlags {
    /// Compiled for edit and continue.
    pub edit_and_continue: bool,
    /// Compiled without debugging info.
    pub no_debug_info: bool,
    /// Compiled with `LTCG`.
    pub link_time_codegen: bool,
    /// Compiled with `/bzalign`.
    pub no_data_align: bool,
    /// Managed code or data is present.
    pub managed: bool,
    /// Compiled with `/GS`.
    pub security_checks: bool,
    /// Compiled with `/hotpatch`.
    pub hot_patch: bool,
    /// Compiled with `CvtCIL`.
    pub cvtcil: bool,
    /// This is a MSIL .NET Module.
    pub msil_module: bool,
    /// Compiled with `/sdl`.
    pub sdl: bool,
    /// Compiled with `/ltcg:pgo` or `pgo:`.
    pub pgo: bool,
    /// This is a .exp module.
    pub exp_module: bool,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct CompilerVersion {
    pub major: u16,
    pub minor: u16,
    pub build: u16,
    pub qfe: Option<u16>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct DebugModule {
    pub name: String,
    pub object_file_name: String,
    pub source_files: Option<Vec<FileInfo>>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum Checksum {
    None,
    Md5(Vec<u8>),
    Sha1(Vec<u8>),
    Sha256(Vec<u8>),
}

impl From<&ms_pdb::lines::FileChecksum<'_>> for Checksum {
    fn from(checksum: &ms_pdb::lines::FileChecksum<'_>) -> Self {
        use ms_pdb::lines::ChecksumKind;
        match checksum.header.checksum_kind {
            ChecksumKind::NONE => Checksum::None,
            ChecksumKind::MD5 => Checksum::Md5(checksum.checksum_data.to_vec()),
            ChecksumKind::SHA_1 => Checksum::Sha1(checksum.checksum_data.to_vec()),
            ChecksumKind::SHA_256 => Checksum::Sha256(checksum.checksum_data.to_vec()),
            _ => Checksum::None, // Unknown checksum types default to None
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct FileInfo {
    pub name: String,
    pub checksum: Checksum,
}

impl From<&ms_pdb::dbi::ModuleInfo<'_>> for DebugModule {
    fn from(module: &ms_pdb::dbi::ModuleInfo<'_>) -> Self {
        // Note: Source file extraction from ms-pdb requires accessing the module stream
        // and parsing line data subsections, which is more complex than the old pdb crate.
        // For now, we'll leave source_files as None and handle it separately if needed.
        DebugModule {
            name: module.module_name().to_string(),
            object_file_name: module.obj_file().to_string(),
            source_files: None,
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct PublicSymbol {
    pub name: String,
    pub is_code: bool,
    pub is_function: bool,
    pub is_managed: bool,
    pub is_msil: bool,
    pub offset: Option<usize>,
}

impl
    From<(
        &ms_pdb::Pdb,
        &ms_pdb::codeview::syms::Pub<'_>,
        Option<usize>,
        &ms_pdb::tpi::TypeStream<Vec<u8>>,
    )> for PublicSymbol
{
    fn from(
        data: (
            &ms_pdb::Pdb,
            &ms_pdb::codeview::syms::Pub<'_>,
            Option<usize>,
            &ms_pdb::tpi::TypeStream<Vec<u8>>,
        ),
    ) -> Self {
        let (pdb, sym, _base_address, _type_stream) = data;

        let offset_segment = sym.offset_segment();

        if offset_segment.segment.get() == 0 {
            warn!(
                "symbol type has an invalid section index and RVA will be invalid: {:?}",
                sym
            )
        }

        // Convert section:offset to RVA using the section map
        let offset = crate::section_offset_to_rva(
            pdb,
            offset_segment.segment.get(),
            offset_segment.offset.get(),
        );

        // Extract flags from the PubFixed structure
        let flags = sym.fixed.flags.get();
        let is_code = (flags & 0x00000001) != 0; // CV_PUBSYMFLAGS_Code
        let is_function = (flags & 0x00000002) != 0; // CV_PUBSYMFLAGS_Function
        let is_managed = (flags & 0x00000004) != 0; // CV_PUBSYMFLAGS_Managed
        let is_msil = (flags & 0x00000008) != 0; // CV_PUBSYMFLAGS_MSIL

        PublicSymbol {
            name: sym.name.to_string(),
            is_code,
            is_function,
            is_managed,
            is_msil,
            offset,
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Data {
    pub name: String,

    pub is_global: bool,

    pub is_managed: bool,

    pub ty: TypeRef,

    pub offset: Option<usize>,
}

impl
    TryFrom<(
        &ms_pdb::Pdb,
        &ms_pdb::codeview::syms::Data<'_>,
        Option<usize>,
        &ms_pdb::tpi::TypeStream<Vec<u8>>,
        &HashMap<TypeIndexNumber, TypeRef>,
    )> for Data
{
    type Error = crate::error::Error;

    fn try_from(
        data: (
            &ms_pdb::Pdb,
            &ms_pdb::codeview::syms::Data<'_>,
            Option<usize>,
            &ms_pdb::tpi::TypeStream<Vec<u8>>,
            &HashMap<TypeIndexNumber, TypeRef>,
        ),
    ) -> Result<Self, Self::Error> {
        let (pdb, sym, _base_address, _type_stream, parsed_types) = data;

        let offset_segment = sym.header.offset_segment;
        let type_index = sym.header.type_.get();

        // Convert section:offset to RVA using the section map
        let offset = crate::section_offset_to_rva(
            pdb,
            offset_segment.segment.get(),
            offset_segment.offset.get(),
        );

        let ty = Rc::clone(
            parsed_types
                .get(&type_index.0)
                .ok_or(Self::Error::UnresolvedType(type_index.0))?,
        );

        // Note: is_global and is_managed are set by handle_symbol() in lib.rs
        // based on SymKind (S_GDATA32, S_GMANDATA, S_LDATA32, S_LMANDATA)

        let data = Data {
            name: sym.name.to_string(),
            is_global: true,   // Set by handle_symbol() based on SymKind
            is_managed: false, // Set by handle_symbol() based on SymKind
            ty,
            offset,
        };

        Ok(data)
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Procedure {
    pub name: String,

    pub signature: Option<String>,
    pub type_index: TypeIndexNumber,

    /// This reflects the RVA in the transformed address space. See [PdbInternalSectionOffset docs](https://docs.rs/pdb/latest/pdb/struct.PdbInternalSectionOffset.html)
    /// for more details.
    pub address: Option<usize>,
    pub len: usize,

    pub is_global: bool,
    pub is_dpc: bool,
    /// length of this procedure in BYTES
    pub prologue_end: usize,
    pub epilogue_start: usize,
}

impl
    From<(
        &ms_pdb::Pdb,
        &ms_pdb::codeview::syms::Proc<'_>,
        Option<usize>,
        &ms_pdb::tpi::TypeStream<Vec<u8>>,
    )> for Procedure
{
    fn from(
        data: (
            &ms_pdb::Pdb,
            &ms_pdb::codeview::syms::Proc<'_>,
            Option<usize>,
            &ms_pdb::tpi::TypeStream<Vec<u8>>,
        ),
    ) -> Self {
        let (pdb, sym, _base_address, type_stream) = data;

        let offset_segment = sym.fixed.offset_segment;
        let type_index = sym.fixed.proc_type.get();

        if offset_segment.segment.get() == 0 {
            warn!(
                "symbol type has an invalid section index and RVA will be invalid: {:?}",
                sym
            )
        }

        // Convert section:offset to RVA using the section map
        let address = crate::section_offset_to_rva(
            pdb,
            offset_segment.segment.get(),
            offset_segment.offset.get(),
        );

        // Try to get the signature from the type stream
        let signature = type_stream.record(type_index).ok().map(|type_info| {
            format!(
                "{:?}",
                type_info
                    .parse()
                    .unwrap_or(ms_pdb::codeview::types::TypeData::Unknown)
            )
        });

        Procedure {
            name: sym.name.to_string(),
            signature,
            type_index: type_index.0,
            address,
            len: sym.fixed.proc_len.get() as usize,
            is_global: true, // Set by handle_symbol() based on SymKind
            is_dpc: false,   // Set by handle_symbol() based on SymKind
            prologue_end: sym.fixed.debug_start.get() as usize,
            epilogue_start: sym.fixed.debug_end.get() as usize,
        }
    }
}
