# Snail Vim Plugin

A Vim/Neovim plugin for the Snail programming language, providing Tree-sitter-based highlighting (Neovim), code formatting, and filetype support.

## Features

- **Tree-sitter Highlighting** (Neovim): Highlighting for all Snail constructs including:
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

- **Tree-sitter Queries** (Neovim): Highlighting, folding, and indentation via `queries`

## Installation

### Using vim-plug

```vim
Plug 'sudonym1/snail'
```

### Using packer.nvim

```lua
use 'sudonym1/snail'
```

### Using lazy.nvim

```lua
{
  'sudonym1/snail',
  lazy = false, -- optional; start plugin by default
}
```

### Legacy Setup

Older Snail versions required `rtp = 'extras/vim'`. Current versions include
an automatic bootstrap, so no extra runtimepath config is needed.

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

" Tree-sitter highlighting is enabled via nvim-treesitter
```

## Tree-sitter (Neovim only)

Tree-sitter is required for syntax highlighting, code folding, and parsing.
Vim does not provide syntax highlighting for Snail. The plugin auto-registers
the Snail parser with nvim-treesitter when running in Neovim.

### Quick Setup

1. **Install nvim-treesitter** (if not already installed).

2. **Restart Neovim** to load the Snail plugin.

3. **Install the parser:**

   ```vim
   :TSInstall snail
   ```

See `extras/tree-sitter-snail/README.md` for grammar details.

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
├── queries/snail/
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
