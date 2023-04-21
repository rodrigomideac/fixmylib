create extension if not exists "uuid-ossp";

create table if not exists filescan_jobs
(
    id UUID PRIMARY KEY,
    full_path TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    finished_at TIMESTAMP
);

create table if not exists folders
(
    folder_full_path TEXT PRIMARY KEY,
    "path" TEXT NOT NULL,
    "name" TEXT NOT NULL,
    parent_folder_full_path TEXT NOT NULL,
    job_id UUID NOT NULL,
    CONSTRAINT fk_parent_folder_full_path FOREIGN KEY (parent_folder_full_path) REFERENCES folders(folder_full_path),
    CONSTRAINT fk_job_id FOREIGN KEY (job_id) REFERENCES filescan_jobs(id)

);
CREATE INDEX idx_folders_fk_parent_folder_full_path ON folders (parent_folder_full_path);
CREATE INDEX idx_folders_fk_job_id ON folders(job_id);

create table if not exists files
(
    file_full_path TEXT PRIMARY KEY,
    folder_full_path TEXT NOT NULL,
    "path" TEXT NOT NULL,
    "size" BIGINT NOT NULL,
    stem TEXT NOT NULL,
    extension TEXT NOT NULL,
    "name" TEXT NOT NULL,
    has_been_processed BOOL NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    file_modified_at TIMESTAMP NOT NULL,
    job_id UUID NOT NULL,
    CONSTRAINT fk_folder_full_path FOREIGN KEY (folder_full_path) REFERENCES folders(folder_full_path),
    CONSTRAINT fk_job_id FOREIGN KEY (job_id) REFERENCES filescan_jobs(id)

);
CREATE INDEX idx_files_fk_folder_full_path ON files (folder_full_path);
CREATE INDEX idx_files_fk_job_id ON files(job_id);

create table if not exists file_jobs
(
    file_full_path TEXT NOT NULL,
    job_name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    finished_at TIMESTAMP,
    command TEXT,
    command_log TEXT,
    has_succeeded BOOL,
    PRIMARY KEY(file_full_path, job_name),
    CONSTRAINT fk_file_full_path FOREIGN KEY (file_full_path) REFERENCES files(file_full_path)
);
CREATE INDEX idx_file_jobs_fk_file_full_path ON file_jobs (file_full_path);
