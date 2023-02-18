#!/bin/sh
set -e
input="<input-file-full-path>"
temp_output="/tmp/<folder-path><file-stem>.mp4"
output="<folder-path><file-stem>.mp4"

# Do not run ffmpeg for h264 files
if ffprobe "$input" 2>&1 | grep -q h264; then
  echo "File is already in h264 format. Copying as it is..."
  cp -p "$input" "/tmp/<folder-path><file-stem>.<file-extension>"
  touch -r "$input" "/tmp/<folder-path><file-stem>.<file-extension>"
  mv "/tmp/<folder-path><file-stem>.<file-extension>" "<folder-path><file-stem>.<file-extension>"
  exit
fi

ffmpeg -nostdin -y -noautorotate -hwaccel vaapi -hwaccel_device /dev/dri/renderD128 -hwaccel_output_format vaapi -i "$input" -vf 'scale_vaapi=w=iw:h=ih:format=nv12' -c:v h264_vaapi -movflags use_metadata_tags -b:v 10M -maxrate 10M "$temp_output"
exiftool -overwrite_original -TagsFromFile "$input" "-all:all>all:all" "$temp_output"
mv "$temp_output" "$output"
touch -r "$input" "$output"
