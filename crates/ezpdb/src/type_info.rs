use crate::error::Error;
use crate::symbol_types::ParsedPdb;
use crate::symbol_types::TypeRef;
#[cfg(feature = "serde")]
use serde::Serialize;
use std::cell::RefCell;
use std::convert::{TryFrom, TryInto};
use std::rc::Rc;

use log::warn;

pub trait Typed {
    /// Returns the size (in bytes) of this type
    fn type_size(&self, pdb: &ParsedPdb) -> usize;

    /// Called after all types have been parsed
    fn on_complete(&mut self, _pdb: &ParsedPdb) {}
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum Type {
    Class(Class),
    VirtualBaseClass(VirtualBaseClass),
    Union(Union),
    Bitfield(Bitfield),
    Enumeration(Enumeration),
    EnumVariant(EnumVariant),
    Pointer(Pointer),
    Primitive(Primitive),
    Array(Array),
    FieldList(FieldList),
    ArgumentList(ArgumentList),
    Modifier(Modifier),
    Member(Member),
    Procedure(Procedure),
    MemberFunction(MemberFunction),
    MethodList(MethodList),
    MethodListEntry(MethodListEntry),
    Nested(Nested),
    OverloadedMethod(OverloadedMethod),
    Method(Method),
    StaticMember(StaticMember),
    BaseClass(BaseClass),
    VTable(VTable),
    Alias(Alias),
    VTableShape(VTableShape),
    VFTableType(VFTableType),
    // IPI stream types
    FuncId(FuncIdType),
    MFuncId(MFuncIdType),
    StringId(StringIdType),
    SubStrList(SubStrListType),
    BuildInfoType(BuildInfoTypeData),
    UdtSrcLineType(UdtSrcLineType),
}

impl Typed for Type {
    fn type_size(&self, pdb: &ParsedPdb) -> usize {
        match self {
            Type::Class(class) => class.type_size(pdb),
            Type::Union(union) => union.type_size(pdb),
            Type::Bitfield(bitfield) => bitfield.underlying_type.borrow().type_size(pdb),
            Type::Enumeration(e) => e.underlying_type.borrow().type_size(pdb),
            Type::Pointer(p) => p.attributes.kind.type_size(pdb),
            Type::Primitive(p) => p.type_size(pdb),
            Type::Array(a) => a.type_size(pdb),
            Type::FieldList(fields) => fields
                .0
                .iter()
                .fold(0, |acc, field| acc + field.borrow().type_size(pdb)),
            Type::EnumVariant(_) => panic!("type_size() invoked for EnumVariant"),
            Type::Modifier(modifier) => modifier.underlying_type.borrow().type_size(pdb),
            Type::Member(_) => panic!("type_size() invoked for Member"),
            Type::ArgumentList(_) => panic!("type_size() invoked for ArgumentList"),
            Type::Procedure(_) => panic!("type_size() invoked for Procedure"),
            Type::MemberFunction(_) => panic!("type_size() invoked for MemberFunction"),
            Type::MethodList(_) => panic!("type_size() invoked for MethodList"),
            Type::MethodListEntry(_) => panic!("type_size() invoked for MethodListEntry"),
            Type::VirtualBaseClass(_) => panic!("type_size() invoked for VirtualBaseClass"),
            Type::Nested(_) => panic!("type_size() invoked for Nested"),
            Type::OverloadedMethod(_) => panic!("type_size() invoked for overloaded method"),
            Type::Method(_) => panic!("type_size() invoked for overloaded method"),
            Type::StaticMember(_) => panic!("type_size() invoked for StaticMember"),
            Type::VTable(_) => panic!("type_size() invoked for VTable"),
            Type::BaseClass(_) => panic!("type_size() invoked for BaseClass"),
            Type::Alias(alias) => alias.underlying_type.borrow().type_size(pdb),
            Type::VTableShape(_) => panic!("type_size() invoked for VTableShape"),
            Type::VFTableType(_) => panic!("type_size() invoked for VFTableType"),
            // IPI types don't have a meaningful size - they're metadata, not data types
            // Return 0 instead of panicking to handle cases where data symbols accidentally
            // reference IPI types (shouldn't happen, but ms-pdb might expose them in TPI stream)
            Type::FuncId(_) => {
                warn!("type_size() invoked for FuncId - IPI types don't have size, returning 0");
                0
            }
            Type::MFuncId(_) => {
                warn!("type_size() invoked for MFuncId - IPI types don't have size, returning 0");
                0
            }
            Type::StringId(_) => {
                warn!("type_size() invoked for StringId - IPI types don't have size, returning 0");
                0
            }
            Type::SubStrList(_) => {
                warn!(
                    "type_size() invoked for SubStrList - IPI types don't have size, returning 0"
                );
                0
            }
            Type::BuildInfoType(_) => {
                warn!("type_size() invoked for BuildInfoType - IPI types don't have size, returning 0");
                0
            }
            Type::UdtSrcLineType(_) => {
                warn!("type_size() invoked for UdtSrcLineType - IPI types don't have size, returning 0");
                0
            }
        }
    }

