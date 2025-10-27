use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;
use thiserror::Error;

mod output;

#[derive(Error, Debug)]
pub enum CliArgumentError {
    #[error("the value `{1}` is not valid for the parameter `{0}`")]
    InvalidValue(&'static str, String),
}

/// Parse base address from either decimal or hexadecimal (0x prefix) format
fn parse_base_address(s: &str) -> Result<usize, std::num::ParseIntError> {
    if let Some(hex_str) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        usize::from_str_radix(hex_str, 16)
    } else {
        s.parse::<usize>()
    }
}

#[derive(StructOpt, Debug)]
#[structopt(name = "pdbview")]
struct Opt {
    /// Print debug information
    #[structopt(short, long)]
    debug: bool,

    /// Output format type. Options include: plain, json
    #[structopt(short, long, default_value = "plain")]
    format: OutputFormatType,

    /// Base address of module in-memory. Accepts decimal or hex (0x prefix).
    /// Example: 0x140000000 or 5368709120
    /// If provided, computed addresses will be RVA + base_address
    #[structopt(short, long, parse(try_from_str = parse_base_address))]
    base_address: Option<usize>,

    /// Pretty-print JSON output (ignored for non-JSON formats)
    #[structopt(long)]
    pretty_json: bool,

    /// PDB file to process
    #[structopt(name = "FILE", parse(from_os_str))]
    file: PathBuf,
}

#[derive(Debug)]
enum OutputFormatType {
    Plain,
    Json,
}

impl FromStr for OutputFormatType {
    type Err = CliArgumentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let result = match s.to_ascii_lowercase().as_ref() {
            "plain" => OutputFormatType::Plain,
            "json" => OutputFormatType::Json,
            _ => return Err(CliArgumentError::InvalidValue("format", s.to_string())),
        };

        Ok(result)
    }
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();

    if opt.debug {
        simplelog::SimpleLogger::init(log::LevelFilter::Debug, simplelog::Config::default())?;
    }

    let parsed_pdb = ezpdb::parse_pdb(&opt.file, opt.base_address)?;
    let stdout = std::io::stdout();
    let mut stdout_lock = stdout.lock();

    match opt.format {
        OutputFormatType::Plain => output::print_plain(&mut stdout_lock, &parsed_pdb)?,
        OutputFormatType::Json => {
            output::print_json(&mut stdout_lock, &parsed_pdb, opt.pretty_json)?
        }
    }

    Ok(())
}
