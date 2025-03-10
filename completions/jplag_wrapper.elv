
use builtin;
use str;

set edit:completion:arg-completer[jplag_wrapper] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'jplag_wrapper'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'jplag_wrapper'= {
            cand -c 'Specify the config toml file to look for if you don''t want to use the default config.toml'
            cand --config 'Specify the config toml file to look for if you don''t want to use the default config.toml'
            cand -s 'Where the input file can be found'
            cand --source-zip 'Where the input file can be found'
            cand -t 'Where to put the results'
            cand --target-dir 'Where to put the results'
            cand --tmp-dir 'Where to put the temporary files'
            cand -i 'Where to find the ignore file'
            cand --ignore-file 'Where to find the ignore file'
            cand -j 'Where the jplag jar can be found'
            cand --jplag-jar 'Where the jplag jar can be found'
            cand -v 'Print version'
            cand --version 'Print version'
            cand --init 'Initialize the config, will create (or override!) `config.toml` with all values and fill it with the defaults'
            cand -d 'Set to use log level `debug`'
            cand --debug 'Set to use log level `debug`'
            cand --keep-non-ascii 'Keep all non ASCII characters from all submissions'
            cand -p 'Set to not remove {{tmp_dir}} when the program finishes'
            cand --preserve-tmp-dir 'Set to not remove {{tmp_dir}} when the program finishes'
            cand --ignore-output 'Set to ignore the output of jplag'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
    ]
    $completions[$command]
}
