use crate::processor::{CommandRunner, FileToBeProcessed, ProcessingMetrics, ProcessingResult, VideoMetrics};
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
        let result = self.run_using_hw_or_sw_transcoding();
        self.add_metrics(result)
    }

    fn run_using_hw_or_sw_transcoding(&self) -> ProcessingResult {
        let hw_transcoding = self.run_hw_transcoding_intel();
        if hw_transcoding.has_succeeded {
            return hw_transcoding;
        }

        info!("video conversion for {} has failed using hw transcoding, fallback to software transcoding...", self.file.file_full_path());
        self.run_software_transcoding()
    }

    fn add_metrics(&self, result: ProcessingResult) -> ProcessingResult {
        if !result.has_succeeded {
            return result;
        }

        let maybe_fps = self.parse_fps(&result.command_log);

        if let Some(fps) = maybe_fps {
            result.with_metrics(ProcessingMetrics::Video(VideoMetrics { fps }))
        } else {
            result
        }
    }

    fn parse_fps(&self, command_log: &str) -> Option<u32> {
        let mut statistics_line = "";
        for line in command_log.lines() {
            if line.starts_with("frame=") {
                statistics_line = line;
            }
        }
        let statistics_lines = statistics_line.split("\r").collect::<Vec<&str>>();
        let mut fps_sum: u32 = 0;
        let mut fps_elements_count: u32 = 0;
        for line in statistics_lines {
            let maybe_fps = self.parse_single_fps_line(line);
            match maybe_fps {
                Some(fps) if fps > 1 => {
                    fps_sum += fps;
                    fps_elements_count += 1;
                }
                _ => { continue; }
            }
        }

        if fps_sum > 0 {
            Some(fps_sum/fps_elements_count)
        } else {
            None
        }
    }

    fn parse_single_fps_line(&self, line: &str) -> Option<u32> {
        let maybe_fps_index = line.find("fps=");
        let maybe_q_index = line.find(" q=");
        if let (Some(fps_index), Some(q_index)) = (maybe_fps_index, maybe_q_index) {
            line[fps_index..q_index]
                .replace("fps=", "")
                .parse::<u32>()
                .map_err(|e| debug!("Failure extracting FPS from video conversion: {}",e))
                .ok()
        } else {
            None
        }
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
