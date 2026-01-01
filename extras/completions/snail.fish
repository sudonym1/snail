# Fish completion for snail
complete -c snail -s h -l help -d "Show help"
complete -c snail -s c -d "Run a one-liner" -r
complete -c snail -s p -l python -d "Output translated Python and exit"
complete -c snail -s a -l awk -d "Run code in awk mode"
# -- marks the end of flags and the start of passthrough argv
complete -c snail -l -- -d "Treat following arguments as program argv"
# Files are positional after options
complete -c snail -f -a "(printf '%s\n' *(N))"
