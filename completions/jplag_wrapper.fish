# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_jplag_wrapper_global_optspecs
	string join \n init l/log-level= abort-on-err c/config= s/source-zip= t/target-dir= tmp-dir= p/preserve-tmp-dir i/ignore-file= ignore-output j/jplag-jar= h/help V/version
end

function __fish_jplag_wrapper_needs_command
	# Figure out if the current invocation already has a command.
	set -l cmd (commandline -opc)
	set -e cmd[1]
	argparse -s (__fish_jplag_wrapper_global_optspecs) -- $cmd 2>/dev/null
	or return
	if set -q argv[1]
		# Also print the command, so this can be used to figure out what it is.
		echo $argv[1]
		return 1
	end
	return 0
end

function __fish_jplag_wrapper_using_subcommand
	set -l cmd (__fish_jplag_wrapper_needs_command)
	test -z "$cmd"
	and return 1
	contains -- $cmd[1] $argv
end

complete -c jplag_wrapper -n "__fish_jplag_wrapper_needs_command" -s l -l log-level -d 'Log Level to use' -r
complete -c jplag_wrapper -n "__fish_jplag_wrapper_needs_command" -s c -l config -d 'Specify the config toml file to look for if you don\'t want to use the default config.toml' -r
complete -c jplag_wrapper -n "__fish_jplag_wrapper_needs_command" -s s -l source-zip -d 'Where the input file can be found' -r
complete -c jplag_wrapper -n "__fish_jplag_wrapper_needs_command" -s t -l target-dir -d 'Where to put the results' -r
complete -c jplag_wrapper -n "__fish_jplag_wrapper_needs_command" -l tmp-dir -d 'Where to put the temporary files' -r
complete -c jplag_wrapper -n "__fish_jplag_wrapper_needs_command" -s i -l ignore-file -d 'Where to find the ignore-file' -r
complete -c jplag_wrapper -n "__fish_jplag_wrapper_needs_command" -s j -l jplag-jar -d 'Where the jplag jar can be found' -r
complete -c jplag_wrapper -n "__fish_jplag_wrapper_needs_command" -l init -d 'Initialize the config, will create (or override!) `config.toml` with all values and fill it with the defaults'
complete -c jplag_wrapper -n "__fish_jplag_wrapper_needs_command" -l abort-on-err -d 'Set to abort on any extraction related error'
complete -c jplag_wrapper -n "__fish_jplag_wrapper_needs_command" -s p -l preserve-tmp-dir -d 'Set to not remove `{{tmp_dir}}` when the program finishes'
complete -c jplag_wrapper -n "__fish_jplag_wrapper_needs_command" -l ignore-output -d 'Set to ignore the output of jplag'
complete -c jplag_wrapper -n "__fish_jplag_wrapper_needs_command" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c jplag_wrapper -n "__fish_jplag_wrapper_needs_command" -s V -l version -d 'Print version'
complete -c jplag_wrapper -n "__fish_jplag_wrapper_needs_command" -a "complete"
complete -c jplag_wrapper -n "__fish_jplag_wrapper_needs_command" -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c jplag_wrapper -n "__fish_jplag_wrapper_using_subcommand complete" -s h -l help -d 'Print help'
complete -c jplag_wrapper -n "__fish_jplag_wrapper_using_subcommand help; and not __fish_seen_subcommand_from complete help" -f -a "complete"
complete -c jplag_wrapper -n "__fish_jplag_wrapper_using_subcommand help; and not __fish_seen_subcommand_from complete help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
