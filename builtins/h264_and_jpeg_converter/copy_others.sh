#!/bin/sh
set -e
input="<input-file-full-path>"
output="<folder-path><file-stem>.<file-extension>"

cp -p "$input" "$output"
touch -r "$input" "$output"
