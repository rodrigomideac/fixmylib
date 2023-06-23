use crate::db::{File, FileJob};

use crate::{db, AppContext};
use anyhow::Result;

use std::time::Duration;
use clap::command;
use sqlx::types::time::PrimitiveDateTime;

use crate::exiftool::{exiftool_on_file, Exiftool};
use crate::image_converter::ImageConverterProcessor;
use crate::time::{now, Ticker};
use crate::video_converter::VideoConverterProcessor;
use subprocess::{Exec, ExitStatus, Redirection};
use tokio::time::sleep;

#[derive(Debug, PartialEq, Clone)]
pub struct VideoMetrics {
    pub fps: u32,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ProcessingMetrics {
    Video(VideoMetrics),
    Image,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ProcessingResult {
    pub command: String,
    pub command_log: String,
    pub has_succeeded: bool,
    processing_started_at: PrimitiveDateTime,
    processing_finished_at: PrimitiveDateTime,
    metrics: Option<ProcessingMetrics>,
}

impl ProcessingResult {
    pub fn new() -> ProcessingResult {
        ProcessingResult {
            command: "".to_string(),
            command_log: "".to_string(),
            has_succeeded: false,
            processing_started_at: now(),
            processing_finished_at: now(),
            metrics: None,
        }
    }

    pub fn with_command_log(mut self, command_log: String) -> ProcessingResult {
        self.command_log = command_log;
        self
    }

    pub fn with_command(mut self, command: String) -> ProcessingResult {
        self.command = command;
        self
    }

    pub fn with_metrics(mut self, metrics: ProcessingMetrics) -> ProcessingResult {
        self.metrics = Some(metrics);
        self
    }

    pub fn succeeded(mut self) -> ProcessingResult {
        self.has_succeeded = true;
        self.processing_finished_at = now();
        self
    }

    pub fn failed(mut self) -> ProcessingResult {
        self.has_succeeded = false;
        self.processing_finished_at = now();
        self
    }
}

pub struct CommandRunner {
    cwd: String,
    cmd: String,
}

impl CommandRunner {
    pub fn build(cwd: impl Into<String>) -> CommandRunner {
        CommandRunner {
            cwd: cwd.into(),
            cmd: "".to_string(),
        }
            .with(r#"#!/bin/sh"#)
            .with(r#"set -e"#)
    }

    pub fn with(mut self, partial_cmd: impl AsRef<str>) -> CommandRunner {
        self.cmd = self.cmd + partial_cmd.as_ref() + "\n";
        self
    }

    pub fn run(&self) -> ProcessingResult {
        debug!("Will run command: {}", &self.cmd);
        let result = ProcessingResult::new();
        let capture_data_result = Exec::shell(&self.cmd)
            .cwd(&self.cwd)
            .stdout(Redirection::Pipe)
            .stderr(Redirection::Merge)
            .capture();
        if let Err(e) = capture_data_result {
            return result.with_command(self.cmd.clone()).with_command_log(e.to_string()).failed();
        }
        let capture_date = capture_data_result.unwrap();
        let stdout = capture_date.stdout_str();
        debug!("Result stdout: {stdout}");
        let exit_status = capture_date.exit_status;
        match exit_status {
            ExitStatus::Exited(code) if code == 0 =>
                result.with_command_log(stdout).succeeded(),
            _ => result.with_command(self.cmd.clone()).with_command_log(stdout).failed(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct FileToBeProcessed<'a> {
    pub root: &'a str,
    pub output_folder: &'a str,
    pub preset_name: &'a str,
    pub file: File,
    pub file_job: FileJob,
    pub exif: Exiftool,
}

impl FileToBeProcessed<'_> {
    pub fn file_full_path(&self) -> &str {
        &self.file.file_full_path
    }

    pub fn file_stem(&self) -> &str {
        &self.file.stem
    }

    pub fn folder_full_path(&self) -> &str {
        &self.file.folder_full_path
    }

    pub fn is_image(&self) -> bool {
        self.exif.mime_type.contains("image")
    }

    pub fn is_video(&self) -> bool {
        self.exif.mime_type.contains("video")
    }

    pub fn relative_path_with_file_stem_and_a_given_extension(
        &self,
        target_extension: &str,
    ) -> String {
        format!(
            "{}/{}.{}",
            self.relative_path(),
            self.file_stem(),
            target_extension
        )
    }
    pub fn relative_path(&self) -> String {
        format!(
            "{}/{}",
            self.preset_name,
            self.folder_full_path().replacen(self.root, ".", 1)
        )
    }
}

pub async fn run(ctx: &AppContext) -> Result<()> {
    loop {
        info!("Checking for unprocessed files...");
        for preset_name in get_preset_names(ctx) {
            debug!("Creating jobs for preset {preset_name}");
            let ticker = Ticker::new();
            create_file_jobs_for_unprocessed_files(ctx, &preset_name)
                .await
                .expect("it should work flawless to create file jobs");
            let processed_files_count = Processor::new(ctx)
                .process_pending_file_jobs(&preset_name)
                .await
                .expect("it should work flawless to process files");
            if processed_files_count != 0 {
                ticker.elapsed(format!(
                    "to process unprocessed files for {preset_name} preset."
                ))
            }
        }
        sleep(Duration::from_secs(
            ctx.config.seconds_between_processor_runs,
        ))
            .await;
    }
}

fn get_preset_names(ctx: &AppContext) -> Vec<String> {
    let mut presets = vec![];
    if ctx.config.enable_preview_preset {
        presets.push("preview".into());
    }
    if ctx.config.enable_thumbnail_preset {
        presets.push("thumbnail".into());
    }
    presets
}

pub async fn create_file_jobs_for_unprocessed_files(
    ctx: &AppContext,
    preset_name: &str,
) -> Result<()> {
    let mut offset = 0;
    let limit = 100;
    let mut count = 0;
    loop {
        let files =
            db::get_unprocessed_files_for_a_given_job_name(&ctx.db, preset_name, offset, limit)
                .await?;
        if files.is_empty() {
            break;
        }
        debug!(
            "Found {} unprocessed files for {}:",
            files.len(),
            preset_name
        );

        for x in files.iter() {
            debug!("{}", x.file_full_path)
        }

        let jobs: Vec<FileJob> = files
            .into_iter()
            .map(|f| FileJob {
                file_full_path: f.file_full_path,
                job_name: preset_name.to_owned(),
                created_at: now(),
                finished_at: None,
                command: None,
                command_log: None,
                has_succeeded: None,
            })
            .collect();
        for job in jobs {
            match db::upsert_file_job(&ctx.db, job).await {
                Ok(_) => count += 1,
                Err(e) => {
                    error!("Failure inserting file_job: {e}")
                }
            }
        }

        offset += limit;
    }
    if count != 0 {
        info!("Created {count} jobs to process files.");
    }
    Ok(())
}

struct Processor<'a> {
    image_converter: ImageConverterProcessor,
    video_converter: VideoConverterProcessor,
    ctx: &'a AppContext,
}

impl Processor<'_> {
    fn new(ctx: &AppContext) -> Processor {
        Processor {
            image_converter: ImageConverterProcessor::new(ctx),
            video_converter: VideoConverterProcessor::new(ctx),
            ctx,
        }
    }
    pub async fn process_pending_file_jobs(&self, preset_name: &str) -> Result<i32> {
        let mut offset = 0;
        let limit = 100;
        let mut count = 0;
        loop {
            let files_and_jobs =
                db::get_unprocessed_file_and_jobs(&self.ctx.db, preset_name, offset, limit).await?;

            if files_and_jobs.is_empty() {
                break;
            }
            info!("Processing {} files...", files_and_jobs.len());

            for (file, _) in files_and_jobs.iter() {
                debug!("{}", file.file_full_path)
            }

            let processed_data = self.process_files(files_and_jobs, preset_name);

            print_statistics(&processed_data);

            let updated_file_jobs = processed_data
                .into_iter()
                .map(
                    |(
                         file,
                         file_job,
                         ProcessingResult {
                             command,
                             command_log,
                             has_succeeded,
                             ..
                         },
                     )| {
                        FileJob {
                            file_full_path: file.file_full_path,
                            job_name: preset_name.to_owned(),
                            finished_at: Some(now()),
                            command: Some(command),
                            command_log: Some(command_log),
                            has_succeeded: Some(has_succeeded),
                            ..file_job
                        }
                    }
                )
                .collect::<Vec<FileJob>>();


            let job_count: i32 = updated_file_jobs.len() as i32;
            let tick = Ticker::new();
            db::upsert_file_jobs(&self.ctx.db, updated_file_jobs).await?;
            tick.elapsed("To insert a batch of filejobs");
            count += job_count;

            offset += limit;
        }
        if count != 0 {
            info!("Processed {count} files.");
        }
        Ok(count)
    }

    fn process_files(
        &self,
        files: Vec<(File, FileJob)>,
        preset_name: &str,
    ) -> Vec<(File, FileJob, ProcessingResult)> {
        enum ExifProcessing {
            Success((File, FileJob, Exiftool)),
            Failure((File, FileJob, ProcessingResult)),
        }
        let files_count = files.len();
        let exifs: Vec<ExifProcessing> = files
            .into_iter()
            .map(|(file, job)| match exiftool_on_file(&file.file_full_path) {
                Ok(exif) => ExifProcessing::Success((file, job, exif)),
                Err(e) => ExifProcessing::Failure((
                    file,
                    job,
                    ProcessingResult::new().with_command_log(format!("Failure extracting exif data: {:?}", e)).failed(),
                )),
            })
            .collect();
        let (success_exifs, failed_exifs): (Vec<_>, Vec<_>) = exifs
            .into_iter()
            .partition(|e| matches!(e, ExifProcessing::Success(_)));
        if !failed_exifs.is_empty() {
            info!(
                "Failure to extract file types of {} files.",
                failed_exifs.len()
            );
        }
        let (media_files, non_media_files): (Vec<_>, Vec<_>) = success_exifs
            .into_iter()
            .flat_map(|e| match e {
                ExifProcessing::Success(file_and_exif) => Some(file_and_exif),
                ExifProcessing::Failure(_) => None,
            })
            .map(|(file, file_job, exif)| FileToBeProcessed {
                root: &self.ctx.config.input_folder,
                output_folder: &self.ctx.config.output_folder,
                preset_name,
                file,
                file_job,
                exif,
            })
            .partition(|f| f.is_video() || f.is_image());

        let (image_files, video_files): (Vec<_>, Vec<_>) =
            media_files.into_iter().partition(|f| f.is_image());

        info!(
            "Found {} images, {} videos and {} non media files out of {} files.",
            image_files.len(),
            video_files.len(),
            non_media_files.len(),
            files_count
        );
        info!("Processing images...");
        let images_processed: Vec<_> = image_files
            .clone()
            .into_iter()
            .zip(self.image_converter.convert_files(image_files))
            .map(|(f, r)| (f.file, f.file_job, r))
            .collect();
        info!("Processing videos...");
        let videos_processed: Vec<_> = video_files
            .clone()
            .into_iter()
            .zip(self.video_converter.convert_files(video_files))
            .map(|(f, r)| (f.file, f.file_job, r))
            .collect();

        let non_media_files_processed: Vec<_> = non_media_files
            .into_iter()
            .map(|f| {
                (
                    f.file,
                    f.file_job,
                    ProcessingResult::new().with_command_log(format!(
                        "File is neither imager or video. Mime type: {}",
                        f.exif.mime_type
                    )).succeeded(),
                )
            })
            .collect();

        let failed_exifs_processed: Vec<_> = failed_exifs
            .into_iter()
            .flat_map(|e| match e {
                ExifProcessing::Success(_) => None,
                ExifProcessing::Failure(e) => Some(e),
            })
            .collect();

        [
            images_processed,
            videos_processed,
            non_media_files_processed,
            failed_exifs_processed,
        ]
            .concat()
    }
}

fn print_statistics(data: &[(File, FileJob, ProcessingResult)]) {
    let elements = data.iter()
        .flat_map(|(_, _, ProcessingResult { metrics, .. })|
            if let Some(ProcessingMetrics::Video(VideoMetrics { fps })) = metrics {
                Some(*fps)
            } else {
                None
            }
        ).collect::<Vec<u32>>();

    let sum: u32 = elements.iter().sum();
    let mean = sum as f64 / elements.len() as f64;
    info!("Mean transcoding FPS for video: {mean}")
}
