use crate::db::{
    get_folder, get_folders, get_unfinished_filescan_jobs, upsert_file, upsert_filescan_job,
    upsert_folder, File, FilescanJob, Folder,
};
use crate::errors::FixMyLibErrors;
use crate::errors::FixMyLibErrors::PathParsing;
use crate::{time, AppContext};
use anyhow::{Context, Result};
use rayon::iter::ParallelIterator;
use rayon::prelude::ParallelBridge;
use sqlx::types::time::{OffsetDateTime, PrimitiveDateTime};
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;
use uuid::Uuid;
use walkdir::{DirEntry, WalkDir};

pub async fn run(ctx: &AppContext) -> anyhow::Result<()> {
    loop {
        let mut unfinished_scan_jobs = get_unfinished_filescan_jobs(&ctx.db).await?;
        if unfinished_scan_jobs.is_empty() {
            info!("Starting new full scan for files on {}", ctx.config.input_folder.clone());
            let job = FilescanJob {
                id: Uuid::new_v4(),
                full_path: ctx.config.input_folder.clone(),
                created_at: time::now(),
                finished_at: None,
            };
            upsert_filescan_job(&ctx.db, job.clone()).await?;
            unfinished_scan_jobs.push(job);
        }
        for job in unfinished_scan_jobs {
            run_job_for_folders(ctx, &job).await?;
            run_job_for_files(ctx, job.clone()).await?;
            upsert_filescan_job(
                &ctx.db,
                FilescanJob {
                    finished_at: Some(time::now()),
                    ..job
                },
            )
                .await?;
        }
        debug!("Done scanning all filescanjobs");
        sleep(Duration::from_secs(ctx.config.seconds_between_file_scans)).await;
    }
}

async fn run_job_for_files(ctx: &AppContext, job: FilescanJob) -> Result<()> {
    let all_folders = get_folders(&ctx.db).await?;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<File>(10);
    debug!("Going to search for files on {:?}", all_folders);
    tokio::spawn(async move {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(32)
            .build()
            .unwrap_or_else(|e| panic!("Failure initing threadpool: {e}"));
        pool.install(|| {
            all_folders.iter().par_bridge().for_each(|folder| {
                debug!("Iterating {}", folder.folder_full_path);
                process_folder_for_file_job(&job, folder, tx.clone());
            });
        });
    });
    let mut count = 0;
    while let Some(file) = rx.recv().await {
        upsert_file(&ctx.db, file).await?;
        count += 1;
    }
    info!("Found {count} files.");
    Ok(())
}

fn process_folder_for_file_job(job: &FilescanJob, folder: &Folder, tx: Sender<File>) {
    for entry in WalkDir::new(&folder.folder_full_path)
        .max_depth(1)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.metadata().is_ok() && entry.metadata().unwrap().is_file())
    {
        debug!("Found file: {:?}", entry);
        let entry = EntryProperties {
            entry: &entry,
            root: &folder.folder_full_path,
        };
        match entry.to_file(job.id) {
            Ok(file) => {
                let _ = tx
                    .blocking_send(file)
                    .context("Failure to send file via channel");
            }
            Err(e) => {
                error!("Could not process {}: {e}", entry.full_path().unwrap())
            }
        }
    }
}

async fn run_job_for_folders(ctx: &AppContext, job: &FilescanJob) -> Result<()> {
    let root = &job.full_path;
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.metadata().is_ok() && entry.metadata().unwrap().is_dir())
    {
        let entry = EntryProperties {
            entry: &entry,
            root,
        };
        process_folder_entry(ctx.clone(), entry, job.id).await?
    }
    Ok(())
}

async fn process_folder_entry(
    ctx: AppContext,
    entry: EntryProperties<'_>,
    job_id: Uuid,
) -> Result<()> {
    let updated_folder = if let Some(folder) = get_folder(&ctx.db, entry.full_path()?).await? {
        debug!("Folder {} already exists on DB", entry.full_path()?);
        Folder { job_id, ..folder }
    } else {
        debug!("Folder {} do not exists on DB", entry.full_path()?);
        let parent_folder_full_path = if entry.full_path()?.as_str() == entry.root {
            debug!(
                "Folder {} is root, overriding parent path ",
                entry.full_path()?
            );
            entry.full_path()?
        } else {
            entry.parent_folder_full_path()?
        };
        Folder {
            folder_full_path: entry.full_path()?,
            path: entry.path()?,
            name: entry.filename()?,
            parent_folder_full_path,
            job_id,
        }
    };
    upsert_folder(&ctx.db, updated_folder).await?;
    Ok(())
}

struct EntryProperties<'a> {
    entry: &'a DirEntry,
    root: &'a String,
}

impl EntryProperties<'_> {
    fn filesize(&self) -> Result<u64> {
        Ok(self.entry.metadata()?.len())
    }

    fn modified_date(&self) -> Result<PrimitiveDateTime> {
        let date: OffsetDateTime = self.entry.metadata()?.modified()?.into();

        Ok(PrimitiveDateTime::new(date.date(), date.time()))
    }

    fn path(&self) -> Result<String, FixMyLibErrors> {
        Ok(self
            .entry
            .path()
            .strip_prefix(self.root)?
            .as_os_str()
            .to_owned()
            .into_string()?)
    }

    fn parent_folder_full_path(&self) -> Result<String, FixMyLibErrors> {
        Ok(self
            .entry
            .path()
            .parent()
            .ok_or(PathParsing("failure obtaining parent_folder".to_owned()))?
            .as_os_str()
            .to_owned()
            .into_string()?)
    }

    fn full_path(&self) -> Result<String, FixMyLibErrors> {
        Ok(self.entry.path().as_os_str().to_owned().into_string()?)
    }

    fn stem(&self) -> Result<String, FixMyLibErrors> {
        Ok(self
            .entry
            .path()
            .file_stem()
            .ok_or(PathParsing(format!(
                "No filestem found for {}",
                self.path()?
            )))?
            .to_os_string()
            .into_string()?)
    }

    fn filename(&self) -> Result<String, FixMyLibErrors> {
        Ok(self
            .entry
            .file_name()
            .to_str()
            .ok_or(FixMyLibErrors::PathParsing(format!(
                "Failure parsing filename for {}",
                self.path()?
            )))?
            .to_string())
    }

    fn extension(&self) -> Result<String, FixMyLibErrors> {
        Ok(self
            .entry
            .path()
            .extension()
            .ok_or(FixMyLibErrors::PathParsing(format!(
                "No extension found for {}",
                self.path()?
            )))?
            .to_str()
            .ok_or(FixMyLibErrors::PathParsing(format!(
                "Failure parsing extension for {}",
                self.path()?
            )))?
            .to_lowercase())
    }

    fn to_file(&self, job_id: Uuid) -> Result<File> {
        Ok(File {
            file_full_path: self.full_path()?,
            folder_full_path: self.parent_folder_full_path()?,
            path: self.path()?,
            size: self.filesize()? as i64,
            stem: self.stem()?,
            extension: self.extension()?,
            name: self.filename()?,
            has_been_processed: false,
            created_at: time::now(),
            updated_at: time::now(),
            file_modified_at: self.modified_date()?,
            job_id,
        })
    }
}
