#!/bin/sh
set -e
input="<input-file-full-path>"
temp_output="/tmp/<folder-path><file-stem>.jpeg"
output="<folder-path><file-stem>.jpeg"

convert -define jpeg:dct-method=float -sampling-factor 4:2:0 -interlace JPEG -quality "<&jpeg_quality>" "$input" "$temp_output"
mv "$temp_output" "$output"
touch -r "$input" "$output"
