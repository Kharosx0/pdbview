//! API Verification Tests
//!
//! This test file validates that the public API documented in ezpdb
//! actually works as advertised. It ensures documentation examples
//! are accurate and the API is correctly exposed.

use ezpdb::parse_pdb;
use ezpdb::type_info::Type;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
#[ignore] // Requires a real PDB file to exist
fn test_parse_pdb_basic_usage() {
    // This matches the example in the parse_pdb() documentation
    let result = parse_pdb("ntdll.pdb", None);

    // The example shows it returns Result<ParsedPdb, Error>
    match result {
        Ok(pdb) => {
            // The example accesses pdb.types.len()
            let _type_count = pdb.types.len();
            // The example accesses pdb.procedures.len()
            let _proc_count = pdb.procedures.len();

            // These field accesses should compile
            let _public_symbols = &pdb.public_symbols;
            let _ipi_types = &pdb.ipi_types;
            let _global_data = &pdb.global_data;
            let _debug_modules = &pdb.debug_modules;
            let _version = &pdb.version;
            let _guid = pdb.guid;
            let _age = pdb.age;
            let _timestamp = pdb.timestamp;
            let _machine_type = &pdb.machine_type;
        }
        Err(_e) => {
            // Error is of type ezpdb::error::Error
            // This is expected when the file doesn't exist
        }
    }
}

#[test]
fn test_type_enum_variants() {
    // Verify all documented Type enum variants are accessible
    // This ensures our documentation matches reality

    // Create dummy TypeRefs for testing
    let dummy_type: Rc<RefCell<Type>> =
        Rc::new(RefCell::new(Type::Primitive(ezpdb::type_info::Primitive {
            kind: ezpdb::type_info::PrimitiveKind::Void,
            indirection: None,
        })));

    // Test pattern matching on Type enum (as users would do)
    {
        let borrowed = dummy_type.borrow();
        match &*borrowed {
            Type::Class(_) => {}
            Type::VirtualBaseClass(_) => {}
            Type::Union(_) => {}
            Type::Bitfield(_) => {}
            Type::Enumeration(_) => {}
            Type::EnumVariant(_) => {}
            Type::Pointer(_) => {}
            Type::Primitive(_) => {}
            Type::Array(_) => {}
            Type::FieldList(_) => {}
            Type::ArgumentList(_) => {}
            Type::Modifier(_) => {}
            Type::Member(_) => {}
            Type::Procedure(_) => {}
            Type::MemberFunction(_) => {}
            Type::MethodList(_) => {}
            Type::MethodListEntry(_) => {}
            Type::Nested(_) => {}
            Type::OverloadedMethod(_) => {}
            Type::Method(_) => {}
            Type::StaticMember(_) => {}
            Type::BaseClass(_) => {}
            Type::VTable(_) => {}
            // New variants documented in our additions
            Type::Alias(_) => {}
            Type::VTableShape(_) => {}
            Type::VFTableType(_) => {}
            // IPI stream types
            Type::FuncId(_) => {}
            Type::MFuncId(_) => {}
            Type::StringId(_) => {}
            Type::SubStrList(_) => {}
            Type::BuildInfoType(_) => {}
            Type::UdtSrcLineType(_) => {}
        }
    }
}

