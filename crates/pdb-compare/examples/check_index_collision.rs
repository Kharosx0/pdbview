/// Check if IPI type indices also exist in TPI (collision detection)
use anyhow::Result;
use ms_pdb::codeview::types::TypeIndex;
use ms_pdb::Pdb;
use std::fs::File;
use std::path::Path;

fn main() -> Result<()> {
    let pdb_path = std::env::args()
        .nth(1)
        .expect("Usage: check_index_collision <pdb_path>");

    println!("═══════════════════════════════════════════════════════════════");
    println!("      CHECKING FOR TPI/IPI INDEX COLLISIONS");
    println!("═══════════════════════════════════════════════════════════════\n");
    println!("PDB File: {}\n", pdb_path);

    // Open PDB
    let file = File::open(&pdb_path)?;
    let pdb = Pdb::open_from_file(file)?;

    // Get ezpdb parsing too
    let parsed_pdb = ezpdb::parse_pdb(Path::new(&pdb_path), None)?;

    // Get type streams
    let type_stream = pdb.read_type_stream()?;
    let ipi_stream = pdb.read_ipi_stream()?;

    println!("📊 STREAM INFO:");
    println!(
        "   TPI type_index_begin: 0x{:08x}",
        type_stream.type_index_begin().0
    );
    println!(
        "   TPI type_index_end:   0x{:08x}",
        type_stream.type_index_end().0
    );
    println!(
        "   TPI count: {}",
        type_stream.type_index_end().0 - type_stream.type_index_begin().0
    );
    println!();
    println!(
        "   IPI type_index_begin: 0x{:08x}",
        ipi_stream.type_index_begin().0
    );
    println!(
        "   IPI type_index_end:   0x{:08x}",
        ipi_stream.type_index_end().0
    );
    println!(
        "   IPI count: {}",
        ipi_stream.type_index_end().0 - ipi_stream.type_index_begin().0
    );

    println!("\n🔍 ANALYZING IPI DATA SYMBOLS FOR COLLISIONS...\n");

    let mut collision_count = 0;
    let mut ipi_only_count = 0;

    for data_symbol in parsed_pdb.global_data.iter() {
        let ty = data_symbol.ty.borrow();
        let type_name = format!("{:?}", ty);

        // Check if this is an IPI type
        let is_ipi = type_name.contains("FuncId")
            || type_name.contains("MFuncId")
            || type_name.contains("StringId")
            || type_name.contains("SubStrList")
            || type_name.contains("BuildInfo")
            || type_name.contains("UdtSrcLine");

        if !is_ipi {
            continue;
        }

        // Try to extract the type index from the debug string
        // Look for patterns like "type_index: TypeIndex(4196)"
        let type_idx_opt: Option<u32> = if let Some(pos) = type_name.find("TypeIndex(") {
            let rest = &type_name[pos + 10..];
            if let Some(end) = rest.find(')') {
                rest[..end].parse().ok()
            } else {
                None
            }
        } else {
            None
        };

        if let Some(type_idx) = type_idx_opt {
            // Check if this index exists in both streams
            let in_tpi = type_idx >= type_stream.type_index_begin().0
                && type_idx < type_stream.type_index_end().0;
            let in_ipi = type_idx >= ipi_stream.type_index_begin().0
                && type_idx < ipi_stream.type_index_end().0;

            if in_tpi && in_ipi {
                collision_count += 1;

                println!("═══ COLLISION DETECTED ═══");
                println!("Symbol: {}", data_symbol.name);
                println!("Type Index: 0x{:08x} ({})", type_idx, type_idx);
                println!("   ⚠️  This index exists in BOTH TPI and IPI!");

                // Get what's at this index in TPI
                if let Ok(tpi_type) = type_stream.record(TypeIndex(type_idx)) {
                    let tpi_parsed = tpi_type.parse()?;
                    println!("\n   TPI contains: {:?}", tpi_type.kind);
                    let tpi_debug = format!("{:?}", tpi_parsed);
                    if let Some(first_line) = tpi_debug.lines().next() {
                        println!("   TPI data: {}", &first_line[..first_line.len().min(70)]);
                    }
                }

                // Get what's at this index in IPI
                if let Ok(ipi_type) = ipi_stream.record(TypeIndex(type_idx)) {
                    let ipi_parsed = ipi_type.parse()?;
                    println!("\n   IPI contains: {:?}", ipi_type.kind);
                    let ipi_debug = format!("{:?}", ipi_parsed);
                    if let Some(first_line) = ipi_debug.lines().next() {
                        println!("   IPI data: {}", &first_line[..first_line.len().min(70)]);
                    }
                }

                println!();
            } else if in_ipi {
                ipi_only_count += 1;
            }
        }
    }

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("                        RESULTS");
    println!("═══════════════════════════════════════════════════════════════\n");
    println!(
        "IPI data symbols with COLLIDING indices (exist in both TPI & IPI): {}",
        collision_count
    );
    println!("IPI data symbols with IPI-only indices: {}", ipi_only_count);

    if collision_count > 0 {
        println!("\n🚨 CRITICAL: Index collision detected!");
        println!("   These type indices exist in BOTH streams with different data.");
        println!("   This explains why the bug occurs - ezpdb is likely picking the");
        println!("   wrong stream (IPI instead of TPI) for these indices.");
    } else {
        println!("\n✅ No collisions: All IPI indices are IPI-only");
        println!("   This is a different kind of bug - the type indices in the");
        println!("   symbol records genuinely only exist in IPI, not TPI.");
    }

    println!("\n═══════════════════════════════════════════════════════════════\n");

    Ok(())
}
