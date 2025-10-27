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
/// A public symbol exported from a module.
///
/// Public symbols represent functions or data that are exported and can be
/// referenced by external modules.
pub struct PublicSymbol {
    /// The name of the symbol.
    pub name: String,

    /// True if this symbol represents executable code.
    pub is_code: bool,

    /// True if this symbol represents a function.
    pub is_function: bool,

    /// True if this symbol is managed code.
    pub is_managed: bool,

    /// True if this symbol is MSIL (Microsoft Intermediate Language).
    pub is_msil: bool,

    /// The section number this symbol is located in (1-based).
    /// None if the section is invalid (0).
    pub section: Option<u16>,

    /// The offset within the PE section, measured in bytes from the start of that section.
    ///
    /// This is NOT a file offset (which would be measured from the start of the PE file).
    /// This is NOT an RVA (which would be measured from the image base).
    ///
    /// # Example
    ///
    /// If a symbol is in section 2 at section_offset 0x100, and section 2 starts
    /// at RVA 0x3000, then the symbol's RVA would be 0x3100 (0x3000 + 0x100).
    ///
    /// This is the raw offset value from the PDB symbol record.
    pub section_offset: u32,

    /// The Relative Virtual Address (RVA) of this symbol.
    /// This is computed by adding the section's virtual_address to section_offset.
    /// None if the section is invalid or section headers couldn't be loaded.
    pub rva: Option<usize>,

    /// The absolute virtual address of this symbol.
    /// This is the RVA plus the base_address (if provided during parsing).
    /// None if RVA is None or no base_address was provided.
    pub address: Option<usize>,
}

impl
    From<(
        &ms_pdb::Pdb,
        uuid::Uuid,
        &ms_pdb::codeview::syms::Pub<'_>,
        Option<usize>,
        &ms_pdb::tpi::TypeStream<Vec<u8>>,
    )> for PublicSymbol
{
    fn from(
        data: (
            &ms_pdb::Pdb,
            uuid::Uuid,
            &ms_pdb::codeview::syms::Pub<'_>,
            Option<usize>,
            &ms_pdb::tpi::TypeStream<Vec<u8>>,
        ),
    ) -> Self {
        let (pdb, guid, sym, base_address, _type_stream) = data;

        let offset_segment = sym.offset_segment();
        let section_num = offset_segment.segment.get();
        let section_offset = offset_segment.offset.get();

        // Validate section number
        let section = if section_num == 0 {
            warn!(
                "symbol '{}' has an invalid section index (0) and RVA will be invalid",
                sym.name
            );
            None
        } else {
            Some(section_num)
        };

        // Convert section:offset to RVA using the section map
        let rva = crate::section_offset_to_rva(pdb, guid, section_num, section_offset);

        // Compute absolute address if base_address is provided
        let address = rva.and_then(|r| base_address.map(|base| r + base));

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
            section,
            section_offset,
            rva,
            address,
        }
    }
}

impl PublicSymbol {
    /// Returns true if this symbol has a valid location (valid section and RVA).
    ///
    /// # Examples
    ///
    /// ```
    /// # use ezpdb::PublicSymbol;
    /// // A symbol with a valid location
    /// let valid_symbol = PublicSymbol {
    ///     name: "MyFunction".to_string(),
    ///     is_code: true,
    ///     is_function: true,
    ///     is_managed: false,
    ///     is_msil: false,
    ///     section: Some(1),
    ///     section_offset: 0x1000,
    ///     rva: Some(0x2000),
    ///     address: Some(0x140002000),
    /// };
    /// assert!(valid_symbol.has_valid_location());
    ///
    /// // A symbol with an invalid section
    /// let invalid_symbol = PublicSymbol {
    ///     name: "InvalidSymbol".to_string(),
    ///     is_code: false,
    ///     is_function: false,
    ///     is_managed: false,
    ///     is_msil: false,
    ///     section: None,
    ///     section_offset: 0x1000,
    ///     rva: None,
    ///     address: None,
    /// };
    /// assert!(!invalid_symbol.has_valid_location());
    /// ```
    pub fn has_valid_location(&self) -> bool {
        self.section.is_some() && self.rva.is_some()
    }