#[test]
fn test_new_type_structures() {
    // Verify new documented types are accessible and have correct fields

    // Alias type
    let _alias = ezpdb::type_info::Alias {
        name: String::from("MyInt"),
        underlying_type: Rc::new(RefCell::new(Type::Primitive(ezpdb::type_info::Primitive {
            kind: ezpdb::type_info::PrimitiveKind::I32,
            indirection: None,
        }))),
    };

    // VTableShape type
    let _vtshape = ezpdb::type_info::VTableShape {
        count: 1,
        descriptors: vec![0x00],
    };

    // VFTableType
    let dummy_type: Rc<RefCell<Type>> =
        Rc::new(RefCell::new(Type::Primitive(ezpdb::type_info::Primitive {
            kind: ezpdb::type_info::PrimitiveKind::Void,
            indirection: None,
        })));

    let _vftable = ezpdb::type_info::VFTableType {
        root: Rc::clone(&dummy_type),
        path: Rc::clone(&dummy_type),
        offset: 0,
        segment: 0,
    };

    // FuncIdType
    let _func_id = ezpdb::type_info::FuncIdType {
        name: String::from("MyFunction"),
        function_type: Some(Rc::clone(&dummy_type)),
        parent_scope: None,
    };

    // MFuncIdType
    let _mfunc_id = ezpdb::type_info::MFuncIdType {
        name: String::from("MyMethod"),
        function_type: Some(Rc::clone(&dummy_type)),
        parent_type: Some(Rc::clone(&dummy_type)),
    };

    // StringIdType
    let _string_id = ezpdb::type_info::StringIdType {
        id: String::from("source.cpp"),
        substring: None,
    };

    // SubStrListType
    let _substr_list = ezpdb::type_info::SubStrListType {
        strings: vec![Rc::clone(&dummy_type)],
    };

    // BuildInfoTypeData
    let _build_info = ezpdb::type_info::BuildInfoTypeData {
        items: vec![Rc::clone(&dummy_type)],
    };

    // UdtSrcLineType
    let _udt_src = ezpdb::type_info::UdtSrcLineType {
        source_file: Some(Rc::clone(&dummy_type)),
        line_number: 42,
        udt: Rc::clone(&dummy_type),
    };
}

#[test]
fn test_error_types() {
    // Verify error types are accessible as documented
    use ezpdb::error::Error;

    // Test that error variants are accessible
    let _err1 = Error::MissingDependency("test");
    let _err2 = Error::Unsupported("test");
    let _err3 = Error::NeedForwardReferenceImplementation;
    let _err4 = Error::UnhandledType(String::from("test"));
    let _err5 = Error::UnresolvedType(0);

    // Verify error can be formatted (implements Display via thiserror)
    let err = Error::Unsupported("feature");
    let _msg = format!("{}", err);
}

#[test]
fn test_symbol_types() {
    // Verify symbol types mentioned in docs are accessible
    use ezpdb::symbol_types::*;

    // These types should all be accessible
    let _: Option<MachineType> = None;
    let _: Option<Version> = None;
    let _: Option<AssemblyInfo> = None;
    let _: Option<BuildInfo> = None;
    let _: Option<PublicSymbol> = None;
    let _: Option<Data> = None;
    let _: Option<Procedure> = None;
    let _: Option<DebugModule> = None;
}

#[test]
fn test_typed_trait() {
    // Verify Typed trait works as documented
    use ezpdb::symbol_types::ParsedPdb;
    use ezpdb::type_info::Typed;
    use std::path::PathBuf;

    let pdb = ParsedPdb::new(PathBuf::from("test.pdb"));

    let prim = ezpdb::type_info::Primitive {
        kind: ezpdb::type_info::PrimitiveKind::I32,
        indirection: None,
    };

    // type_size() should be callable
    let _size = prim.type_size(&pdb);
}

#[test]
fn test_parsed_pdb_fields() {
    // Verify ParsedPdb has all documented fields accessible
    use ezpdb::symbol_types::ParsedPdb;
    use std::path::PathBuf;

    let pdb = ParsedPdb::new(PathBuf::from("test.pdb"));

    // All these fields should be accessible as shown in examples
    let _ = &pdb.path;
    let _ = &pdb.assembly_info;
    let _ = &pdb.public_symbols;
    let _ = &pdb.types;
    let _ = &pdb.ipi_types;
    let _ = &pdb.procedures;
    let _ = &pdb.global_data;
    let _ = &pdb.debug_modules;
    let _ = &pdb.version;
    let _ = pdb.guid;
    let _ = pdb.age;
    let _ = pdb.timestamp;
    let _ = &pdb.machine_type;
}

