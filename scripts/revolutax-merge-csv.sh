#!/bin/bash

# Create the "merged" directory if it doesn't exist
mkdir -p merged

# Create a new file called "merged.csv" in the "merged" directory to hold the combined data
echo "Type,Product,Started Date,Completed Date,Description,Amount,Currency,Fiat amount,Fiat amount (inc. fees),Fee,Base currency,State,Balance" > merged/merged.csv

# Loop through all CSV files in the current directory
for file in *.csv; do
  # Skip the "merged.csv" file itself
  if [[ "$file" == "merged.csv" ]]; then
    continue
  fi
  
  # Append all lines except the first (header) line to the "merged.csv" file
  tail -n +2 "$file" >> merged/merged.csv
done

