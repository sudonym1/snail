#compdef snail

# Zsh completion for snail
_arguments \
  '-h[show help]' \
  '--help[show help]' \
  '-c[run a one-liner]:code' \
  '-p[output translated Python and exit]' \
  '--python[output translated Python and exit]' \
  '-a[run code in awk mode]' \
  '--awk[run code in awk mode]' \
  '--[treat following arguments as program argv]' \
  '*:file:_files'
