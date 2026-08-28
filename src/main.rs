use clap::{Parser, ValueEnum};
use colored::Colorize;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use vuio_media_info::{MediaInfo, MediaReport, OutputFormat};

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum CliOutputFormat {
    Text,
    Json,
    Xml,
    Csv,
    Html,
}

impl From<CliOutputFormat> for OutputFormat {
    fn from(f: CliOutputFormat) -> Self {
        match f {
            CliOutputFormat::Text => OutputFormat::Text,
            CliOutputFormat::Json => OutputFormat::Json,
            CliOutputFormat::Xml => OutputFormat::Xml,
            CliOutputFormat::Csv => OutputFormat::Csv,
            CliOutputFormat::Html => OutputFormat::Html,
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "mediainfo",
    author = "MediaInfo Rust Contributors",
    version = env!("CARGO_PKG_VERSION"),
    about = "Pure Rust rewrite of MediaInfoLib: exhaustive media metadata inspection and reporting",
    long_about = "A high-performance, pure Rust media metadata analyzer. Inspects container headers, video bitstreams (AVC, HEVC, AV1, VP9, ProRes), audio bitstreams (AAC, AC-3, DTS, FLAC, MP3, Opus), and embedded tags."
)]
struct Args {
    /// Files or directories to inspect
    #[arg(required = true)]
    files: Vec<PathBuf>,

    /// Output format: TEXT (default), JSON, XML, CSV, HTML
    #[arg(short = 'O', long = "output", value_enum, default_value = "text")]
    output: CliOutputFormat,

    /// Recursively scan directories
    #[arg(short = 'r', long = "recursive")]
    recursive: bool,

    /// Enable parallel processing across all CPU cores
    #[arg(short = 'p', long = "parallel", default_value_t = true)]
    parallel: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let format: OutputFormat = args.output.into();

    let mut target_files = Vec::new();
    for file_path in args.files {
        if file_path.is_dir() {
            collect_files(&file_path, args.recursive, &mut target_files)?;
        } else if file_path.exists() {
            target_files.push(file_path);
        } else {
            eprintln!(
                "{}: File '{}' not found",
                "Error".red().bold(),
                file_path.display()
            );
        }
    }

    if target_files.is_empty() {
        eprintln!("{}: No files to analyze", "Warning".yellow().bold());
        return Ok(());
    }

    if target_files.len() == 1 {
        // Single file processing
        let path = &target_files[0];
        match MediaInfo::open_path(path) {
            Ok(report) => {
                let output_str = format.format(&report)?;
                println!("{}", output_str);
            }
            Err(e) => {
                eprintln!(
                    "{}: Failed to analyze '{}': {}",
                    "Error".red().bold(),
                    path.display(),
                    e
                );
            }
        }
    } else {
        // Multi-file batch processing
        if args.output == CliOutputFormat::Csv {
            println!(
                "File,Format,FileSize,Duration_ms,Video_Codec,Resolution,FrameRate,Audio_Codec,Audio_Channels,Audio_SamplingRate"
            );
        }

        let process_file = |path: &PathBuf| -> Option<(PathBuf, MediaReport)> {
            match MediaInfo::open_path(path) {
                Ok(report) => Some((path.clone(), report)),
                Err(e) => {
                    eprintln!(
                        "{}: Failed to analyze '{}': {}",
                        "Error".red().bold(),
                        path.display(),
                        e
                    );
                    None
                }
            }
        };

        let results: Vec<_> = if args.parallel {
            target_files.par_iter().filter_map(process_file).collect()
        } else {
            target_files.iter().filter_map(process_file).collect()
        };

        for (_path, report) in results {
            if args.output == CliOutputFormat::Csv {
                if let Ok(csv_line) = format.format(&report) {
                    // Skip header line in batch CSV
                    for line in csv_line.lines().skip(1) {
                        println!("{}", line);
                    }
                }
            } else {
                if let Ok(formatted) = format.format(&report) {
                    println!("{}", formatted);
                }
            }
        }
    }

    Ok(())
}

fn collect_files(dir: &Path, recursive: bool, list: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if recursive {
                    collect_files(&path, true, list)?;
                }
            } else {
                list.push(path);
            }
        }
    }
    Ok(())
}
