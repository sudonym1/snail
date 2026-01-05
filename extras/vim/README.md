# Snail Vim Plugin

A comprehensive Vim/Neovim plugin for the Snail programming language, providing syntax highlighting, code formatting, and Tree-sitter integration.

## Features

- **Syntax Highlighting**: Complete highlighting for all Snail constructs including:
  - Keywords, operators, and literals
  - Subprocess syntax: `$()` and `@()`
  - Compact try operator: `?`
  - Regex literals: `/pattern/`
  - String interpolation: `{expr}`
  - AWK mode: `BEGIN`, `END`, special variables
  - Pipeline operator: `|`
  - Structured accessor: `$[query]`

- **Code Formatting**: Automatic brace-based indentation and formatting

- **Filetype Detection**: Automatic detection of `.snail` files

- **Commands**:
  - `:SnailFormat` - Format the current buffer
  - `:SnailRun` - Execute the current Snail file
  - `:SnailShowPython` - View generated Python code

- **Tree-sitter Support** (Neovim): Advanced syntax highlighting and code analysis

## Installation

### Using vim-plug

```vim
Plug 'sudonym1/snail', { 'rtp': 'extras/vim' }
```

### Using packer.nvim

```lua
use { 'sudonym1/snail', rtp = 'extras/vim' }
```

### Using lazy.nvim

```lua
{
  'sudonym1/snail',
  config = function()
    vim.opt.rtp:append('extras/vim')
  end
}
```

### Manual Installation

Copy the contents of this directory to your Vim runtime:

```bash
# For Vim
cp -r extras/vim/* ~/.vim/

# For Neovim
cp -r extras/vim/* ~/.config/nvim/
```

## Configuration

Add to your vimrc/init.vim:

```vim
" Enable format on save
let g:snail_format_on_save = 1

" Enable AWK variable highlighting (default: 1)
let g:snail_highlight_awk_vars = 1

" Enable string interpolation highlighting (default: 1)
let g:snail_highlight_interpolation = 1
```

## Tree-sitter (Neovim only)

For enhanced parsing, build and configure the Tree-sitter grammar:

```bash
cd extras/tree-sitter-snail
npm install
npm run build
```

Then configure nvim-treesitter to use the parser.

## Directory Structure

```
extras/vim/
├── autoload/
│   └── snail.vim       # Autoload functions (format, run, complete)
├── doc/
│   └── snail.txt       # Vim help documentation
├── ftdetect/
│   └── snail.vim       # Filetype detection
├── ftplugin/
│   └── snail.vim       # Filetype-specific settings
├── indent/
│   └── snail.vim       # Indentation rules
├── plugin/
│   └── snail.vim       # Main plugin configuration
├── syntax/
│   └── snail.vim       # Syntax highlighting
├── after/queries/snail/
│   ├── highlights.scm  # Tree-sitter highlights
│   ├── folds.scm       # Tree-sitter folds
│   └── indents.scm     # Tree-sitter indents
└── README.md           # This file
```

## Usage

Open a `.snail` file and the plugin will automatically activate. Use:

- `gq` to format the buffer or selection
- `:SnailRun` to execute the current file
- `:SnailShowPython` to see the compiled Python
- `:help snail` for full documentation

## License

MIT License - see the main Snail repository for details.

