mod config_parser;
mod scans;

use env_logger::Env;

#[macro_use]
extern crate log;

use rayon::prelude::*;
use rusqlite::{Connection, Error};
use std::ffi::OsString;
use std::fmt;
use std::fmt::Formatter;
use std::path::PathBuf;

use clap::{arg, Parser};

use crate::config_parser::{Config, IterateOn, Scan, Script};
use crate::scans::{ContentReplacer, FileProperties, FolderProperties};
use subprocess::{Exec, ExitStatus, PopenError, Redirection};
use thiserror::Error;
use walkdir::{DirEntry, WalkDir};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, env)]
    config_folder_path: String,

    #[arg(short, long, env)]
    db_folder_path: String,

    #[arg(short, long, env, default_value = "./builtins")]
    builtins_folder_path: String,
}

type Log = String;
type Command = String;

#[derive(Error, Debug)]
pub enum FixMyLibErrors {
    #[error("Invalid config preset: {0}")]
    InvalidConfigPreset(String),

    #[error("Processing command exited with status code != 0: {0}")]
    ExitCodeFailure(Log, Command),

    #[error("Error initing DB: {0}")]
    DbInitFailure(String),

    #[error("Error starting subprocess: {0}")]
    SubprocessOpenFailure(String),

    #[error("Failure parsing path: {0}")]
    PathParsingFailure(String),
}

impl From<rusqlite::Error> for FixMyLibErrors {
    fn from(e: rusqlite::Error) -> Self {
        FixMyLibErrors::DbInitFailure(e.to_string())
    }
}

impl From<OsString> for FixMyLibErrors {
    fn from(e: OsString) -> Self {
        FixMyLibErrors::PathParsingFailure(e.into_string().unwrap())
    }
}

impl From<std::io::Error> for FixMyLibErrors {
    fn from(e: std::io::Error) -> Self {
        FixMyLibErrors::SubprocessOpenFailure(e.to_string())
    }
}

impl From<PopenError> for FixMyLibErrors {
    fn from(e: PopenError) -> Self {
        FixMyLibErrors::SubprocessOpenFailure(e.to_string())
    }
}

//
impl From<walkdir::Error> for FixMyLibErrors {
    fn from(e: walkdir::Error) -> Self {
        FixMyLibErrors::PathParsingFailure(e.to_string())
    }
}

struct Db {
    conn: Connection,
}

impl Db {
    fn new(db_dir: &str) -> Result<Db, FixMyLibErrors> {
        let path = format!("{db_dir}/fixmylib.db");
        let conn = Connection::open(path)?;
        conn.execute(
            "create table if not exists files
            (
                filepath text not null,
                script text not null,
                has_been_processed boolean not null,
                is_last_run_success boolean not null,
                last_run_log text not null,
                failed_command text,
                primary key(filepath,script)
            )",
            [],
        )?;
        Ok(Db { conn })
    }

    fn mark_as_processed(
        &self,
        filepath: &str,
        script: &str,
        log: &str,
    ) -> Result<(), FixMyLibErrors> {
        self.conn.execute(
            "INSERT INTO files (filepath, script, has_been_processed, is_last_run_success, last_run_log, failed_command) values (?1, ?2, ?3, ?4, ?5, ?6);",
            [filepath, script, "true", "true", log, ""],
        )?;
        debug!("Success marking {} as processed", &filepath);
        Ok(())
    }

    fn mark_as_failed(
        &self,
        filepath: &str,
        script: &str,
        log: &str,
        command: &str,
    ) -> Result<(), FixMyLibErrors> {
        self.conn.execute(
            "INSERT INTO files (filepath, script, has_been_processed, is_last_run_success, last_run_log, failed_command) values (?1, ?2, ?3, ?4, ?5, ?6);",
            [filepath, script, "true", "false", log, command],
        )?;
        debug!("Success marking {} as failed  ", &filepath);
        Ok(())
    }

