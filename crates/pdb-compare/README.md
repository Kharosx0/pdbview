# pdb-compare

A comprehensive testing tool that compares PDB parsing between the old `ezpdb` implementation (using `pdb` crate v0.8) and the new `ezpdb` implementation (using `ms-pdb`).

## Purpose

This tool validates that the migration from the old `pdb` crate to the new `ms-pdb` based implementation produces identical results. It parses PDB files with both ezpdb implementations and performs detailed comparisons across:

- **Header Information**: Age, GUID
- **Type Information**: All types from TPI stream (classes, unions, enums, pointers, etc.)
  - **Field Comparison**: Verifies struct/union fields match (count, names, offsets)
  - **Variant Comparison**: Verifies enum variants match (count, names, values)
  - **Size Comparison**: Verifies type sizes match between parsers
  - **Base Class Comparison**: Verifies inheritance information matches
- **Symbol Information**: Public symbols, procedures, data symbols
- **Module Information**: Debug modules and their object files</parameter>

## Quick Start

### Run Integration Tests

The fastest way to validate the comparison tool is to run the comprehensive integration tests:

```bash
cd crates/pdb-compare
cargo test --test integration_tests -- --nocapture
```

This will test against all PDB files in the `cache_test_pdbs/` directory and report:
- Type size mismatches
- Missing or extra struct fields
- Missing or extra enum variants
- Issues with critical kernel types (_EPROCESS, _KPROCESS, etc.)

### Basic CLI Usage

