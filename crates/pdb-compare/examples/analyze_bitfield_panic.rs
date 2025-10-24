//! DEPRECATED: This diagnostic tool was used to debug bitfield panics that have since been fixed.
//! The TypeProperties API has changed significantly, and this example is no longer maintained.
//!
//! The original issue (bitfield panics in UdtProperties) has been resolved in the current codebase.
//! This file is kept for historical reference only.
//!
//! To exclude this from compilation, this file now contains only documentation.

// This example has been intentionally disabled because:
// 1. The bug it was designed to diagnose has been fixed
// 2. The ms-pdb TypeProperties API changed significantly
// 3. Updating it would require substantial work for little value
//
// If you need to debug similar issues in the future, refer to the git history
// for the original implementation, or use the current ms-pdb and ezpdb APIs directly.

fn main() {
    eprintln!("⚠️  DEPRECATED: This example is no longer maintained.");
    eprintln!("The bitfield panic issue it was designed to debug has been fixed.");
    eprintln!("This is a placeholder to prevent compilation errors.");
    std::process::exit(1);
}