    fn has_been_already_processed(&self, filepath: &str, script: &str) -> bool {
        let result: Result<Option<String>, Error> = self.conn.query_row(
            "select has_been_processed from files where filepath = :filepath and script = :script",
            &[(":filepath", filepath), (":script", script)],
            |row| row.get(0),
        );
        match result {
            Ok(Some(v)) if v == "true" => true,
            Err(Error::QueryReturnedNoRows) => false,
            Err(e) => {
                warn!("Found this error when trying to check if file {} has already been processed: {:?}", filepath, e);
                false
            }
            _ => false,
        }
    }
}

fn process_entry(entry: DirEntry, scan: &Scan, args: &Args) {
    match process_entry_result(&entry, scan, &scan.input_folder, &args.db_folder_path) {
        Ok(_) => {}
        Err(msg) => get_full_filepath(&entry).map_or_else(
            |_| error!("Could not process a file: {msg}"),
            |v| error!("Could not process file {v}: {msg}"),
        ),
    }
}

fn get_full_filepath(entry: &DirEntry) -> Result<String, FixMyLibErrors> {
    entry
        .path()
        .canonicalize()?
        .into_os_string()
        .into_string()
        .map_err(|_| {
            FixMyLibErrors::PathParsingFailure(format!("No filepath found for {:?}", entry.path()))
        })
}

enum EntryProperties {
    File(FileProperties),
    Folder(FolderProperties),
}

impl fmt::Display for EntryProperties {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                EntryProperties::File(v) => format!("{}", v),
                EntryProperties::Folder(v) => format!("{}", v),
            }
        )
    }
}

fn parse_entry(entry: &DirEntry, input_dir: &str) -> Result<EntryProperties, FixMyLibErrors> {
    if entry.metadata()?.is_file() {
        Ok(EntryProperties::File(parse_file_entry(entry, input_dir)?))
    } else {
        Ok(EntryProperties::Folder(parse_folder_entry(
            entry, input_dir,
        )?))
    }
}

impl fmt::Display for FolderProperties {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path)
    }
}

fn parse_folder_entry(
    entry: &DirEntry,
    input_dir: &str,
) -> Result<FolderProperties, FixMyLibErrors> {
    let full_filepath = get_full_filepath(entry)?;
    let path = full_filepath.replace(input_dir, ".");
    Ok(FolderProperties {
        full_path: full_filepath,
        path,
    })
}

fn parse_file_entry(entry: &DirEntry, input_dir: &str) -> Result<FileProperties, FixMyLibErrors> {
    let full_filepath = get_full_filepath(entry)?;
    let filestem = entry
        .path()
        .file_stem()
        .ok_or(FixMyLibErrors::PathParsingFailure(format!(
            "No filestem found for {full_filepath}"
        )))?
        .to_os_string()
        .into_string()?;
    let extension = entry
        .path()
        .extension()
        .ok_or(FixMyLibErrors::PathParsingFailure(format!(
            "No extension found for {full_filepath}"
        )))?
        .to_str()
        .ok_or(FixMyLibErrors::PathParsingFailure(format!(
            "Failure parsing extension for {full_filepath}"
        )))?
        .to_lowercase();

    let filename = entry
        .file_name()
        .to_str()
        .ok_or(FixMyLibErrors::PathParsingFailure(format!(
            "Failure parsing filename for {:?}",
            entry.path()
        )))?
        .to_string();
    let absolute_input_dir = PathBuf::from(input_dir)
        .canonicalize()?
        .into_os_string()
        .into_string()?;
    let filepath = full_filepath.replace(&absolute_input_dir, ".");
    let folder_path = filepath.replace(&filename, "");

    Ok(FileProperties {
        filestem,
        full_path: full_filepath,
        extension,
        path: filepath,
        folder_path,
    })
}

fn process_script_for_file(
    file_properties: FileProperties,
    script: &Script,
    db_dir: &str,
) -> Result<(), FixMyLibErrors> {
    if !script.extension_list.is_desired(&file_properties.extension) {
        debug!(
            "File extension <{}> is not desired for script {:?}, skipping it...",
            &file_properties.extension, script.extension_list
        );
        return Ok(());
    }
    process_script_for_both(file_properties, script, db_dir)
}

