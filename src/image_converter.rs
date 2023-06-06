use crate::processor::{CommandRunner, FileToBeProcessed, ProcessingResult};
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelIterator;
use rayon::ThreadPool;

pub struct ImageConverterProcessor {
    pool: ThreadPool,
}

impl ImageConverterProcessor {
    pub fn new() -> ImageConverterProcessor {
        ImageConverterProcessor {
            pool: rayon::ThreadPoolBuilder::new()
                .num_threads(8)
                .build()
                .unwrap_or_else(|e| panic!("Failure initing threadpool for image processing: {e}")),
        }
    }

    pub fn convert_files(&self, files: Vec<FileToBeProcessed>) -> Vec<ProcessingResult> {
        let mut r: Vec<ProcessingResult> = vec![];

        self.pool.install(|| {
            r = files
                .into_par_iter()
                .map(|file| ImageConverter::from(&file).run())
                .collect();
        });
        r
    }
}

pub struct ImageConverter<'a> {
    file: &'a FileToBeProcessed<'a>,
}

impl ImageConverter<'_> {
    pub fn from<'a>(file: &'a FileToBeProcessed) -> ImageConverter<'a> {
        ImageConverter { file }
    }

    fn run(&self) -> ProcessingResult {
        CommandRunner::build(self.file.output_folder)
            .with(self.define_input_and_output_paths())
            .with(match self.file.resolution {
                "thumbnail" => self.convert_image_thumbnail(),
                _ => self.convert_image_preview(),
            })
            .with(self.copy_file_modification_date())
            .run()
    }

    fn define_input_and_output_paths(&self) -> String {
        let output_filepath = self
            .file
            .relative_path_with_file_stem_and_a_given_extension("jpg");
        format!(
            r#"mkdir -p "{}"
input="{}"
output="{}""#,
            self.file.relative_path(),
            self.file.file_full_path(),
            output_filepath
        )
    }

    fn convert_image_preview(&self) -> &str {
        r#"convert "$input" -resize 1280x1280^ "$output""#
    }

    fn convert_image_thumbnail(&self) -> &str {
        r#"convert "$input" -resize 400x400^ "$output""#
    }

    fn copy_file_modification_date(&self) -> &str {
        r#"touch -r "$input" "$output""#
    }
}
