#!/usr/bin/env bash

# Get the folders in submissions and unzip them all
#echo "Unpacking zips"

#echo "Files that don't have filetype zip:"
#find submissions ! -name "*.zip" -type f

# Just Copy a oneliner from stackoverflow. What could go wrong???
find . -name "*.zip" | while read filename; do unzip -o -d "`dirname "$filename"`" "$filename"; done;

# Empty results directory
# rm results/*

# Use jplag
#java -jar jplag.jar -s submissions_7/ -r results_7/ -l java19