use crate::processor::{CommandRunner, FileToBeProcessed, ProcessingResult};
use crate::AppContext;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelIterator;
use rayon::ThreadPool;

pub struct VideoConverterProcessor {
    pool: ThreadPool,
}

impl VideoConverterProcessor {
    pub fn new(ctx: &AppContext) -> VideoConverterProcessor {
        VideoConverterProcessor {
            pool: rayon::ThreadPoolBuilder::new()
                .num_threads(ctx.config.video_converter_threads)
                .build()
                .unwrap_or_else(|e| panic!("Failure initing threadpool for video processing: {e}")),
        }
    }

    pub fn convert_files(&self, files: Vec<FileToBeProcessed>) -> Vec<ProcessingResult> {
        let mut r: Vec<ProcessingResult> = vec![];

        self.pool.install(|| {
            r = files
                .into_par_iter()
                .map(|file| VideoConverter::from(&file).run())
                .collect();
        });
        r
    }
}

pub struct VideoConverter<'a> {
    file: &'a FileToBeProcessed<'a>,
}

impl VideoConverter<'_> {
    pub fn from<'a>(file: &'a FileToBeProcessed) -> VideoConverter<'a> {
        VideoConverter { file }
    }

    pub fn run(&self) -> ProcessingResult {
        let hw_transcoding = self.run_hw_transcoding_intel();
        if hw_transcoding.has_succeeded {
            return hw_transcoding;
        }

        info!("video conversion for {} has failed using hw transcoding, fallback to software transcoding...", self.file.file_full_path());
        self.run_software_transcoding()
    }

    fn run_hw_transcoding_intel(&self) -> ProcessingResult {
        CommandRunner::build(self.file.output_folder)
            .with(self.define_input_and_output_paths())
            .with(match self.file.preset_name {
                "thumbnail" => self.convert_video_preview_intel_hw_transcoding_thumbnail(),
                _ => self.convert_video_preview_intel_hw_transcoding_preview(),
            })
            .with(self.copy_metadata())
            .with(self.copy_file_modification_date())
            .run()
    }

    fn run_software_transcoding(&self) -> ProcessingResult {
        CommandRunner::build(self.file.output_folder)
            .with(self.define_input_and_output_paths())
            .with(match self.file.preset_name {
                "thumbnail" => self.convert_video_preview_software_transcoding_thumbnail(),
                _ => self.convert_video_preview_software_transcoding_preview(),
            })
            .with(self.copy_metadata())
            .with(self.copy_file_modification_date())
            .run()
    }

    fn define_input_and_output_paths(&self) -> String {
        let output_filepath = self
            .file
            .relative_path_with_file_stem_and_a_given_extension("mp4");
        format!(
            r#"mkdir -p "{}"
input="{}"
output="{}""#,
            self.file.relative_path(),
            self.file.file_full_path(),
            output_filepath
        )
    }

    fn copy_metadata(&self) -> &str {
        r#"exiftool -overwrite_original -TagsFromFile "$input" "-all:all>all:all" "$output""#
    }

    fn convert_video_preview_intel_hw_transcoding_preview(&self) -> &str {
        r#"ffmpeg -nostdin -y -noautorotate \
    -hwaccel vaapi -hwaccel_device /dev/dri/renderD128 \
    -hwaccel_output_format vaapi \
    -i "$input" \
    -vf "scale_vaapi=w='if(gt(iw,ih),1280,trunc(oh*a/2)*2)':h='if(gt(iw,ih),trunc(ow/a/2)*2,1280)':format=nv12" -c:v h264_vaapi \
    -movflags use_metadata_tags \
    "$output""#
    }

    fn convert_video_preview_intel_hw_transcoding_thumbnail(&self) -> &str {
        r#"ffmpeg -nostdin -y -noautorotate \
    -hwaccel vaapi -hwaccel_device /dev/dri/renderD128 \
    -hwaccel_output_format vaapi \
    -i "$input" \
    -vf "scale_vaapi=w='if(gt(iw,ih),320,trunc(oh*a/2)*2)':h='if(gt(iw,ih),trunc(ow/a/2)*2,320)':format=nv12" -c:v h264_vaapi \
    -movflags use_metadata_tags \
    "$output""#
    }

    fn convert_video_preview_software_transcoding_preview(&self) -> &str {
        r#"ffmpeg -nostdin -y -noautorotate \
   -i "$input" \
   -vf "scale=w='if(gt(iw,ih),1280,trunc(oh*a/2)*2)':h='if(gt(iw,ih),trunc(ow/a/2)*2,1280)'" -c:v libx264 \
   -pix_fmt yuv420p \
   -movflags use_metadata_tags \
   "$output""#
    }

    fn convert_video_preview_software_transcoding_thumbnail(&self) -> &str {
        r#"ffmpeg -nostdin -y -noautorotate \
   -i "$input" \
   -vf "scale=w='if(gt(iw,ih),320,trunc(oh*a/2)*2)':h='if(gt(iw,ih),trunc(ow/a/2)*2,320)'" -c:v libx264 \
   -pix_fmt yuv420p \
   -movflags use_metadata_tags \
   "$output""#
    }

    fn copy_file_modification_date(&self) -> &str {
        r#"touch -r "$input" "$output""#
    }
}
