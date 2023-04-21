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
}