```bash
cargo run -- --pdb-file path/to/your.pdb
```</parameter>
```

### Options

- `-p, --pdb-file <PATH>`: Path to the PDB file to analyze (can specify multiple times)
- `-o, --output <PATH>`: Write JSON comparison results to file
- `-v, --verbose`: Enable verbose logging (debug level)
- `-c, --categories <CATEGORIES>`: Only run specific test categories (comma-separated: header,types,symbols,modules)
- `--cache-test`: Run cache test with all available test PDBs from the cache directory

### Examples

**Compare all aspects of a PDB:**
```bash
cargo run -- -p C:\Windows\System32\ntdll.pdb
```

**Compare multiple PDB files:**
```bash
cargo run -- -p ntdll.pdb -p kernel32.pdb -p user32.pdb
```

**Save results to JSON:**
```bash
cargo run -- -p ntdll.pdb -o results.json
```

**Only compare specific categories:**
```bash
cargo run -- -p ntdll.pdb -c header,types
```

**Enable verbose logging:**
```bash
cargo run -- -p ntdll.pdb -v
```

**Run cache test with all available test PDBs:**
```bash
# First, copy your test PDBs to the cache_test_pdbs directory
cp /path/to/*.pdb cache_test_pdbs/

# Then run the cache test
cargo run -- --cache-test
```

## Output

The tool provides:

1. **Summary Statistics** (per PDB file):
   - Total differences found
   - Whether header matches
   - Whether type counts match
   - Whether symbol counts match
   - Whether module counts match

2. **Detailed Differences**: For each difference found, shows:
   - Category (header, types, symbols, modules)
   - Description of the mismatch
   - Old value (from `pdb` crate)
   - New value (from `ezpdb`)

3. **Multi-file Summary** (when processing multiple files):
   - Total files processed
   - Files with differences vs files matching perfectly
   - Overall success/failure status

4. **Exit Code**:
   - `0`: No differences (implementations match perfectly for all files)
   - `1`: Differences found in one or more files

## JSON Output Format

When using `-o/--output`, the tool writes a JSON file with:

```json
{
  "summary": {
    "total_differences": 0,
    "header_matches": true,
    "type_count_matches": true,
    "symbol_count_matches": true,
    "module_count_matches": true
  },
  "differences": [
    {
      "category": "types",
      "description": "Type 1234 kind mismatch",
      "old_value": "Class",
      "new_value": "Union"
    }
  ]
}
```

## Testing Strategy

The tool validates:

### Header Validation
- PDB age matches
- GUID matches

### Type Validation
- Same number of types parsed
- For each type:
  - Same type index
  - Same type kind (Class, Union, Enum, Pointer, etc.)
  - Same name (if applicable)
  - Same size (if applicable)
  - **Same field count** (for structs/unions)
  - **Same field names and offsets** (for structs/unions)
  - **Same variant count** (for enums)
  - **Same variant names and values** (for enums)
  - **Same base class count and offsets** (for classes)</parameter>
  - Same size (if applicable)

### Symbol Validation
- Same number of public symbols, procedures, and data symbols
- For each symbol:
  - Same name
  - Same offset/RVA (if applicable)
  - Same flags (is_function, etc.)
  - Same length (for procedures)

### Module Validation
- Same number of modules
- For each module:
  - Same module name
  - Same object file name

## Integration

This tool is part of the `ezpdb` workspace. To run from workspace root:

```bash
cargo run -p pdb-compare -- -p path/to/file.pdb
```

## Development

The tool is organized into modules:

- `main.rs`: CLI argument parsing, main flow, and multi-file processing
- `lib.rs`: Library interface exposing modules for integration tests
- `old_pdb.rs`: Parser using old `ezpdb` from GitHub (landaire/pdbview master branch, uses `pdb` crate v0.8)
  - Extracts type information including fields, variants, and base classes
- `new_pdb.rs`: Parser using new `ezpdb` from local workspace (uses `ms-pdb` crate)
  - Extracts type information including fields, variants, and base classes
- `compare.rs`: Comparison logic and difference reporting
  - Compares field counts and names
  - Compares enum variant counts and values
  - Compares base class information
- `tests/integration_tests.rs`: Comprehensive automated tests</parameter>

### Architecture

The comparison now uses the same `ezpdb` API for both old and new parsers:
- **Old baseline**: `ezpdb-old` (aliased package from https://github.com/landaire/pdbview master branch)
- **New implementation**: `ezpdb` (local workspace package using `ms-pdb`)

This approach compares the same high-level API rather than wrapping two different low-level crates, making the comparison more meaningful and easier to maintain.

### Adding Examples

The `examples/` directory contains utilities:
- `check_sections.rs`: Analyzes section headers and offset calculations
- `explore_dbi.rs`: Explores DBI stream information and module data
- Various diagnostic tools for investigating parsing issues

See `ENHANCED_COMPARISON_SUMMARY.md` for detailed information about:
- What comparisons are performed
- Issues found during testing
- Recommendations for fixes
- Test result summaries</parameter>

## Known Limitations

Some minor differences are expected due to implementation details and architectural improvements:

1. **Type Representation**: The two implementations may represent certain complex types differently due to underlying parser differences
2. **Primitive Types**: New `ms-pdb` based `ezpdb` provides more detailed primitive type decoding
3. **IPI Stream**: New implementation parses additional IPI types (FuncId, BuildInfo, etc.) that may not be available in the old version
4. **Symbol Categorization**: 
   - **Old `ezpdb`** (pdb crate v0.8): Parses symbols using the `pdb` crate's `global_symbols()` API, which has known quirks like duplicating some non-function public symbols as data symbols
   - **New `ezpdb`** (ms-pdb): Parses comprehensive symbol streams (global + module streams) with proper separation between:
     - **Public symbols** (S_PUB32): Minimal linking symbols without type information
     - **Data symbols** (S_GDATA32): Rich symbols with type indices and full type information  
     - **Procedures**: Function symbols from module streams
5. **Offset Calculation**: New implementation has improved section-relative offset handling
6. **Module Information**: Old `ezpdb` has private fields for DebugModule, so detailed module comparison is limited
7. **Expected Discrepancies**: When comparing ntdll.pdb:
   - Old parser: ~995 data symbols (includes 8 duplicated non-function public symbols), ~4800-4900 procedures, ~6946 public symbols
   - New parser: ~987 data symbols (correct, no duplication), ~4844 procedures, ~6946 public symbols

## Regression Detection

This tool is designed to catch regressions in the new ezpdb implementation. The integration tests will:

- ✅ **Pass** if the new parser finds all data the old parser found
- ⚠️ **Warn** if there are minor differences that don't affect functionality
- ❌ **Fail** if the new parser is missing data (fields, variants, types) that the old parser found

This ensures the new implementation is a **strict improvement** over the old one.

### Known Issues Detected

As of the latest test run, the following issues have been identified:

1. **Missing Struct Fields**: Some structs are missing fields in the new parser
   - Example: `_EPROCESS` has 261 fields in old parser, only 163 in new parser
   - Example: `_KHETERO_STATE` has 4 fields in old parser, only 1 in new parser

2. **Root Cause**: Possible issues with field list continuation chain parsing or field type filtering

See `ENHANCED_COMPARISON_SUMMARY.md` for detailed analysis and recommendations.

## Contributing

When adding new features to `ezpdb`, update this tool to validate:

1. Add new fields to comparison structures in `old_pdb.rs` and `new_pdb.rs`
2. Add comparison logic in `compare.rs`
3. Add integration tests in `tests/integration_tests.rs`
4. Test against various PDB files to ensure compatibility
5. Ensure all tests pass before merging changes</parameter>
