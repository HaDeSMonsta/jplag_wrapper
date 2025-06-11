# Jplag wrapper

This is a wrapper for the [jplag plagiarism detection tool.](https://github.com/jplag/JPlag)

# Prerequisites

- [jplag v6.0.0](https://github.com/jplag/JPlag/releases/tag/v6.0.0)
- java
- basic command line knowledge
- a zip file with submissions

# Usage

- For the cli usage use `--help`, or `-h` for compact
  - To avoid having to use cli args, those are the defaults
    - `source_zip`: `submissions.zip`
    - `jplag_jar`: `jplag.jar`
    - `tmp_dir`: `tmp/`
    - `out_dir`: `out/`
  - Another way would be to set the config in a `config.toml` file
    - Use `--init` to auto generate one
    - The generated file will have default values set for most variables (more with `--help`)
    - You don't need to set variables you do not wish to override
- Get a jar file from [jplags releases](https://github.com/jplag/JPlag/releases)
  - This is tested with `v6.0.0`
- Get a zip file with submissions
  - We assume exactly one input zip file
  - Which extracts to at zero or more subdirs (zero would be kinda weird, but you do you)
  - Each of those should contain exactly one zip file with the actual work of the student

# Example usage

```shell
jplag_wrapper --source-zip ./submissions.zip --jplag-jar ./jplag.jar --ignore-file ./excludes.txt
```
