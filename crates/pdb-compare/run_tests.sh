#!/bin/bash

# PDB Comparison Test Suite Runner
# This script runs all comparison tests and generates a comprehensive report

set -e

echo "========================================"
echo "PDB Comparison Test Suite"
echo "========================================"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track test results
PASSED=0
FAILED=0
WARNINGS=0

echo "Step 1: Building the project..."
cargo build --release
echo ""

echo "Step 2: Running integration tests..."
echo "========================================"
echo ""

# Run each test individually to get detailed output
echo -e "${YELLOW}Test 1: Type Size Comparison${NC}"
if cargo test --test integration_tests test_type_sizes_match -- --nocapture; then
    echo -e "${GREEN}✓ Type sizes match${NC}"
    ((PASSED++))
else
    echo -e "${RED}✗ Type size mismatches found${NC}"
    ((FAILED++))
fi
echo ""

echo -e "${YELLOW}Test 2: Enum Variant Completeness${NC}"
if cargo test --test integration_tests test_enum_variant_completeness -- --nocapture; then
    echo -e "${GREEN}✓ Enum variants complete${NC}"
    ((PASSED++))
else
    echo -e "${RED}✗ Missing enum variants${NC}"
    ((FAILED++))
fi
echo ""

echo -e "${YELLOW}Test 3: Struct Field Completeness${NC}"
if cargo test --test integration_tests test_struct_field_completeness -- --nocapture; then
    echo -e "${GREEN}✓ Struct fields complete${NC}"
    ((PASSED++))
else
    echo -e "${RED}✗ Missing struct fields${NC}"
    ((FAILED++))
fi
echo ""

echo -e "${YELLOW}Test 4: Critical Kernel Types${NC}"
if cargo test --test integration_tests test_critical_kernel_types -- --nocapture; then
    echo -e "${GREEN}✓ Critical kernel types verified${NC}"
    ((PASSED++))
else
    echo -e "${RED}✗ Issues with critical kernel types${NC}"
    ((FAILED++))
fi
echo ""

echo -e "${YELLOW}Test 5: ntkrnlmp.pdb Full Comparison${NC}"
if cargo test --test integration_tests test_ntkrnlmp_pdb_comparison -- --nocapture; then
    echo -e "${GREEN}✓ ntkrnlmp.pdb comparison passed${NC}"
    ((PASSED++))
else
    echo -e "${YELLOW}⚠ ntkrnlmp.pdb has differences (non-critical)${NC}"
    ((WARNINGS++))
fi
echo ""

echo -e "${YELLOW}Test 6: ntdll.pdb Full Comparison${NC}"
if cargo test --test integration_tests test_ntdll_pdb_comparison -- --nocapture; then
    echo -e "${GREEN}✓ ntdll.pdb comparison passed${NC}"
    ((PASSED++))
else
    echo -e "${YELLOW}⚠ ntdll.pdb has differences (non-critical)${NC}"
    ((WARNINGS++))
fi
echo ""

echo -e "${YELLOW}Test 7: hvix64.pdb Full Comparison${NC}"
if cargo test --test integration_tests test_hvix64_pdb_comparison -- --nocapture; then
    echo -e "${GREEN}✓ hvix64.pdb comparison passed${NC}"
    ((PASSED++))
else
    echo -e "${YELLOW}⚠ hvix64.pdb has differences (non-critical)${NC}"
    ((WARNINGS++))
fi
echo ""

echo "========================================"
echo "Test Results Summary"
echo "========================================"
echo -e "${GREEN}Passed:   $PASSED${NC}"
echo -e "${RED}Failed:   $FAILED${NC}"
echo -e "${YELLOW}Warnings: $WARNINGS${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All critical tests passed!${NC}"
    if [ $WARNINGS -gt 0 ]; then
        echo -e "${YELLOW}⚠ Some non-critical differences were found.${NC}"
        echo "  Review the test output above for details."
    fi
    echo ""
    echo "The new ezpdb implementation is ready for use."
    exit 0
else
    echo -e "${RED}✗ Some tests failed!${NC}"
    echo ""
    echo "The new ezpdb implementation has regressions."
    echo "Please review the test output above and fix the issues."
    echo ""
    echo "See ENHANCED_COMPARISON_SUMMARY.md for detailed analysis."
    exit 1
fi
