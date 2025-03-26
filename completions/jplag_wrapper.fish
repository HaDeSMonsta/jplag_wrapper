complete -c jplag_wrapper -s l -l log-level -d 'Log Level to use' -r
complete -c jplag_wrapper -s c -l config -d 'Specify the config toml file to look for if you don\'t want to use the default config.toml' -r
complete -c jplag_wrapper -s s -l source-zip -d 'Where the input file can be found' -r
complete -c jplag_wrapper -s t -l target-dir -d 'Where to put the results' -r
complete -c jplag_wrapper -l tmp-dir -d 'Where to put the temporary files' -r
complete -c jplag_wrapper -s i -l ignore-file -d 'Where to find the ignore file' -r
complete -c jplag_wrapper -s j -l jplag-jar -d 'Where the jplag jar can be found' -r
complete -c jplag_wrapper -l init -d 'Initialize the config, will create (or override!) `config.toml` with all values and fill it with the defaults'
complete -c jplag_wrapper -l keep-non-ascii -d 'Keep all non ASCII characters from all submissions'
complete -c jplag_wrapper -s p -l preserve-tmp-dir -d 'Set to not remove {{tmp_dir}} when the program finishes'
complete -c jplag_wrapper -l ignore-output -d 'Set to ignore the output of jplag'
complete -c jplag_wrapper -s h -l help -d 'Print help (see more with \'--help\')'
complete -c jplag_wrapper -s V -l version -d 'Print version'
