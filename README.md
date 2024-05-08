# Env vars

Rename env.example to .env, it has all the required env vars
- Required env vars:
  - `TARGET_DIR` => Where to store the result
- Optional env vars:
  - `TEMP_DIR` => Name of the temporary directory to use for unzipped files
  (will be deleted if exists before start of program and at the end)

# Required Layout

There has to be exactly one .zip file in the same dir as the binary.
Also the .env file and the jplag.jar have to be in the same dir.
