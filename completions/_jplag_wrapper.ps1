
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'jplag_wrapper' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'jplag_wrapper'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'jplag_wrapper' {
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Specify the config toml file to look for if you don''t want to use the default config.toml')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Specify the config toml file to look for if you don''t want to use the default config.toml')
            [CompletionResult]::new('-s', '-s', [CompletionResultType]::ParameterName, 'Where the input file can be found')
            [CompletionResult]::new('--source-zip', '--source-zip', [CompletionResultType]::ParameterName, 'Where the input file can be found')
            [CompletionResult]::new('-t', '-t', [CompletionResultType]::ParameterName, 'Where to put the results')
            [CompletionResult]::new('--target-dir', '--target-dir', [CompletionResultType]::ParameterName, 'Where to put the results')
            [CompletionResult]::new('--tmp-dir', '--tmp-dir', [CompletionResultType]::ParameterName, 'Where to put the temporary files')
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Where to find the ignore file')
            [CompletionResult]::new('--ignore-file', '--ignore-file', [CompletionResultType]::ParameterName, 'Where to find the ignore file')
            [CompletionResult]::new('-j', '-j', [CompletionResultType]::ParameterName, 'Where the jplag jar can be found')
            [CompletionResult]::new('--jplag-jar', '--jplag-jar', [CompletionResultType]::ParameterName, 'Where the jplag jar can be found')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--init', '--init', [CompletionResultType]::ParameterName, 'Initialize the config, will create (or override!) `config.toml` with all values and fill it with the defaults')
            [CompletionResult]::new('-d', '-d', [CompletionResultType]::ParameterName, 'Set to use log level `debug`')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Set to use log level `debug`')
            [CompletionResult]::new('--remove-non-ascii', '--remove-non-ascii', [CompletionResultType]::ParameterName, 'Remove all non ASCII characters from all submissions')
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'Set to not remove {{tmp_dir}} when the program finishes')
            [CompletionResult]::new('--preserve-tmp-dir', '--preserve-tmp-dir', [CompletionResultType]::ParameterName, 'Set to not remove {{tmp_dir}} when the program finishes')
            [CompletionResult]::new('--ignore-output', '--ignore-output', [CompletionResultType]::ParameterName, 'Set to ignore the output of jplag')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
