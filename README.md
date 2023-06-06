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
docker compose build && docker compose run --rm fixmylib
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
      - ~/apps/fixmylib/db:/var/lib/postgresql/data
    environment:
      - POSTGRES_PASSWORD=fixmylib

  fixmylib:
    image: ghcr.io/rodrigomideac/fixmylib:latest
    group_add:
      - "109"
      - "989"
    environment:
      - DATABASE_URL=postgresql://postgres:fixmylib@fixmylib-db/postgres
    volumes:
      - ~/apps/fixmylib/media-out:/media-out
      - /path/to/your/media:/media-in
    devices:
      - /dev/dri:/dev/dri

```

Now run it with `docker compose up`.

Any failed conversion can be checked in the Postgres table `file_jobs`. You can use a tool such as [DBeaver](https://github.com/dbeaver/dbeaver) to do so. 

## Project Vision and Roadmap
This project has the aspiration of being a [photoview](https://github.com/photoview/photoview) but with write features. Upcoming features:

- Generate conversion report to make it easier to debug why conversion failed;
- Store media data in the database;
- API to list and serve media;
- Frontend to view the media library;

## Alternatives
[Tdarr](https://github.com/HaveAGitGat/Tdarr) worked great for its purpose of having a way to convert videos to a desired preset. However, I found its UI a little confusing, and it lacked capability to convert photos. 

[Fileflows](https://github.com/revenz/FileFlows) had a better UI, but again, it lacked the feature of converting HEIF photos.