    /// Returns the section and offset as a tuple, if available.
    ///
    /// This is useful for displaying or comparing section:offset pairs.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ezpdb::PublicSymbol;
    /// let symbol = PublicSymbol {
    ///     name: "MyFunction".to_string(),
    ///     is_code: true,
    ///     is_function: true,
    ///     is_managed: false,
    ///     is_msil: false,
    ///     section: Some(1),
    ///     section_offset: 0x1000,
    ///     rva: Some(0x2000),
    ///     address: None,
    /// };
    /// assert_eq!(symbol.section_and_offset(), Some((1, 0x1000)));
    /// ```
    pub fn section_and_offset(&self) -> Option<(u16, u32)> {
        self.section.map(|sec| (sec, self.section_offset))
    }

    /// Formats the symbol location as "section:offset" for display purposes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use ezpdb::PublicSymbol;
    /// let symbol = PublicSymbol {
    ///     name: "MyFunction".to_string(),
    ///     is_code: true,
    ///     is_function: true,
    ///     is_managed: false,
    ///     is_msil: false,
    ///     section: Some(1),
    ///     section_offset: 0x1000,
    ///     rva: Some(0x2000),
    ///     address: None,
    /// };
    /// assert_eq!(symbol.format_location(), "0001:00001000".to_string());
    ///
    /// let invalid = PublicSymbol {
    ///     name: "Invalid".to_string(),
    ///     is_code: false,
    ///     is_function: false,
    ///     is_managed: false,
    ///     is_msil: false,
    ///     section: None,
    ///     section_offset: 0x1000,
    ///     rva: None,
    ///     address: None,
    /// };
    /// assert_eq!(invalid.format_location(), "0000:00001000".to_string());
    /// ```
    pub fn format_location(&self) -> String {
        format!(
            "{:04x}:{:08x}",
            self.section.unwrap_or(0),
            self.section_offset
        )
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
/// A data symbol (global or local variable).
///
/// Data symbols represent variables that have storage in the program,
/// either as global/static variables or module-local variables.
pub struct Data {
    /// The name of the data symbol.
    pub name: String,

    /// True if this is a global data symbol (S_GDATA32/S_GMANDATA).
    /// False if this is a local/module data symbol (S_LDATA32/S_LMANDATA).
    pub is_global: bool,

    /// True if this is managed data (S_GMANDATA/S_LMANDATA).
    pub is_managed: bool,

    /// The type of this data symbol.
    pub ty: TypeRef,

    /// The section number this symbol is located in (1-based).
    /// None if the section is invalid (0).
    pub section: Option<u16>,

    /// The offset within the PE section, measured in bytes from the start of that section.
    ///
    /// This is NOT a file offset (which would be measured from the start of the PE file).
    /// This is NOT an RVA (which would be measured from the image base).
    ///
    /// # Example
    ///
    /// If a data symbol is in section 3 at section_offset 0x200, and section 3 starts
    /// at RVA 0x5000, then the symbol's RVA would be 0x5200 (0x5000 + 0x200).
    ///
    /// This is the raw offset value from the PDB symbol record.
    pub section_offset: u32,

    /// The Relative Virtual Address (RVA) of this symbol.
    /// This is computed by adding the section's virtual_address to section_offset.
    /// None if the section is invalid or section headers couldn't be loaded.
    pub rva: Option<usize>,

    /// The absolute virtual address of this symbol.
    /// This is the RVA plus the base_address (if provided during parsing).
    /// None if RVA is None or no base_address was provided.
    pub address: Option<usize>,
}

impl
    TryFrom<(
        &ms_pdb::Pdb,
        uuid::Uuid,
        &ms_pdb::codeview::syms::Data<'_>,
        Option<usize>,
        &ms_pdb::tpi::TypeStream<Vec<u8>>,
        &HashMap<TypeIndexNumber, TypeRef>,
        &HashMap<TypeIndexNumber, TypeRef>,
    )> for Data
{
    type Error = crate::error::Error;

    fn try_from(
        data: (
            &ms_pdb::Pdb,
            uuid::Uuid,
            &ms_pdb::codeview::syms::Data<'_>,
            Option<usize>,
            &ms_pdb::tpi::TypeStream<Vec<u8>>,
            &HashMap<TypeIndexNumber, TypeRef>,
            &HashMap<TypeIndexNumber, TypeRef>,
        ),
    ) -> Result<Self, Self::Error> {
        let (pdb, guid, sym, base_address, _type_stream, parsed_tpi_types, _parsed_ipi_types) =
            data;

        let offset_segment = sym.header.offset_segment;
        let type_index = sym.header.type_.get();
        let section_num = offset_segment.segment.get();
        let section_offset = offset_segment.offset.get();

        // Validate section number
        let section = if section_num == 0 {
            warn!(
                "data symbol '{}' has an invalid section index (0) and RVA will be invalid",
                sym.name
            );
            None
        } else {
            Some(section_num)
        };

        // Convert section:offset to RVA using the section map
        let rva = crate::section_offset_to_rva(pdb, guid, section_num, section_offset);

        // Compute absolute address if base_address is provided
        let address = rva.and_then(|r| base_address.map(|base| r + base));

        // Resolve type index: Data symbols should ONLY reference TPI types (data types)
        // IPI types are metadata (FuncId, BuildInfo, etc.) and should never be referenced by data symbols
        // According to LLVM PDB documentation, TPI contains actual types (int, struct, pointer, etc.)
        // while IPI contains metadata (function IDs, source line info, etc.)
        let ty = Rc::clone(
            parsed_tpi_types
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
            section,
            section_offset,
            rva,
            address,
        };

        Ok(data)
    }
}

impl Data {
    /// Returns true if this symbol has a valid location (valid section and RVA).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // A data symbol with a valid location
    /// assert!(data_symbol.has_valid_location());
    /// ```
    pub fn has_valid_location(&self) -> bool {
        self.section.is_some() && self.rva.is_some()
    }

    /// Returns the section and offset as a tuple, if available.
    pub fn section_and_offset(&self) -> Option<(u16, u32)> {
        self.section.map(|sec| (sec, self.section_offset))
    }

    /// Formats the symbol location as "section:offset" for display purposes.
    pub fn format_location(&self) -> String {
        format!(
            "{:04x}:{:08x}",
            self.section.unwrap_or(0),
            self.section_offset
        )
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
/// A procedure (function) symbol.
///
/// Procedures represent executable functions in the program with debug information
/// about their location, size, and prologue/epilogue boundaries.
pub struct Procedure {
    /// The name of the procedure/function.
    pub name: String,

    /// A string representation of the function signature (for debugging).
    pub signature: Option<String>,

    /// The type index for the procedure's type information.
    pub type_index: TypeIndexNumber,

    /// The section number this procedure is located in (1-based).
    /// None if the section is invalid (0).
    pub section: Option<u16>,

    /// The offset within the PE section where this procedure starts, measured in bytes
    /// from the start of that section.
    ///
    /// This is NOT a file offset (which would be measured from the start of the PE file).
    /// This is NOT an RVA (which would be measured from the image base).
    ///
    /// # Example
    ///
    /// If a procedure is in section 1 at section_offset 0x1000, and section 1 (.text)
    /// starts at RVA 0x1000, then the procedure's RVA would be 0x2000 (0x1000 + 0x1000).
    ///
    /// This is the raw offset value from the PDB symbol record.
    pub section_offset: u32,

    /// The Relative Virtual Address (RVA) of this procedure's entry point.
    /// This is computed by adding the section's virtual_address to section_offset.
    /// None if the section is invalid or section headers couldn't be loaded.
    pub rva: Option<usize>,

    /// The absolute virtual address of this procedure's entry point.
    /// This is the RVA plus the base_address (if provided during parsing).
    /// None if RVA is None or no base_address was provided.
    pub address: Option<usize>,

    /// The length of this procedure in bytes.
    pub len: usize,

    /// True if this is a global procedure (S_GPROC32/S_GPROC32_ID).
    /// False if this is a local/module procedure (S_LPROC32/S_LPROC32_ID).
    pub is_global: bool,

    /// True if this is a DPC (Deferred Procedure Call) procedure.
    pub is_dpc: bool,

    /// Offset in bytes from the procedure start to the end of the prologue.
    /// This is where the function's main body begins after setup code.
    pub prologue_end: usize,

    /// Offset in bytes from the procedure start to the start of the epilogue.
    /// This is where the function's cleanup code begins before returning.
    pub epilogue_start: usize,
}

impl
    From<(
        &ms_pdb::Pdb,
        uuid::Uuid,
        &ms_pdb::codeview::syms::Proc<'_>,
        Option<usize>,
        &ms_pdb::tpi::TypeStream<Vec<u8>>,
    )> for Procedure
{
    fn from(
        data: (
            &ms_pdb::Pdb,
            uuid::Uuid,
            &ms_pdb::codeview::syms::Proc<'_>,
            Option<usize>,
            &ms_pdb::tpi::TypeStream<Vec<u8>>,
        ),
    ) -> Self {
        let (pdb, guid, sym, base_address, type_stream) = data;

        let offset_segment = sym.fixed.offset_segment;
        let type_index = sym.fixed.proc_type.get();
        let section_num = offset_segment.segment.get();
        let section_offset = offset_segment.offset.get();

        // Validate section number
        let section = if section_num == 0 {
            warn!(
                "procedure '{}' has an invalid section index (0) and RVA will be invalid",
                sym.name
            );
            None
        } else {
            Some(section_num)
        };

        // Convert section:offset to RVA using the section map
        let rva = crate::section_offset_to_rva(pdb, guid, section_num, section_offset);

        // Compute absolute address if base_address is provided
        let address = rva.and_then(|r| base_address.map(|base| r + base));

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
            section,
            section_offset,
            rva,
            address,
            len: sym.fixed.proc_len.get() as usize,
            is_global: true, // Set by handle_symbol() based on SymKind
            is_dpc: false,   // Set by handle_symbol() based on SymKind
            prologue_end: sym.fixed.debug_start.get() as usize,
            epilogue_start: sym.fixed.debug_end.get() as usize,
        }
    }
}

impl Procedure {
    /// Returns true if this procedure has a valid location (valid section and RVA).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // A procedure with a valid location
    /// assert!(procedure.has_valid_location());
    /// ```
    pub fn has_valid_location(&self) -> bool {
        self.section.is_some() && self.rva.is_some()
    }

    /// Returns the section and offset as a tuple, if available.
    pub fn section_and_offset(&self) -> Option<(u16, u32)> {
        self.section.map(|sec| (sec, self.section_offset))
    }

    /// Formats the procedure location as "section:offset" for display purposes.
    pub fn format_location(&self) -> String {
        format!(
            "{:04x}:{:08x}",
            self.section.unwrap_or(0),
            self.section_offset
        )
    }

    /// Returns the end RVA of this procedure (start RVA + length).
    ///
    /// Returns None if the procedure has no valid RVA.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// if let Some(end_rva) = procedure.end_rva() {
    ///     println!("Procedure spans from {:x} to {:x}", procedure.rva.unwrap(), end_rva);
    /// }
    /// ```
    pub fn end_rva(&self) -> Option<usize> {
        self.rva.map(|rva| rva.saturating_add(self.len))
    }

    /// Returns the end address of this procedure (start address + length).
    ///
    /// Returns None if the procedure has no valid address.
    pub fn end_address(&self) -> Option<usize> {
        self.address.map(|addr| addr.saturating_add(self.len))
    }

    /// Returns true if the given RVA falls within this procedure's range.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// if procedure.contains_rva(0x2500) {
    ///     println!("RVA 0x2500 is inside this procedure");
    /// }
    /// ```
    pub fn contains_rva(&self, rva: usize) -> bool {
        if let Some(start_rva) = self.rva {
            let end_rva = start_rva.saturating_add(self.len);
            rva >= start_rva && rva < end_rva
        } else {
            false
        }
    }

    /// Returns true if the given address falls within this procedure's range.
    pub fn contains_address(&self, address: usize) -> bool {
        if let Some(start_addr) = self.address {
            let end_addr = start_addr.saturating_add(self.len);
            address >= start_addr && address < end_addr
        } else {
            false
        }
    }
}