    fn on_complete(&mut self, pdb: &ParsedPdb) {
        match self {
            Type::Class(class) => class.on_complete(pdb),
            Type::Union(union) => union.on_complete(pdb),
            Type::Array(a) => a.on_complete(pdb),
            _ => {}
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct TypeProperties {
    pub packed: bool,
    pub constructors: bool,
    pub overlapped_operators: bool,
    pub is_nested_type: bool,
    pub contains_nested_types: bool,
    pub overload_assignment: bool,
    pub overload_coasting: bool,
    pub forward_reference: bool,
    pub scoped_definition: bool,
    pub has_unique_name: bool,
    pub sealed: bool,
    pub hfa: u8,
    pub intristic_type: bool,
    pub mocom: u8,
}

impl TryFrom<ms_pdb::codeview::types::UdtProperties> for TypeProperties {
    type Error = Error;
    fn try_from(props: ms_pdb::codeview::types::UdtProperties) -> Result<Self, Self::Error> {
        Ok(TypeProperties {
            packed: props.packed(),
            constructors: props.ctor(),
            overlapped_operators: props.ovlops(),
            is_nested_type: props.isnested(),
            contains_nested_types: props.cnested(),
            overload_assignment: props.opassign(),
            overload_coasting: props.opcast(),
            forward_reference: props.fwdref(),
            scoped_definition: props.scoped(),
            has_unique_name: props.hasuniquename(),
            sealed: props.sealed(),
            hfa: props.hfa() as u8,
            intristic_type: props.intrinsic(),
            mocom: props.mocom() as u8,
        })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Class {
    pub name: String,
    pub unique_name: Option<String>,
    pub kind: ClassKind,
    pub properties: TypeProperties,
    pub derived_from: Option<TypeRef>,
    pub fields: Vec<TypeRef>,
    pub size: usize,
}

impl Typed for Class {
    fn type_size(&self, pdb: &ParsedPdb) -> usize {
        if self.properties.forward_reference {
            // Find the implementation
            for value in pdb.types.values() {
                if let Ok(borrow) = value.as_ref().try_borrow() {
                    if let Type::Class(class) = &*borrow {
                        if !class.properties.forward_reference
                            && class.unique_name == self.unique_name
                        {
                            return class.type_size(pdb);
                        }
                    }
                }
            }

            warn!("could not get forward reference for {}", self.name);
        }

        self.size
    }
}

type FromClass<'a, 'b> = (
    &'b ms_pdb::codeview::types::Struct<'a>,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromClass<'_, '_>> for Class {
    type Error = Error;
    fn try_from(info: FromClass<'_, '_>) -> Result<Self, Self::Error> {
        let (class, type_stream, output_pdb) = info;

        let _count = class.fixed.num_elements.get();
        let properties = class.fixed.property.get();
        let fields = class.fixed.field_list.get();
        let derived_from = class.fixed.derivation_list.get();
        let _vtable_shape = class.fixed.vtable_shape.get();
        let size = u64::try_from(class.length).unwrap_or(0);

        let derived_from = if derived_from.0 != 0 {
            Some(crate::handle_type(derived_from, output_pdb, type_stream)?)
        } else {
            None
        };

        let fields: Vec<TypeRef> = if fields.0 != 0 {
            // Parse the field list directly using TypeIndex
            // This allows iter_fields() to properly handle continuation chains
            let field_list: FieldList = (fields, type_stream, output_pdb).try_into()?;
            field_list.0
        } else {
            vec![]
        };

        let unique_name = class.unique_name.map(|s| s.to_string());

        // Determine ClassKind based on available information
        // In ms-pdb, the Struct type represents LF_STRUCTURE, LF_CLASS, and LF_INTERFACE
        // Since we can't directly access the leaf type, we use heuristics:
        // - Interfaces typically have all methods (no data members) and are abstract
        // - Classes typically have constructors/destructors
        // - Structs are simpler
        let kind = if class.name.to_string().starts_with("I") && properties.cnested() {
            // Heuristic: names starting with 'I' and having nested types might be interfaces
            ClassKind::Interface
        } else if properties.ctor() || properties.ovlops() || properties.opassign() {
            // Has constructors or operator overloads - likely a class
            ClassKind::Class
        } else {
            // Default to struct
            ClassKind::Struct
        };

        Ok(Class {
            name: class.name.to_string(),
            unique_name,
            kind,
            properties: properties.try_into()?,
            derived_from,
            fields,
            size: size as usize,
        })
    }
}
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct BaseClass {
    pub kind: ClassKind,
    pub base_class: TypeRef,
    pub offset: usize,
}

type FromBaseClass<'a, 'b> = (
    &'b ms_pdb::codeview::types::fields::BaseClass<'a>,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromBaseClass<'_, '_>> for BaseClass {
    type Error = Error;
    fn try_from(info: FromBaseClass<'_, '_>) -> Result<Self, Self::Error> {
        let (class, type_stream, output_pdb) = info;

        let base_class = crate::handle_type(class.ty, output_pdb, type_stream)?;

        // Determine ClassKind from the base class type itself
        let kind = {
            let borrowed = base_class.as_ref().borrow();
            match &*borrowed {
                Type::Class(c) => c.kind,
                _ => ClassKind::Struct, // Default if not a class type
            }
        };

        Ok(BaseClass {
            kind,
            base_class,
            offset: u64::try_from(class.offset).unwrap_or(0) as usize,
        })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct VirtualBaseClass {
    pub direct: bool,
    pub base_class: TypeRef,
    pub base_pointer: TypeRef,
    pub base_pointer_offset: usize,
    pub virtual_base_offset: usize,
}

type FromVirtualBaseClass<'a, 'b> = (
    &'b ms_pdb::codeview::types::fields::DirectVirtualBaseClass<'a>,
    bool, // true = direct, false = indirect
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromVirtualBaseClass<'_, '_>> for VirtualBaseClass {
    type Error = Error;
    fn try_from(info: FromVirtualBaseClass<'_, '_>) -> Result<Self, Self::Error> {
        let (vbc, is_direct, type_stream, output_pdb) = info;

        let base_class = crate::handle_type(vbc.fixed.btype.get(), output_pdb, type_stream)?;
        let base_pointer = crate::handle_type(vbc.fixed.vbtype.get(), output_pdb, type_stream)?;

        Ok(VirtualBaseClass {
            direct: is_direct,
            base_class,
            base_pointer,
            base_pointer_offset: u64::try_from(vbc.vbpoff).unwrap_or(0) as usize,
            virtual_base_offset: u64::try_from(vbc.vboff).unwrap_or(0) as usize,
        })
    }
}

#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum ClassKind {
    Class,
    Struct,
    Interface,
}

// Note: In ms-pdb, Class/Struct/Interface distinction is maintained via the Leaf type
// (LF_CLASS, LF_STRUCTURE, LF_INTERFACE), not via a ClassKind enum.
// The ClassKind is only used internally in ezpdb for display purposes.
// We'll infer it from context or default to Struct.
impl TryFrom<ms_pdb::codeview::types::Leaf> for ClassKind {
    type Error = Error;
    fn try_from(leaf: ms_pdb::codeview::types::Leaf) -> Result<Self, Self::Error> {
        Ok(match leaf {
            ms_pdb::codeview::types::Leaf::LF_CLASS => ClassKind::Class,
            ms_pdb::codeview::types::Leaf::LF_STRUCTURE => ClassKind::Struct,
            ms_pdb::codeview::types::Leaf::LF_INTERFACE => ClassKind::Interface,
            _ => ClassKind::Struct, // Default for other cases
        })
    }
}

impl std::fmt::Display for ClassKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClassKind::Class => write!(f, "Class"),
            ClassKind::Struct => write!(f, "Struct"),
            ClassKind::Interface => write!(f, "Interface"),
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Union {
    pub name: String,
    pub unique_name: Option<String>,
    pub properties: TypeProperties,
    pub size: usize,
    pub count: usize,
    pub fields: Vec<TypeRef>,
}

impl Typed for Union {
    fn type_size(&self, pdb: &ParsedPdb) -> usize {
        if self.properties.forward_reference {
            // Find the implementation
            for value in pdb.types.values() {
                if let Ok(value) = value.as_ref().try_borrow() {
                    if let Type::Union(union) = &*value {
                        if !union.properties.forward_reference
                            && union.unique_name == self.unique_name
                        {
                            return union.type_size(pdb);
                        }
                    }
                }
            }

            warn!("could not get forward reference for {}", self.name);
        }

        self.size
    }
}
type FromUnion<'a, 'b> = (
    &'b ms_pdb::codeview::types::Union<'a>,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);
impl TryFrom<FromUnion<'_, '_>> for Union {
    type Error = Error;
    fn try_from(data: FromUnion<'_, '_>) -> Result<Self, Self::Error> {
        let (union, type_stream, output_pdb) = data;

        let count = union.fixed.count.get();
        let properties = union.fixed.property.get();
        let fields = union.fixed.fields.get();

        let fields = if fields.0 != 0 {
            // Parse the field list directly using TypeIndex
            // This allows iter_fields() to properly handle continuation chains
            let field_list: FieldList = (fields, type_stream, output_pdb).try_into()?;
            field_list.0
        } else {
            vec![]
        };

        let union_result = Union {
            name: union.name.to_string(),
            unique_name: union.unique_name.map(|s| s.to_string()),
            properties: properties.try_into()?,
            size: u64::try_from(union.length).unwrap_or(0) as usize,
            count: count as usize,
            fields,
        };

        Ok(union_result)
    }
}

type FromBitfield<'a, 'b> = (
    &'a ms_pdb::codeview::types::Bitfield,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Bitfield {
    pub underlying_type: TypeRef,
    pub len: usize,
    pub position: usize,
}
impl TryFrom<FromBitfield<'_, '_>> for Bitfield {
    type Error = Error;
    fn try_from(data: FromBitfield<'_, '_>) -> Result<Self, Self::Error> {
        let (bitfield_data, type_stream, output_pdb) = data;

        let underlying_type =
            crate::handle_type(bitfield_data.underlying_type.get(), output_pdb, type_stream)?;

        Ok(Bitfield {
            underlying_type,
            len: bitfield_data.length as usize,
            position: bitfield_data.position as usize,
        })
    }
}

impl Typed for Bitfield {
    fn type_size(&self, _pdb: &ParsedPdb) -> usize {
        panic!("calling type_size() directly on a bitfield is probably not what you want");
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Enumeration {
    pub name: String,
    pub unique_name: Option<String>,
    pub underlying_type: TypeRef,
    pub variants: Vec<EnumVariant>,
    pub properties: TypeProperties,
}

type FromEnumeration<'a, 'b> = (
    &'b ms_pdb::codeview::types::Enum<'a>,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromEnumeration<'_, '_>> for Enumeration {
    type Error = Error;
    fn try_from(data: FromEnumeration<'_, '_>) -> Result<Self, Self::Error> {
        let (e, type_stream, output_pdb) = data;

        let _count = e.fixed.count.get();
        let properties = e.fixed.property.get();
        let underlying_type = e.fixed.underlying_type.get();
        let fields = e.fixed.fields.get();

        let underlying_type = crate::handle_type(underlying_type, output_pdb, type_stream)?;

        // Parse the field list directly using TypeIndex, just like Class and Union do
        // This allows iter_fields() to properly handle continuation chains
        let fields: Vec<EnumVariant> = if fields.0 != 0 {
            let field_list: FieldList = (fields, type_stream, output_pdb).try_into()?;
            field_list
                .0
                .iter()
                .filter_map(|field| {
                    if let Type::EnumVariant(var) = &*field.borrow() {
                        Some(var.clone())
                    } else {
                        warn!(
                            "Unexpected non-EnumVariant field in enum field list: {:?}",
                            field
                        );
                        None
                    }
                })
                .collect()
        } else {
            vec![]
        };

        Ok(Enumeration {
            name: e.name.to_string(),
            unique_name: e.unique_name.map(|s| s.to_string()),
            underlying_type,
            variants: fields,
            properties: properties.try_into()?,
        })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct EnumVariant {
    pub name: String,
    pub value: VariantValue,
}

type FromEnumerate<'a, 'b> = &'b ms_pdb::codeview::types::fields::Enumerate<'a>;

impl TryFrom<FromEnumerate<'_, '_>> for EnumVariant {
    type Error = Error;
    fn try_from(data: FromEnumerate<'_, '_>) -> Result<Self, Self::Error> {
        let e = data;

        Ok(Self {
            name: e.name.to_string(),
            value: e.value.try_into()?,
        })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum VariantValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
}

type FromVariant<'a> = ms_pdb::codeview::parser::Number<'a>;

impl TryFrom<FromVariant<'_>> for VariantValue {
    type Error = Error;
    fn try_from(number: FromVariant<'_>) -> Result<Self, Self::Error> {
        // Try different types in order of likelihood
        if let Ok(val) = u64::try_from(number) {
            if val <= u8::MAX as u64 {
                return Ok(VariantValue::U8(val as u8));
            } else if val <= u16::MAX as u64 {
                return Ok(VariantValue::U16(val as u16));
            } else if val <= u32::MAX as u64 {
                return Ok(VariantValue::U32(val as u32));
            } else {
                return Ok(VariantValue::U64(val));
            }
        }

        if let Ok(val) = i64::try_from(number) {
            if val >= i8::MIN as i64 && val <= i8::MAX as i64 {
                return Ok(VariantValue::I8(val as i8));
            } else if val >= i16::MIN as i64 && val <= i16::MAX as i64 {
                return Ok(VariantValue::I16(val as i16));
            } else if val >= i32::MIN as i64 && val <= i32::MAX as i64 {
                return Ok(VariantValue::I32(val as i32));
            } else {
                return Ok(VariantValue::I64(val));
            }
        }

        Err(anyhow::anyhow!("Could not convert Number to VariantValue"))?
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Pointer {
    pub underlying_type: Option<TypeRef>,
    pub attributes: PointerAttributes,
}

type FromPointer<'a, 'b> = (
    &'b ms_pdb::codeview::types::Pointer<'a>,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);
impl TryFrom<FromPointer<'_, '_>> for Pointer {
    type Error = Error;
    fn try_from(data: FromPointer<'_, '_>) -> Result<Self, Self::Error> {
        let (pointer, type_stream, output_pdb) = data;

        let underlying_type =
            crate::handle_type(pointer.fixed.ty.get(), output_pdb, type_stream).ok();
        let attr = pointer.fixed.attr();

        // Extract pointer kind to determine size
        let pointer_kind_value = attr.pointer_kind();

        // Calculate size based on pointer kind
        let size = match pointer_kind_value {
            0 => 2,  // Near16
            1 => 4,  // Far16 (segment:offset = 2+2)
            2 => 4,  // Huge16 (segment:offset = 2+2)
            3 => 4,  // BaseSeg
            4 => 4,  // BaseVal
            5 => 4,  // BaseSegVal
            6 => 4,  // BaseAddr
            7 => 4,  // BaseSegAddr
            8 => 4,  // BaseType
            9 => 4,  // BaseSelf
            10 => 4, // Near32
            11 => 6, // Far32 (selector:offset = 2+4)
            12 => 8, // Ptr64
            _ => {
                warn!(
                    "Unknown pointer kind: {}, defaulting to 8 bytes",
                    pointer_kind_value
                );
                8
            }
        };

        let pointer_kind = match pointer_kind_value {
            0 => PointerKind::Near16,
            1 => PointerKind::Far16,
            2 => PointerKind::Huge16,
            3 => PointerKind::BaseSeg,
            4 => PointerKind::BaseVal,
            5 => PointerKind::BaseSegVal,
            6 => PointerKind::BaseAddr,
            7 => PointerKind::BaseSegAddr,
            8 => PointerKind::BaseType,
            9 => PointerKind::BaseSelf,
            10 => PointerKind::Near32,
            11 => PointerKind::Far32,
            12 => PointerKind::Ptr64,
            _ => {
                warn!(
                    "Unknown pointer kind: {}, defaulting to Ptr64",
                    pointer_kind_value
                );
                PointerKind::Ptr64
            }
        };

        Ok(Pointer {
            underlying_type,
            attributes: PointerAttributes {
                kind: pointer_kind,
                is_volatile: attr.volatile(),
                is_const: attr.r#const(),
                is_unaligned: attr.unaligned(),
                is_restrict: attr.restrict(),
                is_reference: attr.islref() || attr.isrref(),
                size,
                is_mocom: attr.ismocom(),
            },
        })
    }
}

#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum PointerKind {
    Near16,
    Far16,
    Huge16,
    BaseSeg,
    BaseVal,
    BaseSegVal,
    BaseAddr,
    BaseSegAddr,
    BaseType,
    BaseSelf,
    Near32,
    Far32,
    Ptr64,
}

// Note: PointerKind conversion from ms-pdb is done in the Pointer TryFrom implementation
// by extracting the pointer_kind field from PointerFlags

impl Typed for PointerKind {
    fn type_size(&self, _pdb: &ParsedPdb) -> usize {
        match self {
            PointerKind::Near16 | PointerKind::Far16 | PointerKind::Huge16 => 2,
            PointerKind::Near32 | PointerKind::Far32 => 4,
            PointerKind::Ptr64 => 8,
            other @ (PointerKind::BaseSeg
            | PointerKind::BaseVal
            | PointerKind::BaseSegVal
            | PointerKind::BaseAddr
            | PointerKind::BaseSegAddr
            | PointerKind::BaseType
            | PointerKind::BaseSelf) => {
                warn!(
                    "type_size() called for pointer type {:?}, returning 4 bytes",
                    other
                );
                4
            }
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct PointerAttributes {
    pub kind: PointerKind,
    pub is_volatile: bool,
    pub is_const: bool,
    pub is_unaligned: bool,
    pub is_restrict: bool,
    pub is_reference: bool,
    pub size: usize,
    pub is_mocom: bool,
}

// Note: PointerAttributes conversion is handled in the Pointer TryFrom implementation
// by extracting fields from PointerFlags in ms-pdb

/// A primitive type from the PDB type system.
///
/// Primitive types are encoded directly in TypeIndex values (rather than requiring
/// separate type records). The TypeIndex encodes both the base type kind and optional
/// indirection (pointer) information.
///
/// # Encoding
///
/// - Bits 0-7: Type kind (see [`PrimitiveKind`])
/// - Bits 8-11: Indirection mode (see [`Indirection`])
///
/// Primitive types are identified by TypeIndex values below the type record range
/// (typically < 0x1000).
///
/// # References
///
/// - Microsoft PDB format: `cvinfo.h` (CV_typ_e enumeration)
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Primitive {
    pub kind: PrimitiveKind,
    pub indirection: Option<Indirection>,
}

impl Typed for Primitive {
    fn type_size(&self, _pdb: &ParsedPdb) -> usize {
        self.size()
    }
}

impl Primitive {
    pub fn size(&self) -> usize {
        if let Some(indirection) = self.indirection.as_ref() {
            return indirection.size();
        }

        self.kind.size()
    }
}

/// Pointer indirection modes for primitive types.
///
/// These modes specify the pointer type when a primitive TypeIndex includes indirection
/// information (bits 8-11 of the TypeIndex value). Different modes represent different
/// memory models and pointer sizes used in various architectures.
///
/// # References
///
/// - CodeView specification: MODE_* constants in `cvinfo.h`
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum Indirection {
    /// 16-bit near pointer
    Near16,
    /// 16-bit far pointer
    Far16,
    /// 16-bit huge pointer
    Huge16,
    /// 32-bit near pointer
    Near32,
    /// 32-bit far pointer
    Far32,
    /// 64-bit pointer
    Near64,
    /// 128-bit pointer
    Near128,
}

impl Typed for Indirection {
    fn type_size(&self, _pdb: &ParsedPdb) -> usize {
        self.size()
    }
}

impl Indirection {
    pub fn size(&self) -> usize {
        match self {
            Indirection::Near16 | Indirection::Far16 | Indirection::Huge16 => 2,
            Indirection::Near32 | Indirection::Far32 => 4,
            Indirection::Near64 => 8,
            Indirection::Near128 => 8,
        }
    }
}

/// Primitive type kinds as defined by the Microsoft PDB format.
///
/// These types correspond to the CV_typ_e enumeration in the CodeView debug format.
/// In PDB files, primitive types use special TypeIndex values where the type information
/// is encoded directly in the index rather than requiring a separate type record.
///
/// # Platform Dependencies
///
/// Note that some types like `Long`, `ULong`, `Quad`, and `UQuad` have platform-dependent
/// sizes, while explicitly-sized types like `I32`, `U32`, `I64`, `U64` have fixed sizes.
///
/// # References
///
/// - Microsoft Debug Interface Access SDK: `cvinfo.h`
/// - ms-pdb type constants: [`ms_pdb::codeview::types::primitive`](https://github.com/microsoft/pdb-rs/blob/main/crates/codeview/src/types/primitive.rs)
#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum PrimitiveKind {
    NoType,
    Void,
    Char,
    UChar,
    RChar,
    WChar,
    RChar16,
    RChar32,
    I8,
    U8,
    Short,
    UShort,
    I16,
    U16,
    Long,
    ULong,
    I32,
    U32,
    Quad,
    UQuad,
    I64,
    U64,
    Octa,
    UOcta,
    I128,
    U128,
    F16,
    F32,
    F32PP,
    F48,
    F64,
    F80,
    F128,
    Complex32,
    Complex64,
    Complex80,
    Complex128,
    Bool8,
    Bool16,
    Bool32,
    Bool64,
    HRESULT,
}

impl Typed for PrimitiveKind {
    fn type_size(&self, _pdb: &ParsedPdb) -> usize {
        self.size()
    }
}

impl PrimitiveKind {
    pub fn size(&self) -> usize {
        match self {
            PrimitiveKind::NoType | PrimitiveKind::Void => 0,

            PrimitiveKind::Char
            | PrimitiveKind::UChar
            | PrimitiveKind::RChar
            | PrimitiveKind::I8
            | PrimitiveKind::U8
            | PrimitiveKind::Bool8 => 1,

            PrimitiveKind::RChar16
            | PrimitiveKind::WChar
            | PrimitiveKind::Short
            | PrimitiveKind::UShort
            | PrimitiveKind::I16
            | PrimitiveKind::U16
            | PrimitiveKind::F16
            | PrimitiveKind::Bool16 => 2,

            PrimitiveKind::RChar32
            | PrimitiveKind::Long
            | PrimitiveKind::ULong
            | PrimitiveKind::I32
            | PrimitiveKind::U32
            | PrimitiveKind::F32
            | PrimitiveKind::F32PP
            | PrimitiveKind::Bool32
            | PrimitiveKind::HRESULT
            | PrimitiveKind::Complex32 => 4,

            PrimitiveKind::F48 => 6,

            PrimitiveKind::Quad
            | PrimitiveKind::UQuad
            | PrimitiveKind::I64
            | PrimitiveKind::U64
            | PrimitiveKind::F64
            | PrimitiveKind::Bool64
            | PrimitiveKind::Complex64 => 8,

            PrimitiveKind::Octa
            | PrimitiveKind::UOcta
            | PrimitiveKind::I128
            | PrimitiveKind::U128 => 16,

            PrimitiveKind::F80 | PrimitiveKind::Complex80 => 10,

            PrimitiveKind::F128 | PrimitiveKind::Complex128 => 16,
        }
    }
}

impl std::fmt::Display for PrimitiveKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimitiveKind::NoType => write!(f, "NoType"),
            PrimitiveKind::Void => write!(f, "Void"),
            PrimitiveKind::Char => write!(f, "Char"),
            PrimitiveKind::UChar => write!(f, "UChar"),
            PrimitiveKind::RChar => write!(f, "RChar"),
            PrimitiveKind::WChar => write!(f, "WChar"),
            PrimitiveKind::RChar16 => write!(f, "RChar16"),
            PrimitiveKind::RChar32 => write!(f, "RChar32"),
            PrimitiveKind::I8 => write!(f, "I8"),
            PrimitiveKind::U8 => write!(f, "U8"),
            PrimitiveKind::Short => write!(f, "Short"),
            PrimitiveKind::UShort => write!(f, "UShort"),
            PrimitiveKind::I16 => write!(f, "I16"),
            PrimitiveKind::U16 => write!(f, "U16"),
            PrimitiveKind::Long => write!(f, "Long"),
            PrimitiveKind::ULong => write!(f, "ULong"),
            PrimitiveKind::I32 => write!(f, "I32"),
            PrimitiveKind::U32 => write!(f, "U32"),
            PrimitiveKind::Quad => write!(f, "Quad"),
            PrimitiveKind::UQuad => write!(f, "UQuad"),
            PrimitiveKind::I64 => write!(f, "I64"),
            PrimitiveKind::U64 => write!(f, "U64"),
            PrimitiveKind::Octa => write!(f, "Octa"),
            PrimitiveKind::UOcta => write!(f, "UOcta"),
            PrimitiveKind::I128 => write!(f, "I128"),
            PrimitiveKind::U128 => write!(f, "U128"),
            PrimitiveKind::F16 => write!(f, "F16"),
            PrimitiveKind::F32 => write!(f, "F32"),
            PrimitiveKind::F32PP => write!(f, "F32PP"),
            PrimitiveKind::F48 => write!(f, "F48"),
            PrimitiveKind::F64 => write!(f, "F64"),
            PrimitiveKind::F80 => write!(f, "F80"),
            PrimitiveKind::F128 => write!(f, "F128"),
            PrimitiveKind::Complex32 => write!(f, "Complex32"),
            PrimitiveKind::Complex64 => write!(f, "Complex64"),
            PrimitiveKind::Complex80 => write!(f, "Complex80"),
            PrimitiveKind::Complex128 => write!(f, "Complex128"),
            PrimitiveKind::Bool8 => write!(f, "Bool8"),
            PrimitiveKind::Bool16 => write!(f, "Bool16"),
            PrimitiveKind::Bool32 => write!(f, "Bool32"),
            PrimitiveKind::Bool64 => write!(f, "Bool64"),
            PrimitiveKind::HRESULT => write!(f, "HRESULT"),
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Array {
    pub element_type: TypeRef,
    pub indexing_type: TypeRef,
    pub stride: Option<u32>,
    pub size: usize,
    pub dimensions_bytes: Vec<usize>,
    pub dimensions_elements: Vec<usize>,
}

impl Typed for Array {
    fn type_size(&self, _pdb: &ParsedPdb) -> usize {
        self.size
    }

    fn on_complete(&mut self, pdb: &ParsedPdb) {
        self.dimensions_elements.clear();

        if self.size == 0 {
            self.dimensions_elements.push(0);
            return;
        }

        let element_size = self.element_type.as_ref().borrow().type_size(pdb);

        // Calculate element counts for each dimension
        // For multi-dimensional arrays, dimensions_bytes contains sizes for each dimension
        if element_size == 0 {
            warn!("Array element type has zero size, cannot calculate dimensions");
            self.dimensions_elements.push(0);
            return;
        }

        for byte_size in &self.dimensions_bytes {
            let element_count = *byte_size / element_size;
            self.dimensions_elements.push(element_count);
        }
    }
}

type FromArray<'a, 'b> = (
    &'b ms_pdb::codeview::types::Array<'a>,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromArray<'_, '_>> for Array {
    type Error = Error;
    fn try_from(data: FromArray<'_, '_>) -> Result<Self, Self::Error> {
        let (array, type_stream, output_pdb) = data;

        let element_type =
            crate::handle_type(array.fixed.element_type.get(), output_pdb, type_stream)?;
        let indexing_type =
            crate::handle_type(array.fixed.index_type.get(), output_pdb, type_stream)?;
        let size = u64::try_from(array.len).unwrap_or(0) as usize;

        // Calculate stride from element type
        // Stride is the distance between consecutive elements
        // For basic arrays, this is just the element size
        let element_size = {
            let borrowed = element_type.as_ref().borrow();
            borrowed.type_size(&crate::symbol_types::ParsedPdb::new(
                std::path::PathBuf::new(),
            ))
        };

        let stride = if element_size > 0 {
            Some(element_size as u32)
        } else {
            None
        };

        let arr = Array {
            element_type,
            indexing_type,
            stride,
            size,
            dimensions_bytes: vec![size],    // Single dimension for now
            dimensions_elements: Vec::new(), // Filled in on_complete
        };

        Ok(arr)
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct FieldList(pub Vec<TypeRef>);

type FromFieldList<'a, 'b> = (
    ms_pdb::codeview::types::TypeIndex,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromFieldList<'_, '_>> for FieldList {
    type Error = Error;
    fn try_from(data: FromFieldList<'_, '_>) -> Result<Self, Self::Error> {
        let (field_list_index, type_stream, output_pdb) = data;

        let mut fields = Vec::new();

        // Use iter_fields() which correctly handles field list continuation chains
        // (fields can be split across multiple LF_FIELDLIST records)
        for field in type_stream.iter_fields(field_list_index) {
            match field {
                ms_pdb::codeview::types::fields::Field::Member(member) => {
                    match crate::handle_type(member.ty, output_pdb, type_stream) {
                        Ok(member_type) => {
                            let offset = u64::try_from(member.offset).unwrap_or(0) as usize;
                            // Create a Type::Member with the name
                            let member_with_name = Type::Member(Member {
                                name: member.name.to_string(),
                                underlying_type: member_type,
                                offset,
                            });
                            fields.push(Rc::new(RefCell::new(member_with_name)));
                        }
                        Err(e) => {
                            warn!("Failed to parse member field '{}': {}", member.name, e);
                            // Continue parsing other fields instead of failing entire struct
                        }
                    }
                }
                ms_pdb::codeview::types::fields::Field::StaticMember(static_member) => {
                    match crate::handle_type(static_member.ty, output_pdb, type_stream) {
                        Ok(member_type) => {
                            // Create a Type::StaticMember with the name
                            let static_member_with_name = Type::StaticMember(StaticMember {
                                name: static_member.name.to_string(),
                                field_type: member_type,
                            });
                            fields.push(Rc::new(RefCell::new(static_member_with_name)));
                        }
                        Err(e) => {
                            warn!(
                                "Failed to parse static member '{}': {}",
                                static_member.name, e
                            );
                        }
                    }
                }
                ms_pdb::codeview::types::fields::Field::BaseClass(base_class) => {
                    match crate::handle_type(base_class.ty, output_pdb, type_stream) {
                        Ok(base_type) => {
                            let offset = u64::try_from(base_class.offset).unwrap_or(0) as usize;

                            // Determine ClassKind from the base class type itself
                            let kind = {
                                let borrowed = base_type.as_ref().borrow();
                                match &*borrowed {
                                    Type::Class(c) => c.kind,
                                    _ => ClassKind::Struct, // Default if not a class type
                                }
                            };

                            // Create a Type::BaseClass
                            let base_class_type = Type::BaseClass(BaseClass {
                                kind,
                                base_class: base_type,
                                offset,
                            });
                            fields.push(Rc::new(RefCell::new(base_class_type)));
                        }
                        Err(e) => {
                            warn!("Failed to parse base class: {}", e);
                        }
                    }
                }
                ms_pdb::codeview::types::fields::Field::DirectVirtualBaseClass(vbase) => {
                    match (
                        crate::handle_type(vbase.fixed.btype.get(), output_pdb, type_stream),
                        crate::handle_type(vbase.fixed.vbtype.get(), output_pdb, type_stream),
                    ) {
                        (Ok(base_type), Ok(base_pointer)) => {
                            let base_pointer_offset =
                                u64::try_from(vbase.vbpoff).unwrap_or(0) as usize;
                            let virtual_base_offset =
                                u64::try_from(vbase.vboff).unwrap_or(0) as usize;
                            // Create a Type::VirtualBaseClass
                            let vbase_type = Type::VirtualBaseClass(VirtualBaseClass {
                                direct: true,
                                base_class: base_type,
                                base_pointer,
                                base_pointer_offset,
                                virtual_base_offset,
                            });
                            fields.push(Rc::new(RefCell::new(vbase_type)));
                        }
                        _ => {
                            warn!("Failed to parse direct virtual base class");
                        }
                    }
                }
                ms_pdb::codeview::types::fields::Field::IndirectVirtualBaseClass(vbase) => {
                    match (
                        crate::handle_type(vbase.fixed.btype.get(), output_pdb, type_stream),
                        crate::handle_type(vbase.fixed.vbtype.get(), output_pdb, type_stream),
                    ) {
                        (Ok(base_type), Ok(base_pointer)) => {
                            let base_pointer_offset =
                                u64::try_from(vbase.vbpoff).unwrap_or(0) as usize;
                            let virtual_base_offset =
                                u64::try_from(vbase.vboff).unwrap_or(0) as usize;
                            // Create a Type::VirtualBaseClass
                            let vbase_type = Type::VirtualBaseClass(VirtualBaseClass {
                                direct: false,
                                base_class: base_type,
                                base_pointer,
                                base_pointer_offset,
                                virtual_base_offset,
                            });
                            fields.push(Rc::new(RefCell::new(vbase_type)));
                        }
                        _ => {
                            warn!("Failed to parse indirect virtual base class");
                        }
                    }
                }
                ms_pdb::codeview::types::fields::Field::NestedType(nested) => {
                    match crate::handle_type(nested.nested_ty, output_pdb, type_stream) {
                        Ok(nested_type) => {
                            // Create a Type::Nested
                            let nested_with_name = Type::Nested(Nested {
                                name: nested.name.to_string(),
                                nested_type,
                            });
                            fields.push(Rc::new(RefCell::new(nested_with_name)));
                        }
                        Err(e) => {
                            warn!("Failed to parse nested type '{}': {}", nested.name, e);
                        }
                    }
                }
                ms_pdb::codeview::types::fields::Field::OneMethod(_method) => {
                    // Methods don't have a separate type, but we could create a placeholder
                    // For now, skip methods as they're not data fields
                }
                ms_pdb::codeview::types::fields::Field::Method(_method) => {
                    // Skip method lists for now
                }
                ms_pdb::codeview::types::fields::Field::Enumerate(enumerate) => {
                    // Enumerate fields are enum variants - convert them to EnumVariant type
                    match EnumVariant::try_from(&enumerate) {
                        Ok(variant) => {
                            let enum_variant_type = Type::EnumVariant(variant);
                            fields.push(Rc::new(RefCell::new(enum_variant_type)));
                        }
                        Err(e) => {
                            warn!("Failed to parse enum variant '{}': {}", enumerate.name, e);
                        }
                    }
                }
                ms_pdb::codeview::types::fields::Field::VFuncTable(vtable_type) => {
                    match crate::handle_type(vtable_type, output_pdb, type_stream) {
                        Ok(vtable) => {
                            // VFuncTables are stored as VTable type (tuple struct)
                            let vtable_type = Type::VTable(VTable(vtable));
                            fields.push(Rc::new(RefCell::new(vtable_type)));
                        }
                        Err(e) => {
                            warn!("Failed to parse VFuncTable: {}", e);
                        }
                    }
                }
                _ => {
                    // Skip unknown field types
                }
            }
        }

        Ok(FieldList(fields))
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct ArgumentList(pub Vec<TypeRef>);

type FromArgumentList<'a, 'b> = (
    &'b ms_pdb::codeview::types::ArgList<'a>,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromArgumentList<'_, '_>> for ArgumentList {
    type Error = Error;
    fn try_from(data: FromArgumentList<'_, '_>) -> Result<Self, Self::Error> {
        let (arg_list, type_stream, output_pdb) = data;

        let arguments: Result<Vec<TypeRef>, Self::Error> = arg_list
            .args
            .iter()
            .map(|typ| crate::handle_type(typ.get(), output_pdb, type_stream))
            .collect();

        Ok(ArgumentList(arguments?))
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Modifier {
    pub underlying_type: TypeRef,
    pub constant: bool,
    pub volatile: bool,
    pub unaligned: bool,
}

type FromModifier<'a, 'b> = (
    &'b ms_pdb::codeview::types::TypeModifier,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromModifier<'_, '_>> for Modifier {
    type Error = Error;
    fn try_from(data: FromModifier<'_, '_>) -> Result<Self, Self::Error> {
        let (modifier, type_stream, output_pdb) = data;

        let underlying_type =
            crate::handle_type(modifier.underlying_type.get(), output_pdb, type_stream)?;

        Ok(Modifier {
            underlying_type,
            constant: modifier.is_const(),
            volatile: modifier.is_volatile(),
            unaligned: modifier.is_unaligned(),
        })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Member {
    pub name: String,
    pub underlying_type: TypeRef,
    pub offset: usize,
}

type FromMember<'a, 'b> = (
    &'b ms_pdb::codeview::types::fields::Member<'a>,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromMember<'_, '_>> for Member {
    type Error = Error;

    fn try_from(data: FromMember<'_, '_>) -> Result<Self, Self::Error> {
        let (member, type_stream, output_pdb) = data;

        let underlying_type = crate::handle_type(member.ty, output_pdb, type_stream)?;
        let offset = u64::try_from(member.offset).unwrap_or(0) as usize;

        Ok(Member {
            name: member.name.to_string(),
            underlying_type,
            offset,
        })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Procedure {
    pub return_type: Option<TypeRef>,
    pub argument_list: Vec<TypeRef>,
    pub attributes: FunctionAttributes,
}

type FromProcedure<'a, 'b> = (
    &'b ms_pdb::codeview::types::Proc,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromProcedure<'_, '_>> for Procedure {
    type Error = Error;
    fn try_from(data: FromProcedure<'_, '_>) -> Result<Self, Self::Error> {
        let (proc, type_stream, output_pdb) = data;

        let return_type = crate::handle_type(proc.return_value.get(), output_pdb, type_stream).ok();

        let arguments: Vec<TypeRef>;
        let field = crate::handle_type(proc.arg_list.get(), output_pdb, type_stream)?;
        if let Type::ArgumentList(argument_list) = &*field.as_ref().borrow() {
            arguments = argument_list.0.clone();
        } else {
            // Return empty if not an argument list
            arguments = Vec::new();
        }

        Ok(Procedure {
            return_type,
            argument_list: arguments,
            attributes: FunctionAttributes {
                calling_convention: proc.call,
                // cxx_return_udt would be encoded in the calling convention byte
                // Bit 5 (0x20) indicates C++ return UDT
                cxx_return_udt: (proc.call & 0x20) != 0,
                is_constructor: false, // Not directly available in Proc type
                is_constructor_with_virtual_bases: false, // Not directly available in Proc type
            },
        })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct FunctionAttributes {
    pub calling_convention: u8,
    pub cxx_return_udt: bool,
    pub is_constructor: bool,
    pub is_constructor_with_virtual_bases: bool,
}

// NOTE: FunctionAttributes conversion not needed - created inline in Procedure/MemberFunction
// impl TryFrom<pdb::FunctionAttributes> for FunctionAttributes {
//     type Error = Error;
//     fn try_from(data: pdb::FunctionAttributes) -> Result<Self, Self::Error> {
//         Ok(FunctionAttributes {
//             calling_convention: data.calling_convention(),
//             cxx_return_udt: data.cxx_return_udt(),
//             is_constructor: data.is_constructor(),
//             is_constructor_with_virtual_bases: data.is_constructor_with_virtual_bases(),
//         })
//     }
// }

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct MemberFunction {
    pub return_type: TypeRef,
    pub class_type: TypeRef,
    pub this_pointer_type: Option<TypeRef>,
    pub argument_list: Vec<TypeRef>,
    pub attributes: FunctionAttributes,
    pub this_adjustment: u32,
}

type FromMemberFunction<'a, 'b> = (
    &'b ms_pdb::codeview::types::MemberFunc,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromMemberFunction<'_, '_>> for MemberFunction {
    type Error = Error;
    fn try_from(data: FromMemberFunction<'_, '_>) -> Result<Self, Self::Error> {
        let (member, type_stream, output_pdb) = data;

        let return_type = crate::handle_type(member.return_value.get(), output_pdb, type_stream)?;
        let class_type = crate::handle_type(member.class.get(), output_pdb, type_stream)?;
        let this_pointer_type = crate::handle_type(member.this.get(), output_pdb, type_stream).ok();

        let arguments: Vec<TypeRef>;
        let field = crate::handle_type(member.arg_list.get(), output_pdb, type_stream)?;
        if let Type::ArgumentList(argument_list) = &*field.as_ref().borrow() {
            arguments = argument_list.0.clone();
        } else {
            arguments = Vec::new();
        }

        Ok(MemberFunction {
            return_type,
            class_type,
            this_pointer_type,
            argument_list: arguments,
            attributes: FunctionAttributes {
                calling_convention: member.call,
                // cxx_return_udt would be encoded in the calling convention byte
                // Bit 5 (0x20) indicates C++ return UDT
                cxx_return_udt: (member.call & 0x20) != 0,
                // Constructor detection could be done by checking if the function name
                // matches the class name, but we don't have access to the function name here
                is_constructor: false,
                is_constructor_with_virtual_bases: false,
            },
            this_adjustment: member.this_adjust.get(),
        })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct MethodList(pub Vec<MethodListEntry>);

// NOTE: MethodList in ms-pdb is MethodListData with an iterator
// For now, we'll create an empty implementation or simplify
type FromMethodList<'a, 'b> = (
    &'b ms_pdb::codeview::types::MethodListData<'a>,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromMethodList<'_, '_>> for MethodList {
    type Error = Error;
    fn try_from(data: FromMethodList<'_, '_>) -> Result<Self, Self::Error> {
        let (_method_list_data, _type_stream, _output_pdb) = data;

        // NOTE: MethodListData in ms-pdb doesn't currently provide an iterator
        // to access individual method list items. This would require parsing the
        // raw byte data, which is complex. For now, we return an empty list.
        // This is a known limitation that should be addressed in ms-pdb.
        warn!("MethodList parsing is not yet fully supported in ms-pdb - returning empty list");
        Ok(MethodList(Vec::new()))
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct MethodListEntry {
    pub method_type: TypeRef,
    pub vtable_offset: Option<usize>,
}

// NOTE: ms-pdb has MethodListItem struct instead of MethodListEntry
type FromMethodListEntry<'a, 'b> = (
    &'b ms_pdb::codeview::types::MethodListItem,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromMethodListEntry<'_, '_>> for MethodListEntry {
    type Error = Error;
    fn try_from(data: FromMethodListEntry<'_, '_>) -> Result<Self, Self::Error> {
        let (method_item, type_stream, output_pdb) = data;

        let method_type = crate::handle_type(method_item.ty, output_pdb, type_stream)?;

        Ok(MethodListEntry {
            method_type,
            vtable_offset: method_item.vtab_offset.map(|o| o as usize),
        })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Nested {
    pub name: String,
    pub nested_type: TypeRef,
}

type FromNested<'a, 'b> = (
    &'b ms_pdb::codeview::types::fields::NestedType<'a>,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromNested<'_, '_>> for Nested {
    type Error = Error;
    fn try_from(data: FromNested<'_, '_>) -> Result<Self, Self::Error> {
        let (nested, type_stream, output_pdb) = data;

        let nested_type = crate::handle_type(nested.nested_ty, output_pdb, type_stream)?;

        Ok(Nested {
            name: nested.name.to_string(),
            nested_type,
        })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct OverloadedMethod {
    pub name: String,
    pub method_list: TypeRef,
}

type FromOverloadedMethod<'a, 'b> = (
    &'b ms_pdb::codeview::types::fields::Method<'a>,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromOverloadedMethod<'_, '_>> for OverloadedMethod {
    type Error = Error;
    fn try_from(data: FromOverloadedMethod<'_, '_>) -> Result<Self, Self::Error> {
        let (method, type_stream, output_pdb) = data;

        let method_list = crate::handle_type(method.methods, output_pdb, type_stream)?;

        Ok(OverloadedMethod {
            name: method.name.to_string(),
            method_list,
        })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Method {
    pub name: String,
    pub method_type: TypeRef,
    pub vtable_offset: Option<usize>,
}

// NOTE: ms-pdb has OneMethod in fields module
type FromMethod<'a, 'b> = (
    &'b ms_pdb::codeview::types::fields::OneMethod<'a>,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromMethod<'_, '_>> for Method {
    type Error = Error;
    fn try_from(data: FromMethod<'_, '_>) -> Result<Self, Self::Error> {
        let (method, type_stream, output_pdb) = data;

        let method_type = crate::handle_type(method.ty, output_pdb, type_stream)?;

        Ok(Method {
            name: method.name.to_string(),
            method_type,
            vtable_offset: if method.vbaseoff != 0 {
                Some(method.vbaseoff as usize)
            } else {
                None
            },
        })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct StaticMember {
    pub name: String,
    pub field_type: TypeRef,
}

type FromStaticMember<'a, 'b> = (
    &'b ms_pdb::codeview::types::fields::StaticMember<'a>,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromStaticMember<'_, '_>> for StaticMember {
    type Error = Error;
    fn try_from(data: FromStaticMember<'_, '_>) -> Result<Self, Self::Error> {
        let (member, type_stream, output_pdb) = data;

        let field_type = crate::handle_type(member.ty, output_pdb, type_stream)?;

        Ok(StaticMember {
            name: member.name.to_string(),
            field_type,
        })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct VTable(pub TypeRef);

// NOTE: ms-pdb doesn't have a separate VirtualFunctionTablePointerType
// VFuncTable field in FieldList enum represents this
// For now, create a simplified placeholder implementation
type FromVirtualFunctionTablePointer<'a, 'b> = (
    ms_pdb::codeview::types::TypeIndex,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromVirtualFunctionTablePointer<'_, '_>> for VTable {
    type Error = Error;
    fn try_from(data: FromVirtualFunctionTablePointer<'_, '_>) -> Result<Self, Self::Error> {
        let (table_index, type_stream, output_pdb) = data;

        let vtable_type = crate::handle_type(table_index, output_pdb, type_stream)?;

        Ok(VTable(vtable_type))
    }
}

// ============================================================================
// IPI Stream Types
// ============================================================================

/// Represents a function identifier from the IPI (ID Program Information) stream.
///
/// FuncId records store metadata about functions, including their name and type signature.
/// These are used for linking symbols to their type information and for organizing
/// functions within their parent scopes (namespaces, classes, etc.).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct FuncIdType {
    /// The name of the function
    pub name: String,
    /// The function's type signature (if available)
    pub function_type: Option<TypeRef>,
    /// The parent scope containing this function (namespace, class, etc.)
    pub parent_scope: Option<TypeRef>,
}

type FromFuncId<'a, 'b> = (
    &'b ms_pdb::codeview::types::FuncId<'a>,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromFuncId<'_, '_>> for FuncIdType {
    type Error = Error;
    fn try_from(data: FromFuncId<'_, '_>) -> Result<Self, Self::Error> {
        let (func_id, type_stream, output_pdb) = data;

        let function_type = if func_id.fixed.func_type.get().0 != 0 {
            let type_idx = func_id.fixed.func_type.get();
            // FuncId.function_type references a Procedure (TPI type) - check TPI first
            if let Some(typ) = crate::lookup_tpi_type(type_idx.0, output_pdb) {
                Some(typ)
            } else {
                crate::lookup_ipi_type(type_idx.0, output_pdb)
            }
        } else {
            None
        };

        let parent_scope = if func_id.fixed.scope.get() != 0 {
            let type_idx = ms_pdb::codeview::types::TypeIndex(func_id.fixed.scope.get());
            // FuncId.parent_scope is typically a StringId (IPI type) - ONLY check IPI
            if let Some(typ) = crate::lookup_ipi_type(type_idx.0, output_pdb) {
                Some(typ)
            } else {
                // Not yet parsed - parse from IPI stream
                Some(crate::handle_ipi_type(type_idx, output_pdb, type_stream)?)
            }
        } else {
            None
        };

        Ok(FuncIdType {
            name: func_id.name.to_string(),
            function_type,
            parent_scope,
        })
    }
}

/// Represents a member function identifier from the IPI stream.
///
/// Similar to `FuncIdType`, but specifically for class/struct member functions.
/// Contains the member function's name, type signature, and parent class.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct MFuncIdType {
    /// The name of the member function
    pub name: String,
    /// The member function's type signature (if available)
    pub function_type: Option<TypeRef>,
    /// The parent class/struct that owns this member function
    pub parent_type: Option<TypeRef>,
}

type FromMFuncId<'a, 'b> = (
    &'b ms_pdb::codeview::types::MFuncId<'a>,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromMFuncId<'_, '_>> for MFuncIdType {
    type Error = Error;
    fn try_from(data: FromMFuncId<'_, '_>) -> Result<Self, Self::Error> {
        let (mfunc_id, _type_stream, output_pdb) = data;

        let function_type = if mfunc_id.fixed.func_type.get().0 != 0 {
            let type_idx = mfunc_id.fixed.func_type.get();
            // MFuncId.function_type references a MemberFunction (TPI type) - check TPI first
            if let Some(typ) = crate::lookup_tpi_type(type_idx.0, output_pdb) {
                Some(typ)
            } else {
                crate::lookup_ipi_type(type_idx.0, output_pdb)
            }
        } else {
            None
        };

        let parent_type = if mfunc_id.fixed.parent_type.get().0 != 0 {
            let type_idx = mfunc_id.fixed.parent_type.get();
            // MFuncId.parent_type references a Class (TPI type) - check TPI first
            if let Some(typ) = crate::lookup_tpi_type(type_idx.0, output_pdb) {
                Some(typ)
            } else {
                crate::lookup_ipi_type(type_idx.0, output_pdb)
            }
        } else {
            None
        };

        Ok(MFuncIdType {
            name: mfunc_id.name.to_string(),
            function_type,
            parent_type,
        })
    }
}

/// Represents a string identifier from the IPI stream.
///
/// StringId records store string values used in build information, source paths,
/// compiler arguments, and other metadata. Strings can be hierarchical, where
/// one string references another as a substring.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct StringIdType {
    /// The string value
    pub id: String,
    /// Optional reference to another string that this string extends or modifies
    pub substring: Option<TypeRef>,
}

type FromStringId<'a, 'b> = (
    &'b ms_pdb::codeview::types::StringId<'a>,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromStringId<'_, '_>> for StringIdType {
    type Error = Error;
    fn try_from(data: FromStringId<'_, '_>) -> Result<Self, Self::Error> {
        let (string_id, type_stream, output_pdb) = data;

        let substring = if string_id.id != 0 {
            let type_idx = ms_pdb::codeview::types::TypeIndex(string_id.id);
            // StringId.substring is another StringId (IPI type) - ONLY check IPI
            if let Some(typ) = crate::lookup_ipi_type(type_idx.0, output_pdb) {
                Some(typ)
            } else {
                // Not yet parsed - parse from IPI stream
                Some(crate::handle_ipi_type(type_idx, output_pdb, type_stream)?)
            }
        } else {
            None
        };

        Ok(StringIdType {
            id: string_id.name.to_string(),
            substring,
        })
    }
}

/// Represents a list of string identifiers from the IPI stream.
///
/// SubStrList is used to group multiple StringId records together, typically
/// for representing lists of paths, arguments, or other string collections.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct SubStrListType {
    /// List of string identifiers
    pub strings: Vec<TypeRef>,
}

type FromSubStrList<'a, 'b> = (
    &'b ms_pdb::codeview::types::SubStrList<'a>,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromSubStrList<'_, '_>> for SubStrListType {
    type Error = Error;
    fn try_from(data: FromSubStrList<'_, '_>) -> Result<Self, Self::Error> {
        let (substr_list, type_stream, output_pdb) = data;

        let strings: Result<Vec<TypeRef>, Error> = substr_list
            .ids
            .iter()
            .map(|id| id.get())
            .filter(|id| *id != 0)
            .map(|id| {
                let type_idx = ms_pdb::codeview::types::TypeIndex(id);
                // SubStrList contains StringIds (IPI types) - ONLY check IPI
                if let Some(typ) = crate::lookup_ipi_type(type_idx.0, output_pdb) {
                    Ok(typ)
                } else {
                    // Not yet parsed - parse from IPI stream
                    crate::handle_ipi_type(type_idx, output_pdb, type_stream)
                }
            })
            .collect();

        Ok(SubStrListType { strings: strings? })
    }
}

/// Represents build information from the IPI stream.
///
/// BuildInfo records contain metadata about how the binary was compiled,
/// including:
/// - Current directory during compilation
/// - Compiler executable path
/// - Source file paths
/// - PDB output path
/// - Command line arguments
///
/// Each item is typically a `StringId` reference.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct BuildInfoTypeData {
    /// List of build information items (typically 5 items: cwd, compiler, source, pdb, cmdline)
    pub items: Vec<TypeRef>,
}

type FromBuildInfo<'a, 'b> = (
    &'b ms_pdb::codeview::types::BuildInfo<'a>,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromBuildInfo<'_, '_>> for BuildInfoTypeData {
    type Error = Error;
    fn try_from(data: FromBuildInfo<'_, '_>) -> Result<Self, Self::Error> {
        let (build_info, type_stream, output_pdb) = data;

        let items: Result<Vec<TypeRef>, Error> = build_info
            .args
            .iter()
            .map(|id| id.get())
            .filter(|id| *id != 0)
            .map(|id| {
                let type_idx = ms_pdb::codeview::types::TypeIndex(id);
                // BuildInfo ONLY references IPI types (StringIds) - ONLY check IPI, never TPI
                if let Some(typ) = crate::lookup_ipi_type(type_idx.0, output_pdb) {
                    Ok(typ)
                } else {
                    // Not yet parsed - parse from IPI stream
                    crate::handle_ipi_type(type_idx, output_pdb, type_stream)
                }
            })
            .collect();

        Ok(BuildInfoTypeData { items: items? })
    }
}

/// Represents source line information for a user-defined type (UDT) from the IPI stream.
///
/// UdtSrcLine records link a class, struct, union, or enum definition to its
/// source code location. This allows debuggers to navigate from a type to where
/// it was defined in the source code.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct UdtSrcLineType {
    /// Reference to the source file StringId (if available)
    pub source_file: Option<TypeRef>,
    /// Line number in the source file where the UDT is defined
    pub line_number: u32,
    /// Reference to the user-defined type (class, struct, union, or enum)
    pub udt: TypeRef,
}

type FromUdtSrcLine<'a, 'b> = (
    &'b ms_pdb::codeview::types::UdtSrcLine,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromUdtSrcLine<'_, '_>> for UdtSrcLineType {
    type Error = Error;
    fn try_from(data: FromUdtSrcLine<'_, '_>) -> Result<Self, Self::Error> {
        let (udt_src_line, _type_stream, output_pdb) = data;

        // src is a NameIndex, not a TypeIndex - it references the /names stream, not type stream
        // For now, we'll skip the source file lookup and just store the UDT type
        let source_file = None;

        // UdtSrcLine wraps a real UDT type. When parsing from IPI stream (where UdtSrcLine lives),
        // we need to resolve the wrapped UDT type. The wrapped type index might exist in BOTH
        // TPI and IPI streams (they share the same index space). We should prefer TPI since
        // UdtSrcLine.udt references a Class/Struct/Union (TPI type) - check TPI first
        let udt_type_idx = udt_src_line.ty.get();
        let udt = if let Some(typ) = crate::lookup_tpi_type(udt_type_idx.0, output_pdb) {
            typ
        } else if let Some(typ) = crate::lookup_ipi_type(udt_type_idx.0, output_pdb) {
            typ
        } else {
            // Not yet parsed - this is a cross-stream reference (IPI -> TPI)
            // For now return error since we can't parse TPI from IPI context
            return Err(Error::UnhandledType(format!(
                "UdtSrcLine references unparsed UDT at index 0x{:08x}",
                udt_type_idx.0
            )));
        };

        Ok(UdtSrcLineType {
            source_file,
            line_number: udt_src_line.line.get(),
            udt,
        })
    }
}

type FromUdtModSrcLine<'a, 'b> = (
    &'b ms_pdb::codeview::types::UdtModSrcLine,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromUdtModSrcLine<'_, '_>> for UdtSrcLineType {
    type Error = Error;
    fn try_from(data: FromUdtModSrcLine<'_, '_>) -> Result<Self, Self::Error> {
        let (udt_mod_src_line, _type_stream, output_pdb) = data;

        // Similar to UdtSrcLine, but includes module index (imod)
        // src is a NameIndex referencing /names stream
        let source_file = None;

        // UdtModSrcLine wraps a real UDT type. When parsing from IPI stream (where UdtModSrcLine lives),
        // we need to resolve the wrapped UDT type. The wrapped type index might exist in BOTH
        // TPI and IPI streams (they share the same index space). We should prefer TPI since
        // UdtModSrcLine.udt references a Class/Struct/Union (TPI type) - check TPI first
        let udt_type_idx = udt_mod_src_line.ty.get();
        let udt = if let Some(typ) = crate::lookup_tpi_type(udt_type_idx.0, output_pdb) {
            typ
        } else if let Some(typ) = crate::lookup_ipi_type(udt_type_idx.0, output_pdb) {
            typ
        } else {
            // Not yet parsed - this is a cross-stream reference (IPI -> TPI)
            return Err(Error::UnhandledType(format!(
                "UdtModSrcLine references unparsed UDT at index 0x{:08x}",
                udt_type_idx.0
            )));
        };

        Ok(UdtSrcLineType {
            source_file,
            line_number: udt_mod_src_line.line.get(),
            udt,
        })
    }
}

// ============================================================================
// Alias Type (LF_ALIAS) - TPI Stream
// ============================================================================

/// Represents a type alias (typedef in C/C++).
///
/// This type creates an alternate name for an existing type. For example:
/// ```c
/// typedef int MyInt;  // Creates an Alias with name="MyInt", underlying_type=int
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Alias {
    /// The name of the alias (e.g., "MyInt" for `typedef int MyInt`)
    pub name: String,
    /// The underlying type that this alias refers to
    pub underlying_type: TypeRef,
}

impl Typed for Alias {
    fn type_size(&self, pdb: &ParsedPdb) -> usize {
        self.underlying_type.borrow().type_size(pdb)
    }
}

type FromAlias<'a, 'b> = (
    &'b ms_pdb::codeview::types::Alias<'a>,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromAlias<'_, '_>> for Alias {
    type Error = Error;
    fn try_from(data: FromAlias<'_, '_>) -> Result<Self, Self::Error> {
        let (alias, type_stream, output_pdb) = data;

        let underlying_type = crate::handle_type(alias.utype, output_pdb, type_stream)?;

        Ok(Alias {
            name: alias.name.to_string(),
            underlying_type,
        })
    }
}

// ============================================================================
// VTableShape Type (LF_VTSHAPE) - TPI Stream
// ============================================================================

/// Describes the shape of a virtual function table (vtable).
///
/// A vtable shape defines how many virtual functions exist and their calling conventions.
/// Each virtual function slot is described by a 4-bit descriptor (nibble) packed into bytes.
///
/// The descriptors encode calling conventions:
/// - 0x00: Near
/// - 0x01: Far
/// - 0x02: Thin
/// - 0x03: Outer (adjustor thunk)
/// - 0x04: Meta (for virtual bases)
/// - 0x05: Near32
/// - 0x06: Far32
/// - And others...
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct VTableShape {
    /// Number of virtual function entries in the vtable
    pub count: u16,
    /// Packed 4-bit descriptors for each virtual function's calling convention.
    /// Contains `(count + 1) / 2` bytes, with potential padding for alignment.
    pub descriptors: Vec<u8>,
}

impl Typed for VTableShape {
    fn type_size(&self, _pdb: &ParsedPdb) -> usize {
        // VTableShape describes virtual function table layout
        // Size is count * pointer_size (typically 4 or 8 bytes per entry)
        // We'll assume 8 bytes for 64-bit (modern systems)
        self.count as usize * 8
    }
}

type FromVTableShape<'a, 'b> = (
    &'b ms_pdb::codeview::types::VTableShapeData<'a>,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromVTableShape<'_, '_>> for VTableShape {
    type Error = Error;
    fn try_from(data: FromVTableShape<'_, '_>) -> Result<Self, Self::Error> {
        let (vtshape, _type_stream, _output_pdb) = data;

        Ok(VTableShape {
            count: vtshape.count,
            descriptors: vtshape.descriptors.to_vec(),
        })
    }
}

// ============================================================================
// VFTable Type (LF_VFTABLE) - TPI Stream
// ============================================================================

/// Represents a virtual function table (vftable) with its location and inheritance path.
///
/// VFTables are used in C++ classes with virtual functions. This type describes
/// where the vtable is located in memory and which class hierarchy it belongs to.
///
/// # Fields
/// - `root`: The root class that owns this vtable
/// - `path`: Type index describing the inheritance path to this vtable
/// - `offset`: Offset from the class base where this vtable pointer is located
/// - `segment`: Section/segment index where the vtable data resides
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct VFTableType {
    /// The root class that owns this virtual function table
    pub root: TypeRef,
    /// Type describing the inheritance path to this vtable
    pub path: TypeRef,
    /// Offset in bytes from the class base address
    pub offset: u32,
    /// Section/segment index containing the vtable
    pub segment: u16,
}

impl Typed for VFTableType {
    fn type_size(&self, _pdb: &ParsedPdb) -> usize {
        // VFTable is a pointer to a virtual function table
        // Size is typically pointer size (8 bytes on 64-bit)
        8
    }
}

type FromVFTable<'a, 'b> = (
    &'a ms_pdb::codeview::types::VFTable,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromVFTable<'_, '_>> for VFTableType {
    type Error = Error;
    fn try_from(data: FromVFTable<'_, '_>) -> Result<Self, Self::Error> {
        let (vftable, type_stream, output_pdb) = data;

        // VFTable doesn't implement Copy, but it's a simple struct with Copy fields
        // Access fields directly from the reference
        let root = crate::handle_type(vftable.root.get(), output_pdb, type_stream)?;
        let path = crate::handle_type(vftable.path.get(), output_pdb, type_stream)?;

        Ok(VFTableType {
            root,
            path,
            offset: vftable.off.get(),
            segment: vftable.seg.get(),
        })
    }
}
