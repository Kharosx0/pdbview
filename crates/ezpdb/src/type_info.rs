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
            // IPI types don't have a size
            Type::FuncId(_) => panic!("type_size() invoked for FuncId"),
            Type::MFuncId(_) => panic!("type_size() invoked for MFuncId"),
            Type::StringId(_) => panic!("type_size() invoked for StringId"),
            Type::SubStrList(_) => panic!("type_size() invoked for SubStrList"),
            Type::BuildInfoType(_) => panic!("type_size() invoked for BuildInfoType"),
            Type::UdtSrcLineType(_) => panic!("type_size() invoked for UdtSrcLineType"),
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
        // Use catch_unwind for each bitfield access to handle potential overflow panics
        use std::panic::{catch_unwind, AssertUnwindSafe};

        Ok(TypeProperties {
            packed: catch_unwind(AssertUnwindSafe(|| props.packed())).unwrap_or(false),
            constructors: catch_unwind(AssertUnwindSafe(|| props.ctor())).unwrap_or(false),
            overlapped_operators: catch_unwind(AssertUnwindSafe(|| props.ovlops()))
                .unwrap_or(false),
            is_nested_type: catch_unwind(AssertUnwindSafe(|| props.isnested())).unwrap_or(false),
            contains_nested_types: catch_unwind(AssertUnwindSafe(|| props.cnested()))
                .unwrap_or(false),
            overload_assignment: catch_unwind(AssertUnwindSafe(|| props.opassign()))
                .unwrap_or(false),
            overload_coasting: catch_unwind(AssertUnwindSafe(|| props.opcast())).unwrap_or(false),
            forward_reference: catch_unwind(AssertUnwindSafe(|| props.fwdref())).unwrap_or(false),
            scoped_definition: catch_unwind(AssertUnwindSafe(|| props.scoped())).unwrap_or(false),
            has_unique_name: catch_unwind(AssertUnwindSafe(|| props.hasuniquename()))
                .unwrap_or(false),
            sealed: catch_unwind(AssertUnwindSafe(|| props.sealed())).unwrap_or(false),
            hfa: catch_unwind(AssertUnwindSafe(|| props.hfa() as u8)).unwrap_or(0),
            intristic_type: catch_unwind(AssertUnwindSafe(|| props.intrinsic())).unwrap_or(false),
            mocom: catch_unwind(AssertUnwindSafe(|| props.mocom() as u8)).unwrap_or(0),
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

        let fields: Vec<TypeRef> = if fields.0 != 0 {
            // TODO: perhaps change FieldList to Rc<Vec<TypeRef>>?
            if let Type::FieldList(fields_list) =
                &*crate::handle_type(fields, output_pdb, type_stream)?
                    .as_ref()
                    .borrow()
            {
                fields_list.0.clone()
            } else {
                panic!("got an unexpected type when FieldList was expected")
            }
        } else {
            vec![]
        };

        let derived_from = if derived_from.0 != 0 {
            Some(crate::handle_type(derived_from, output_pdb, type_stream)?)
        } else {
            None
        };

        let unique_name = class.unique_name.map(|s| s.to_string());

        Ok(Class {
            name: class.name.to_string(),
            unique_name,
            kind: ClassKind::Struct, // Default, would need Leaf type to determine
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

        Ok(BaseClass {
            kind: ClassKind::Struct, // Default, would need more context to determine
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

        let fields_type = crate::handle_type(fields, output_pdb, type_stream)?;

        let fields = {
            let borrowed_fields = fields_type.as_ref().borrow();
            match &*borrowed_fields {
                Type::FieldList(fields_list) => fields_list.0.clone(),
                _ => {
                    drop(borrowed_fields);
                    vec![fields_type]
                }
            }
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
    &'b ms_pdb::codeview::types::TypeIndex,
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
        let (type_index, type_stream, output_pdb) = data;

        // Note: ms-pdb doesn't have explicit Bitfield support in TypeData enum yet
        // LF_BITFIELD records map to Unknown. For now, we'll create a minimal implementation
        // that just wraps the type index
        let underlying_type = crate::handle_type(*type_index, output_pdb, type_stream)?;

        Ok(Bitfield {
            underlying_type,
            len: 0,      // TODO: Extract from raw type record if needed
            position: 0, // TODO: Extract from raw type record if needed
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

        let fields_type = crate::handle_type(fields, output_pdb, type_stream)?;

        let fields = {
            let borrowed_fields = fields_type.as_ref().borrow();
            match &*borrowed_fields {
                Type::FieldList(fields_list) => fields_list.0.clone(),
                _other => {
                    vec![]
                }
            }
        };

        let fields = fields
            .iter()
            .map(|field| {
                if let Type::EnumVariant(var) = &*field.borrow() {
                    var.clone()
                } else {
                    panic!("field {:?} is not an enumvariant", field)
                }
            })
            .collect::<Vec<_>>();

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

        // Try to extract size, but use a safe fallback if bitfield access panics
        let size = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| attr.size() as usize))
            .unwrap_or_else(|_| {
                // If bitfield access panics, infer size from pointer kind
                match attr.pointer_kind() {
                    0..=2 => 2,   // Near16, Far16, Huge16
                    10 | 11 => 4, // Near32, Far32
                    12 => 8,      // Ptr64
                    _ => 8,       // default to 64-bit
                }
            });

        Ok(Pointer {
            underlying_type,
            attributes: PointerAttributes {
                kind: match attr.pointer_kind() {
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
                    _ => PointerKind::Ptr64, // default to 64-bit
                },
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
            other => panic!("type_size() not implemented for pointer type: {:?}", other),
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

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Primitive {
    pub kind: PrimitiveKind,
    pub indirection: Option<Indirection>,
}

// Note: Primitive conversion from TypeIndex is handled in handle_type_data
// by checking if TypeIndex is < 0x1000 (primitive type range)

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

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum Indirection {
    Near16,
    Far16,
    Huge16,
    Near32,
    Far32,
    Near64,
    Near128,
}

// Note: Indirection is derived from pointer attributes in ms-pdb
// Conversion happens in the Pointer TryFrom implementation

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

// Note: PrimitiveKind is derived from TypeIndex in ms-pdb
// Primitive types are represented as special TypeIndex values
// Conversion happens in the Primitive TryFrom implementation

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

        let mut running_size = self.element_type.as_ref().borrow().type_size(pdb);

        for byte_size in &self.dimensions_bytes {
            // TODO: may be incorrect behavior
            if running_size == 0 {
                continue;
            }

            let size = *byte_size / running_size;

            self.dimensions_elements.push(size);

            running_size = size;
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

        let arr = Array {
            element_type,
            indexing_type,
            stride: None, // TODO: Stride calculation not available from Array structure
            size,
            dimensions_bytes: vec![size], // Simplified: single dimension
            dimensions_elements: Vec::new(),
        };

        Ok(arr)
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct FieldList(pub Vec<TypeRef>);

type FromFieldList<'a, 'b> = (
    &'b ms_pdb::codeview::types::FieldList<'a>,
    &'b ms_pdb::tpi::TypeStream<Vec<u8>>,
    &'b mut crate::symbol_types::ParsedPdb,
);

impl TryFrom<FromFieldList<'_, '_>> for FieldList {
    type Error = Error;
    fn try_from(data: FromFieldList<'_, '_>) -> Result<Self, Self::Error> {
        let (field_list, type_stream, output_pdb) = data;

        let mut fields = Vec::new();

        // Iterate through all fields in the field list
        for field in field_list.iter() {
            match field {
                ms_pdb::codeview::types::fields::Field::Member(member) => {
                    let member_type = crate::handle_type(member.ty, output_pdb, type_stream)?;
                    let offset = u64::try_from(member.offset).unwrap_or(0) as usize;
                    // Create a Type::Member with the name
                    let member_with_name = Type::Member(Member {
                        name: member.name.to_string(),
                        underlying_type: member_type,
                        offset,
                    });
                    fields.push(Rc::new(RefCell::new(member_with_name)));
                }
                ms_pdb::codeview::types::fields::Field::StaticMember(static_member) => {
                    let member_type =
                        crate::handle_type(static_member.ty, output_pdb, type_stream)?;
                    // Create a Type::StaticMember with the name
                    let static_member_with_name = Type::StaticMember(StaticMember {
                        name: static_member.name.to_string(),
                        field_type: member_type,
                    });
                    fields.push(Rc::new(RefCell::new(static_member_with_name)));
                }
                ms_pdb::codeview::types::fields::Field::BaseClass(base_class) => {
                    let base_type = crate::handle_type(base_class.ty, output_pdb, type_stream)?;
                    let offset = u64::try_from(base_class.offset).unwrap_or(0) as usize;
                    // Create a Type::BaseClass
                    let base_class_type = Type::BaseClass(BaseClass {
                        kind: ClassKind::Struct, // Default to Struct
                        base_class: base_type,
                        offset,
                    });
                    fields.push(Rc::new(RefCell::new(base_class_type)));
                }
                ms_pdb::codeview::types::fields::Field::DirectVirtualBaseClass(vbase) => {
                    let base_type =
                        crate::handle_type(vbase.fixed.btype.get(), output_pdb, type_stream)?;
                    let base_pointer =
                        crate::handle_type(vbase.fixed.vbtype.get(), output_pdb, type_stream)?;
                    let base_pointer_offset = u64::try_from(vbase.vbpoff).unwrap_or(0) as usize;
                    let virtual_base_offset = u64::try_from(vbase.vboff).unwrap_or(0) as usize;
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
                ms_pdb::codeview::types::fields::Field::IndirectVirtualBaseClass(vbase) => {
                    let base_type =
                        crate::handle_type(vbase.fixed.btype.get(), output_pdb, type_stream)?;
                    let base_pointer =
                        crate::handle_type(vbase.fixed.vbtype.get(), output_pdb, type_stream)?;
                    let base_pointer_offset = u64::try_from(vbase.vbpoff).unwrap_or(0) as usize;
                    let virtual_base_offset = u64::try_from(vbase.vboff).unwrap_or(0) as usize;
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
                ms_pdb::codeview::types::fields::Field::NestedType(nested) => {
                    let nested_type =
                        crate::handle_type(nested.nested_ty, output_pdb, type_stream)?;
                    // Create a Type::Nested
                    let nested_with_name = Type::Nested(Nested {
                        name: nested.name.to_string(),
                        nested_type,
                    });
                    fields.push(Rc::new(RefCell::new(nested_with_name)));
                }
                ms_pdb::codeview::types::fields::Field::OneMethod(_method) => {
                    // Methods don't have a separate type, but we could create a placeholder
                    // For now, skip methods as they're not data fields
                }
                ms_pdb::codeview::types::fields::Field::Method(_method) => {
                    // Skip method lists for now
                }
                ms_pdb::codeview::types::fields::Field::Enumerate(_enumerate) => {
                    // Enumerates are values, not types, so skip them
                }
                ms_pdb::codeview::types::fields::Field::VFuncTable(vtable_type) => {
                    let vtable = crate::handle_type(vtable_type, output_pdb, type_stream)?;
                    // VFuncTables are stored as VTable type (tuple struct)
                    let vtable_type = Type::VTable(VTable(vtable));
                    fields.push(Rc::new(RefCell::new(vtable_type)));
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
                cxx_return_udt: false, // TODO: Extract from call convention if needed
                is_constructor: false, // Not available in Proc
                is_constructor_with_virtual_bases: false, // Not available in Proc
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
                cxx_return_udt: false, // TODO: Extract from calling convention if needed
                is_constructor: false, // Not directly available
                is_constructor_with_virtual_bases: false, // Not directly available
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
        // TODO: Parse MethodListData.bytes to extract individual methods
        // This is complex and would require parsing the byte structure
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

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct FuncIdType {
    pub name: String,
    pub function_type: Option<TypeRef>,
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
            Some(crate::handle_type(
                func_id.fixed.func_type.get(),
                output_pdb,
                type_stream,
            )?)
        } else {
            None
        };

        let parent_scope = if func_id.fixed.scope.get() != 0 {
            Some(crate::handle_type(
                ms_pdb::codeview::types::TypeIndex(func_id.fixed.scope.get()),
                output_pdb,
                type_stream,
            )?)
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

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct MFuncIdType {
    pub name: String,
    pub function_type: Option<TypeRef>,
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
        let (mfunc_id, type_stream, output_pdb) = data;

        let function_type = if mfunc_id.fixed.func_type.get().0 != 0 {
            Some(crate::handle_type(
                mfunc_id.fixed.func_type.get(),
                output_pdb,
                type_stream,
            )?)
        } else {
            None
        };

        let parent_type = if mfunc_id.fixed.parent_type.get().0 != 0 {
            Some(crate::handle_type(
                mfunc_id.fixed.parent_type.get(),
                output_pdb,
                type_stream,
            )?)
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

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct StringIdType {
    pub id: String,
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
            Some(crate::handle_type(
                ms_pdb::codeview::types::TypeIndex(string_id.id),
                output_pdb,
                type_stream,
            )?)
        } else {
            None
        };

        Ok(StringIdType {
            id: string_id.name.to_string(),
            substring,
        })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct SubStrListType {
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
                crate::handle_type(
                    ms_pdb::codeview::types::TypeIndex(id),
                    output_pdb,
                    type_stream,
                )
            })
            .collect();

        Ok(SubStrListType { strings: strings? })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct BuildInfoTypeData {
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
                crate::handle_type(
                    ms_pdb::codeview::types::TypeIndex(id),
                    output_pdb,
                    type_stream,
                )
            })
            .collect();

        Ok(BuildInfoTypeData { items: items? })
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct UdtSrcLineType {
    pub source_file: Option<TypeRef>,
    pub line_number: u32,
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
        let (udt_src_line, type_stream, output_pdb) = data;

        // src is a NameIndex, not a TypeIndex - it references the /names stream, not type stream
        // For now, we'll skip the source file lookup and just store the UDT type
        let source_file = None;

        let udt = crate::handle_type(udt_src_line.ty.get(), output_pdb, type_stream)?;

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
        let (udt_mod_src_line, type_stream, output_pdb) = data;

        // Similar to UdtSrcLine, but includes module index (imod)
        // src is a NameIndex referencing /names stream
        let source_file = None;

        let udt = crate::handle_type(udt_mod_src_line.ty.get(), output_pdb, type_stream)?;

        Ok(UdtSrcLineType {
            source_file,
            line_number: udt_mod_src_line.line.get(),
            udt,
        })
    }
}