fn process_script_for_both(
    properties: impl ContentReplacer,
    script: &Script,
    db_dir: &str,
) -> Result<(), FixMyLibErrors> {
    let db = Db::new(db_dir)?;
    let id = properties.main_identifier();

    warn!("Processing file: {} for script {}", &id, script.name);
    if db.has_been_already_processed(&id, &script.name) {
        warn!(
            "File {}  has been been already processed by script {}",
            &properties.main_identifier(),
            &script.name
        );
        return Ok(());
    }

    match execute_script(properties, script) {
        Ok(log) => db.mark_as_processed(&id, &script.name, &log),
        Err(FixMyLibErrors::ExitCodeFailure(log, cmd)) => {
            db.mark_as_failed(&id, &script.name, &log, &cmd)
        }
        Err(e) => {
            error!("Something went wrong processing file {}: {e}", &id);
            Ok(())
        }
    }
}

fn process_entry_result(
    entry: &DirEntry,
    scan: &Scan,
    input_dir: &str,
    db_dir: &str,
) -> Result<(), FixMyLibErrors> {
    for script in &scan.scripts {
        let entry_properties = parse_entry(entry, input_dir)?;
        info!("****** {} -> {} ", &entry_properties, script.name);
        match entry_properties {
            EntryProperties::File(file_properties) => {
                process_script_for_file(file_properties, script, db_dir)
                    .map_or_else(|e| error!("Failure processing script: {e}"), |_| ())
            }
            EntryProperties::Folder(folder_properties) => {
                process_script_for_both(folder_properties, script, db_dir)
                    .map_or_else(|e| error!("Failure processing script: {e}"), |_| ())
            }
        }
    }
    Ok(())
}

fn execute_script(
    properties: impl ContentReplacer,
    script: &Script,
) -> Result<String, FixMyLibErrors> {
    let script_contents = &script.contents;
    let prefix = properties.main_identifier();
    let c = properties.replace_tokens(script_contents);
    debug!("{prefix} Will run command: {c}");

    let capture_data = Exec::shell(&c)
        .cwd(&script.working_dir)
        .stdout(Redirection::Pipe)
        .stderr(Redirection::Merge)
        .capture()?;
    let stdout = capture_data.stdout_str();
    let exit_status = capture_data.exit_status;
    debug!("{prefix} Result stdout: {stdout}");
    match exit_status {
        ExitStatus::Exited(code) if code == 0 => Ok(stdout),
        _ => Err(FixMyLibErrors::ExitCodeFailure(stdout, c)),
    }
}

fn process_config(config: Config, args: Args) -> Result<(), FixMyLibErrors> {
    for preset in config.presets {
        warn!("[{}] Start processing ", preset.name);
        for scan in preset.scans {
            warn!(
                "[{}/{}] Start processing new scan ",
                preset.name,
                scan.name()
            );
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(scan.threads)
                .build()
                .unwrap_or_else(|e| panic!("Failure initing threadpool: {e}"));
            pool.install(|| {
                WalkDir::new(&scan.input_folder)
                    .into_iter()
                    .par_bridge()
                    .filter(|entry| entry.is_ok())
                    .filter(|entry| entry.as_ref().unwrap().metadata().is_ok())
                    .map(|entry| entry.unwrap())
                    .filter(|entry| match scan.iterate_on {
                        IterateOn::Folders => entry.metadata().unwrap().is_dir(),
                        IterateOn::Files => entry.metadata().unwrap().is_file(),
                    })
                    .for_each(|entry| process_entry(entry, &scan, &args));
            });
        }
    }

    Ok(())
}

fn main() -> Result<(), FixMyLibErrors> {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    let args: Args = Args::parse();
    let config = config_parser::get_config(&args.config_folder_path, &args.builtins_folder_path);

    process_config(config, args)
        .map_or_else(|e| error!("Something went wrong: {e}"), |_| info!("Done!"));
    Ok(())
}