#[test]
#[ignore] // Requires a real PDB file to exist
fn test_base_address_public_symbols() {
    // Test that base_address is correctly added to symbol offsets
    // This verifies the fix for the bug where base_address was ignored

    let pdb_path = "../pdb-compare/cache_test_pdbs/ntdll.pdb";

    // Parse without base address
    let pdb_no_base = match parse_pdb(pdb_path, None) {
        Ok(pdb) => pdb,
        Err(_) => return, // Skip if file doesn't exist
    };

    // Parse with base address of 0x140000000 (typical Windows DLL base)
    let base_addr = 0x140000000;
    let pdb_with_base = parse_pdb(pdb_path, Some(base_addr)).expect("Failed to parse PDB");

    // Verify we have the same number of symbols
    assert_eq!(
        pdb_no_base.public_symbols.len(),
        pdb_with_base.public_symbols.len(),
        "Symbol count should be identical"
    );

    // Find a public symbol with an rva in the no-base version
    let symbol_with_rva = pdb_no_base
        .public_symbols
        .iter()
        .find(|s| s.rva.is_some())
        .expect("Should have at least one symbol with rva");

    // Find the same symbol in the with-base version
    let symbol_name = &symbol_with_rva.name;
    let symbol_with_base = pdb_with_base
        .public_symbols
        .iter()
        .find(|s| &s.name == symbol_name)
        .expect("Symbol should exist in both versions");

    // Verify section and section_offset are identical
    assert_eq!(
        symbol_with_rva.section, symbol_with_base.section,
        "Symbol '{}': sections should be identical",
        symbol_name
    );
    assert_eq!(
        symbol_with_rva.section_offset, symbol_with_base.section_offset,
        "Symbol '{}': section_offsets should be identical",
        symbol_name
    );

    // Verify RVAs are identical (base_address doesn't affect RVA)
    assert_eq!(
        symbol_with_rva.rva, symbol_with_base.rva,
        "Symbol '{}': RVAs should be identical",
        symbol_name
    );

    // Verify the address is RVA + base_address
    if let (Some(rva_no_base), Some(addr_with_base)) =
        (symbol_with_rva.rva, symbol_with_base.address)
    {
        assert_eq!(
            addr_with_base,
            rva_no_base + base_addr,
            "Symbol '{}': address with base ({:#x}) should equal rva ({:#x}) + base_address ({:#x})",
            symbol_name,
            addr_with_base,
            rva_no_base,
            base_addr
        );
    } else {
        panic!("Symbol should have rva and address with base");
    }

    // Verify the no-base version has no address field set
    assert_eq!(
        symbol_with_rva.address, None,
        "Symbol '{}': address should be None when no base_address provided",
        symbol_name
    );
}

