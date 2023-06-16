use clap::{arg, Parser};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    #[arg(short, long, env)]
    pub database_url: String,

    #[arg(short, long, env)]
    pub input_folder: String,

    #[arg(short, long, env)]
    pub output_folder: String,

    #[arg(short, long, env)]
    pub scanner_threads: usize,

    #[arg(long, env)]
    pub image_converter_threads: usize,

    #[arg(long, env)]
    pub video_converter_threads: usize,

    #[arg(long, env)]
    pub seconds_between_file_scans: u64,

    #[arg(long, env)]
    pub seconds_between_processor_runs: u64,

    #[arg(long, env)]
    pub enable_thumbnail_preset: bool,

    #[arg(long, env)]
    pub enable_preview_preset: bool,
}
