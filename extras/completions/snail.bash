# Bash completion for snail

_snail_complete() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"

    opts="-h --help -c -p --python -a --awk --"

    if [[ "${prev}" == "-c" ]]; then
        # After -c we accept inline code, so fall back to default completion
        return 0
    fi

    if [[ "${prev}" == "--" ]]; then
        COMPREPLY=( $(compgen -f -- "${cur}") )
        return 0
    fi

    if [[ "${cur}" == -* ]]; then
        COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
        return 0
    fi

    COMPREPLY=( $(compgen -f -- "${cur}") )
}

complete -F _snail_complete snail
