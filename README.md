# fixmylib

This will go through a media library and convert it to a given preset. It works by running a shell script against folders and files.

The Docker image contains `exiftool`, `ffmpeg` and `image-magick` with all available delegates (including for HEIC format). The motivation for this project was to convert a media library with HEVC videos and HEIF photos to H264 and JPEG, respectively, for better playback compatibility. 

Limitations:
- Hardware transcoding has been tested with VA-API on an Intel i3-8100T with Intel HD Graphics 630 with Proxmox as supervisor and GPU passthrough to a virtual machine;
- No software transcoding fallback yet;
- This tool makes no assumptions about the type of the file being processed, thus the scripts need to perform any kind of extra work, such as checking if the video is already in h264 format;
- Only one preset developed: `h264_and_jpeg_converter`.

## Getting started

You need to have installed `ffmpeg` and `image-magick` with HEIF delegate to run the example:

```bash
cargo run -- --config-folder-path examples/convert_heic_and_hevc/config --db-folder-path examples/convert_heic_and_hevc/db
```

There is a SQLite DB which stores data about each file and folder processed. You can have a look at its contents using a tool such as [DBeaver](https://github.com/dbeaver/dbeaver).

## Getting started - Docker

To run the example without needing to install any dependencies besides Docker compose:

```bash
cd examples/convert_heic_and_hevc
docker compose build && docker compose run --rm fixmylib
```

The generated files will be on `examples/convert_heic_and_hevc/media-out`.

## Alternatives
[Tdarr](https://github.com/HaveAGitGat/Tdarr) worked great for its purpose of having a way to convert videos to a desired preset. However, I found its UI a little confusing, and it lacked capability to convert photos. 

[Fileflows](https://github.com/revenz/FileFlows) had a better UI, but again, it lacked the feature of converting HEIF photos.