#[test]
#[ignore] // Requires a real PDB file to exist
fn test_base_address_procedures() {
    // Test that base_address is correctly added to procedure addresses

    let pdb_path = "../pdb-compare/cache_test_pdbs/ntdll.pdb";

    // Parse without base address
    let pdb_no_base = match parse_pdb(pdb_path, None) {
        Ok(pdb) => pdb,
        Err(_) => return, // Skip if file doesn't exist
    };

    // Parse with base address
    let base_addr = 0x180000000;
    let pdb_with_base = parse_pdb(pdb_path, Some(base_addr)).expect("Failed to parse PDB");

    // Verify we have the same number of procedures
    assert_eq!(
        pdb_no_base.procedures.len(),
        pdb_with_base.procedures.len(),
        "Procedure count should be identical"
    );

    // Find a procedure with an rva in the no-base version
    let proc_with_rva = pdb_no_base
        .procedures
        .iter()
        .find(|p| p.rva.is_some())
        .expect("Should have at least one procedure with rva");

    // Find the same procedure in the with-base version
    let proc_name = &proc_with_rva.name;
    let proc_with_base = pdb_with_base
        .procedures
        .iter()
        .find(|p| &p.name == proc_name)
        .expect("Procedure should exist in both versions");

    // Verify section and section_offset are identical
    assert_eq!(
        proc_with_rva.section, proc_with_base.section,
        "Procedure '{}': sections should be identical",
        proc_name
    );
    assert_eq!(
        proc_with_rva.section_offset, proc_with_base.section_offset,
        "Procedure '{}': section_offsets should be identical",
        proc_name
    );

    // Verify RVAs are identical (base_address doesn't affect RVA)
    assert_eq!(
        proc_with_rva.rva, proc_with_base.rva,
        "Procedure '{}': RVAs should be identical",
        proc_name
    );

    // Verify the address is RVA + base_address
    if let (Some(rva_no_base), Some(addr_with_base)) = (proc_with_rva.rva, proc_with_base.address) {
        assert_eq!(
            addr_with_base,
            rva_no_base + base_addr,
            "Procedure '{}': address with base ({:#x}) should equal rva ({:#x}) + base_address ({:#x})",
            proc_name,
            addr_with_base,
            rva_no_base,
            base_addr
        );
    } else {
        panic!("Procedure should have rva and address with base");
    }

    // Verify the no-base version has no address field set
    assert_eq!(
        proc_with_rva.address, None,
        "Procedure '{}': address should be None when no base_address provided",
        proc_name
    );
}

#[test]
#[ignore] // Requires a real PDB file to exist
fn test_base_address_data_symbols() {
    // Test that base_address is correctly added to data symbol offsets

    let pdb_path = "../pdb-compare/cache_test_pdbs/ntdll.pdb";

    // Parse without base address
    let pdb_no_base = match parse_pdb(pdb_path, None) {
        Ok(pdb) => pdb,
        Err(_) => return, // Skip if file doesn't exist
    };

    // Parse with base address
    let base_addr = 0x7FF000000000;
    let pdb_with_base = parse_pdb(pdb_path, Some(base_addr)).expect("Failed to parse PDB");

    // Verify we have the same number of data symbols
    assert_eq!(
        pdb_no_base.global_data.len(),
        pdb_with_base.global_data.len(),
        "Data symbol count should be identical"
    );

    // Find a data symbol with an rva in the no-base version
    let data_with_rva = pdb_no_base
        .global_data
        .iter()
        .find(|d| d.rva.is_some())
        .expect("Should have at least one data symbol with rva");

    // Find the same data symbol in the with-base version
    let data_name = &data_with_rva.name;
    let data_with_base = pdb_with_base
        .global_data
        .iter()
        .find(|d| &d.name == data_name)
        .expect("Data symbol should exist in both versions");

    // Verify section and section_offset are identical
    assert_eq!(
        data_with_rva.section, data_with_base.section,
        "Data '{}': sections should be identical",
        data_name
    );
    assert_eq!(
        data_with_rva.section_offset, data_with_base.section_offset,
        "Data '{}': section_offsets should be identical",
        data_name
    );

    // Verify RVAs are identical (base_address doesn't affect RVA)
    assert_eq!(
        data_with_rva.rva, data_with_base.rva,
        "Data '{}': RVAs should be identical",
        data_name
    );

    // Verify the address is RVA + base_address
    if let (Some(rva_no_base), Some(addr_with_base)) = (data_with_rva.rva, data_with_base.address) {
        assert_eq!(
            addr_with_base,
            rva_no_base + base_addr,
            "Data '{}': address with base ({:#x}) should equal rva ({:#x}) + base_address ({:#x})",
            data_name,
            addr_with_base,
            rva_no_base,
            base_addr
        );
    } else {
        panic!("Data symbol should have rva and address with base");
    }

    // Verify the no-base version has no address field set
    assert_eq!(
        data_with_rva.address, None,
        "Data '{}': address should be None when no base_address provided",
        data_name
    );
}
