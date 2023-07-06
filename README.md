# fixmylib

This project converts photos and videos to lower resolutions and more widely accepted formats for playback. Currently, two presets are created:

|  Preset name  |    Image extension     | Image minimum width/height* | Video codec  | Video minimum width/height* |
|:-------------:|:----------------------:|:---------------------------:|:------------:|:---------------------------:|
|   thumbnail   |          JPEG          |            400px            |   H264/AVC   |            320px            |
|    preview    |          JPEG          |           1280px            |   H264/AVC   |           1280px            |

*: Media aspect ratio will be respected.

Features:
- Video hardware transcoding for Intel iGPUs using VA-API, with fallback to software transcoding if HW transcoding fails;
- Converted videos will keep relevant video metadata;
- Any file with MIME type `image` or `video` are elegible to be tried to be converted;

The motivation for this project was to convert a media library with HEVC videos and HEIF photos to H264 and JPEG, respectively, for better playback compatibility. Also, it works great if you want to have small backups (compromising quality) of your media library that can fit in SD cards. 

## Getting started

To run the example without needing to install any dependencies besides Docker compose:

```bash
cd examples/convert_heic_and_hevc
docker compose up
```

The generated files will be on `examples/convert_heic_and_hevc/media-out`.

## Configure for your needs

Create a `docker-compose.yaml` file with these contents and customize by your needs:

```yaml
version: "2"
services:
  fixmylib-db:
    image: postgres:14
    ports:
      - "5432:5432"
    volumes:
      # Mount a folder from your host to store the database data
      - ~/apps/fixmylib/db:/var/lib/postgresql/data
    environment:
      - POSTGRES_PASSWORD=fixmylib

  fixmylib:
    image: rodrigomideac/fixmylib:latest
    environment:
      # The following envvars are the default values. You can omit them if you are not going to customize.
      
      # Do not change, it is configured for the postgres companion container. 
      # Format: postgresql://USERNAME:PASSWORD@HOST/DB_NAME
      - DATABASE_URL="postgresql://postgres:fixmylib@localhost/postgres"
      
      # Set log level. 
      - RUST_LOG="fixmylib=info,sqlx=warn"
       
      # Set input folder containing media to be converted.  
      - INPUT_FOLDER=/media-in
      
      # Set output folder where converted files for the presets are going to be stored.
      - OUTPUT_FOLDER=/media-out
       
      # Set folder to write error report at the end of conversion.  
      - LOGS_FOLDER=/media-out

      # Set concurrency for scanner job.  
      - SCANNER_THREADS=4
      
      # Set concurrency for image conversion. It is CPU bound, so it is suggested to keep it the same value
      # as the number of available threads in you system.
      - IMAGE_CONVERTER_THREADS=4
      
      # Set concurrency for video conversion. Good results for both Hardware and Software transcoding were obtained for the value 1.
      - VIDEO_CONVERTER_THREADS=1

      # Set time to wait between new file discovery by scanner job.  
      - SECONDS_BETWEEN_FILE_SCANS=3600

      # Set time to wait between checks for new discovered files that haven't been processed yet.
      - SECONDS_BETWEEN_PROCESSOR_RUNS=10

      # Enable conversion for the thumbnail preset.
      - ENABLE_THUMBNAIL_PRESET=true

      # Enable conversion for the preview preset.
      - ENABLE_PREVIEW_PRESET=true
    volumes:
      # Mount the destination folder from your host to the OUTPUT_FOLDER set above
      - ~/apps/fixmylib/media-out:/media-out
      # Mount the input folder from your host to the INPUT_FOLDER set above
      - /path/to/your/media:/media-in
    devices:
      # The following is necessary if you want to enable hardware transcoding for Intel iGPUs
      - /dev/dri:/dev/dri
```

Now run it with `docker compose up`.

Any failed conversion will be reported in the file `processing_errors.csv` at the `/media-out` folder. 

## Project Vision and Roadmap
This project has the aspiration of being a [photoview](https://github.com/photoview/photoview) but with write features. Upcoming features:

- Generate conversion report to make it easier to debug why conversion failed;
- Store media data in the database;
- API to list and serve media;
- Frontend to view the media library;

## Alternatives
[Tdarr](https://github.com/HaveAGitGat/Tdarr) worked great for its purpose of having a way to convert videos to a desired preset. However, I found its UI a little confusing, and it lacked capability to convert photos. 

[Fileflows](https://github.com/revenz/FileFlows) had a better UI, but again, it lacked the feature of converting HEIF photos.

