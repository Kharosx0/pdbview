#!/bin/bash

# Comprehensive TPI/IPI Type Parsing Test Script
# Tests all PDBs in cache_test_pdbs folder and validates parsing completeness

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PDB_DIR="$SCRIPT_DIR/cache_test_pdbs"
CARGO_DIR="$SCRIPT_DIR/../.."

echo "=========================================="
echo "TPI/IPI Type Parsing Validation Test"
echo "=========================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check if PDB directory exists
if [ ! -d "$PDB_DIR" ]; then
    echo -e "${RED}Error: PDB directory not found: $PDB_DIR${NC}"
    exit 1
fi

# Find all PDB files
PDB_FILES=($(find "$PDB_DIR" -name "*.pdb" -type f))

if [ ${#PDB_FILES[@]} -eq 0 ]; then
    echo -e "${RED}Error: No PDB files found in $PDB_DIR${NC}"
    exit 1
fi

echo -e "${BLUE}Found ${#PDB_FILES[@]} PDB file(s) to test${NC}"
echo ""

# Build the project first
echo -e "${YELLOW}Building pdbview...${NC}"
cd "$CARGO_DIR"
cargo build --release 2>&1 | tail -5
echo -e "${GREEN}Build complete${NC}"
echo ""

TOTAL_ERRORS=0
TOTAL_WARNINGS=0
TOTAL_TYPES=0
TOTAL_PDBS=0

# Test each PDB file
for pdb_file in "${PDB_FILES[@]}"; do
    TOTAL_PDBS=$((TOTAL_PDBS + 1))
    pdb_name=$(basename "$pdb_file")

    echo "=========================================="
    echo -e "${BLUE}Testing: $pdb_name${NC}"
    echo "=========================================="

    # Run pdbview and capture output
    OUTPUT=$(cargo run --release --bin pdbview -- "$pdb_file" 2>&1)

    # Count types parsed
    TPI_COUNT=$(echo "$OUTPUT" | grep -i "TPI stream" | grep -oP '\d+ types?' | grep -oP '^\d+' || echo "0")
    IPI_COUNT=$(echo "$OUTPUT" | grep -i "IPI stream" | grep -oP '\d+ types?' | grep -oP '^\d+' || echo "0")

    # Check for errors
    ERROR_COUNT=$(echo "$OUTPUT" | grep -c "Error" || echo "0")
    WARNING_COUNT=$(echo "$OUTPUT" | grep -c "Warning" || echo "0")

    # Check for specific type indicators
    ALIAS_COUNT=$(echo "$OUTPUT" | grep -c "Alias" || echo "0")
    VTSHAPE_COUNT=$(echo "$OUTPUT" | grep -c "VTableShape" || echo "0")
    VFTABLE_COUNT=$(echo "$OUTPUT" | grep -c "VFTable" || echo "0")

    echo ""
    echo -e "${GREEN}Statistics:${NC}"
    echo "  TPI Types: $TPI_COUNT"
    echo "  IPI Types: $IPI_COUNT"
    echo "  Total Types: $((TPI_COUNT + IPI_COUNT))"
    echo ""
    echo -e "${YELLOW}New Type Coverage:${NC}"
    echo "  Alias types: $ALIAS_COUNT"
    echo "  VTableShape types: $VTSHAPE_COUNT"
    echo "  VFTable types: $VFTABLE_COUNT"
    echo ""

    if [ $ERROR_COUNT -gt 0 ]; then
        echo -e "${RED}Errors found: $ERROR_COUNT${NC}"
        echo "$OUTPUT" | grep "Error" || true
        TOTAL_ERRORS=$((TOTAL_ERRORS + ERROR_COUNT))
    else
        echo -e "${GREEN}✓ No errors${NC}"
    fi

    if [ $WARNING_COUNT -gt 0 ]; then
        echo -e "${YELLOW}Warnings found: $WARNING_COUNT${NC}"
        TOTAL_WARNINGS=$((TOTAL_WARNINGS + WARNING_COUNT))
    else
        echo -e "${GREEN}✓ No warnings${NC}"
    fi

    TOTAL_TYPES=$((TOTAL_TYPES + TPI_COUNT + IPI_COUNT))

    echo ""

    # Save detailed output for inspection
    OUTPUT_FILE="$SCRIPT_DIR/test_output_$(basename "$pdb_file" .pdb).txt"
    echo "$OUTPUT" > "$OUTPUT_FILE"
    echo -e "${BLUE}Detailed output saved to: $(basename "$OUTPUT_FILE")${NC}"
    echo ""
done

# Final summary
echo "=========================================="
echo -e "${BLUE}FINAL TEST SUMMARY${NC}"
echo "=========================================="
echo ""
echo "PDBs tested: $TOTAL_PDBS"
echo "Total types parsed: $TOTAL_TYPES"
echo "Total errors: $TOTAL_ERRORS"
echo "Total warnings: $TOTAL_WARNINGS"
echo ""

if [ $TOTAL_ERRORS -eq 0 ]; then
    echo -e "${GREEN}✓✓✓ ALL TESTS PASSED ✓✓✓${NC}"
    echo -e "${GREEN}All PDBs parsed successfully with no errors!${NC}"
    exit 0
else
    echo -e "${RED}✗✗✗ TESTS FAILED ✗✗✗${NC}"
    echo -e "${RED}Found $TOTAL_ERRORS error(s) across $TOTAL_PDBS PDB(s)${NC}"
    exit 1
fi
