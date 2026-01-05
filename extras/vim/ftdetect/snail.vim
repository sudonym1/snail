" Snail filetype detection
" Automatically set filetype to snail for .snail files

augroup snail_ftdetect
  autocmd!
  autocmd BufRead,BufNewFile *.snail setfiletype snail
  " Also detect by shebang
  autocmd BufRead,BufNewFile * if getline(1) =~# '^#!.*snail' | setfiletype snail | endif
augroup END