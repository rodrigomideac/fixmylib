use crate::time;
use anyhow::{Context, Result};
use sqlx::types::time::PrimitiveDateTime;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

#[derive(Debug, PartialEq, Clone)]
pub struct File {
    pub file_full_path: String,
    pub folder_full_path: String,
    pub path: String,
    pub size: i64,
    pub stem: String,
    pub extension: String,
    pub name: String,
    pub has_been_processed: bool,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub file_modified_at: PrimitiveDateTime,
    pub job_id: Uuid,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Folder {
    pub folder_full_path: String,
    pub path: String,
    pub name: String,
    pub parent_folder_full_path: String,
    pub job_id: Uuid,
}

#[derive(Debug, PartialEq, Clone)]
pub struct FilescanJob {
    pub id: Uuid,
    pub full_path: String,
    pub created_at: PrimitiveDateTime,
    pub finished_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct FileJob {
    pub file_full_path: String,
    pub job_name: String,
    pub created_at: PrimitiveDateTime,
    pub finished_at: Option<PrimitiveDateTime>,
    pub command: Option<String>,
    pub command_log: Option<String>,
    pub has_succeeded: Option<bool>,
}

pub async fn get_unfinished_filescan_jobs(db: &Pool<Postgres>) -> Result<Vec<FilescanJob>> {
    let filescans = sqlx::query_as!(
        FilescanJob,
        r#"
        select * from filescan_jobs where finished_at is null
        "#
    )
    .fetch_all(db)
    .await
    .context("could not get  folder")?;
    Ok(filescans)
}

pub async fn upsert_filescan_job(db: &Pool<Postgres>, job: FilescanJob) -> Result<FilescanJob> {
    let filescan_job = sqlx::query_as!(
        FilescanJob,
        r#"
        with filescan_job_upsert as (
        insert into filescan_jobs (id, full_path, created_at, finished_at) values ($1, $2, $3, $4)
        on conflict(id) do update set
            full_path = excluded.full_path,
            created_at = excluded.created_at,
            finished_at = excluded.finished_at
            returning *
        )
        select * from filescan_job_upsert where id = $1
        "#,
        job.id,
        job.full_path,
        job.created_at,
        job.finished_at
    )
    .fetch_one(db)
    .await
    .context("could not upsert filescan job")?;
    Ok(filescan_job)
}

pub async fn upsert_file_job(db: &Pool<Postgres>, job: FileJob) -> Result<FileJob> {
    let filescan_job = sqlx::query_as!(
        FileJob,
        r#"
        with file_job_upsert as (
        insert into file_jobs (file_full_path, job_name, created_at, finished_at) values ($1, $2, $3, $4)
        on conflict(file_full_path, job_name) do update set
            created_at = excluded.created_at,
            finished_at = excluded.finished_at
            returning *
        )
        select * from file_job_upsert where file_full_path = $1 and job_name = $2
        "#,
        job.file_full_path,
        job.job_name,
        job.created_at,
        job.finished_at
    )
        .fetch_one(db)
        .await
        .context("could not upsert file job")?;
    Ok(filescan_job)
}

pub async fn mark_file_job_as_completed(
    db: &Pool<Postgres>,
    file_full_path: &str,
    job_name: &str,
    command: Option<String>,
    command_log: Option<String>,
    has_succeeded: Option<bool>,
) -> Result<()> {
    sqlx::query!(
        r#"
        update file_jobs set finished_at = $1, command = $2, command_log = $3, has_succeeded = $4
        where file_full_path = $5 and job_name = $6
        "#,
        time::now(),
        command,
        command_log,
        has_succeeded,
        file_full_path,
        job_name
    )
    .execute(db)
    .await
    .context("could not mark file job as completed")?;
    Ok(())
}

pub async fn get_folder(db: &Pool<Postgres>, full_path: String) -> Result<Option<Folder>> {
    let folder = sqlx::query_as!(
        Folder,
        r#"
        select * from folders where folder_full_path = $1
        "#,
        full_path
    )
    .fetch_optional(db)
    .await
    .context("could not get folder")?;
    Ok(folder)
}

pub async fn get_folders(db: &Pool<Postgres>) -> Result<Vec<Folder>> {
    let folders = sqlx::query_as!(
        Folder,
        r#"
        select * from folders;
        "#
    )
    .fetch_all(db)
    .await
    .context("could not get  folder")?;
    Ok(folders)
}

pub async fn upsert_folder(db: &Pool<Postgres>, folder: Folder) -> Result<Folder> {
    info!("Going to insert {:?}", folder);
    let folder = sqlx::query_as!(
        Folder,
        r#"
        with folder_upsert as (
        insert into
                "folders" (
                  folder_full_path,
                  path,
                  name,
                  parent_folder_full_path,
                  job_id
                )
              values
                ($1, $2, $3, $4, $5) on conflict (folder_full_path) DO UPDATE SET
            "path" = excluded."path",
            "name" = excluded."name",
            parent_folder_full_path = excluded.parent_folder_full_path,
            job_id = excluded.job_id
            returning *
        )
        select * from folder_upsert where folder_full_path = $1
    "#,
        folder.folder_full_path,
        folder.path,
        folder.name,
        folder.parent_folder_full_path,
        folder.job_id
    )
    .fetch_one(db)
    .await
    .context("Failed to insert in db")?;
    Ok(folder)
}

pub async fn upsert_file(db: &Pool<Postgres>, file: File) -> Result<File> {
    let file = sqlx::query_as!(
        File,
        r#"
        with file_insert as (
            insert into "files" (
            file_full_path,
            folder_full_path,
            path,
            size,
            stem,
            extension,
            name,
            has_been_processed,
            created_at,
            updated_at,
            file_modified_at,
            job_id
            ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            on conflict (file_full_path) DO UPDATE SET
                file_full_path = excluded.file_full_path,
                folder_full_path = excluded.folder_full_path,
                path = excluded.path,
                size = excluded.size,
                stem = excluded.stem,
                extension = excluded.extension,
                name = excluded.name,
                has_been_processed = excluded.has_been_processed,
                created_at = excluded.created_at,
                updated_at = excluded.updated_at,
                file_modified_at = excluded.file_modified_at,
                job_id = excluded.job_id
            returning *
        ) select * from file_insert where file_full_path = $1
        "#,
        file.file_full_path,
        file.folder_full_path,
        file.path,
        file.size,
        file.stem,
        file.extension,
        file.name,
        file.has_been_processed,
        file.created_at,
        file.updated_at,
        file.file_modified_at,
        file.job_id
    )
    .fetch_one(db)
    .await
    .context("could not insert file")?;
    Ok(file)
}

pub async fn get_unprocessed_files_for_a_given_job_name(
    db: &Pool<Postgres>,
    resolution: &str,
    offset: i64,
    limit: i64,
) -> Result<Vec<File>> {
    let files = sqlx::query_as!(
        File,
        r#"SELECT
            files.file_full_path as "file_full_path!",
            files.folder_full_path as "folder_full_path!",
            files.path as "path!",
            files.size as "size!",
            files.stem as "stem!",
            files.extension as "extension!",
            files.name as "name!",
            files.has_been_processed as "has_been_processed!",
            files.created_at as "created_at!",
            files.updated_at as "updated_at!",
            files.file_modified_at as "file_modified_at!",
            files.job_id as "job_id!"
             from files
             LEFT JOIN file_jobs ON files.file_full_path = file_jobs.file_full_path AND file_jobs.job_name = $1
             WHERE file_jobs.finished_at IS NULL
             ORDER BY files.folder_full_path
             OFFSET $2 ROWS
             FETCH NEXT $3 ROWS ONLY
             "#,
        resolution,
        offset,
        limit
    )
        .fetch_all(db)
        .await
        .context("could not fetch files")?;
    Ok(files)
}
