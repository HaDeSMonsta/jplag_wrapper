
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
            cand -l 'Log Level to use'
            cand --log-level 'Log Level to use'
            cand -c 'Specify the config toml file to look for if you don''t want to use the default config.toml'
            cand --config 'Specify the config toml file to look for if you don''t want to use the default config.toml'
            cand -s 'Where the input file can be found'
            cand --source-zip 'Where the input file can be found'
            cand -t 'Where to put the results'
            cand --target-dir 'Where to put the results'
            cand --tmp-dir 'Where to put the temporary files'
            cand -i 'Where to find the ignore-file'
            cand --ignore-file 'Where to find the ignore-file'
            cand -j 'Where the jplag jar can be found'
            cand --jplag-jar 'Where the jplag jar can be found'
            cand --init 'Initialize the config, will create (or override!) `config.toml` with all values and fill it with the defaults'
            cand --abort-on-err 'Set to abort on any extraction related error'
            cand -p 'Set to not remove `{{tmp_dir}}` when the program finishes'
            cand --preserve-tmp-dir 'Set to not remove `{{tmp_dir}}` when the program finishes'
            cand --ignore-output 'Set to ignore the output of jplag'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand -V 'Print version'
            cand --version 'Print version'
        }
    ]
    $completions[$command]
}
