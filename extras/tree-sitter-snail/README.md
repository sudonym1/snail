# tree-sitter-snail

Tree-sitter grammar for the Snail programming language.

## Features

- Full Snail syntax support including:
  - String interpolation with `{expr}`
  - Subprocess syntax: `$(cmd)` and `@(cmd)`
  - Regex literals: `/pattern/`
  - Compact try operator: `expr?` and `expr:fallback?`
  - AWK mode with special variables (`$0`, `$<num>`, `$n`, etc.)
  - Pipeline operator: `|`
  - Structured accessors: `$[query]`
- Accurate syntax highlighting
  - Code folding support
  - Indentation support

## Installation

### For Neovim with nvim-treesitter

1. **Option A: Local parser (recommended for development)**

   Add this to your Neovim configuration:

   ```lua
   local function register_snail_parser()
     local parser_dir = vim.fn.expand("~/path/to/snail/extras/tree-sitter-snail")
     local parsers = require("nvim-treesitter.parsers")
     local parser_configs = type(parsers.get_parser_configs) == "function"
       and parsers.get_parser_configs() or parsers
     local snail_parser = parser_configs.snail or parsers.snail

     if not snail_parser then
       snail_parser = {
         install_info = {
           path = parser_dir,
           url = parser_dir, -- compatibility fallback for older nvim-treesitter
           files = { "src/parser.c" },
           branch = "main",
           generate_requires_npm = false,
           requires_generate_from_grammar = false,
         },
         filetype = "snail",
       }
     end

     parser_configs.snail = snail_parser
     parsers.snail = snail_parser
   end

   register_snail_parser()
   vim.api.nvim_create_autocmd("User", {
     pattern = "TSUpdate",
     callback = register_snail_parser,
   })
   ```

   Then run `:TSInstall snail` in Neovim.

2. **Option B: Manual compilation**

   ```bash
   cd extras/tree-sitter-snail

   # Compile the parser to a shared library
   cc -o parser.so -I./src src/parser.c -shared -Os -fPIC

   # Copy to Neovim's parser directory
   mkdir -p ~/.local/share/nvim/site/parser
   cp parser.so ~/.local/share/nvim/site/parser/snail.so
   ```

3. **Install the Snail Neovim plugin**

   ```lua
   -- Using lazy.nvim (preferred)
   {
     'sudonym1/snail',
     lazy = false, -- optional
   }
   ```

   Or with vim-plug:

   ```vim
   Plug 'sudonym1/snail'
   ```

   Then run `:TSInstall snail` in Neovim.

### For other editors

Check your editor's tree-sitter integration documentation. The parser can be built with:

```bash
cd extras/tree-sitter-snail
npm install    # Install dependencies
npm run build  # Generate parser from grammar.js
```

## Development

### Regenerating the parser

If you modify `grammar.js`, regenerate the parser:

```bash
# Using npm scripts
npm run build

# Or directly with tree-sitter CLI
tree-sitter generate
```

### Testing

```bash
# Test the grammar on a file
tree-sitter parse /path/to/file.snail

# Run tests (if available)
tree-sitter test
```

### Grammar structure

The grammar is defined in `grammar.js` and closely mirrors the Pest grammar at `crates/snail-parser/src/snail.pest`. Key differences:

- Tree-sitter uses JavaScript DSL instead of Pest's PEG syntax
- Explicit conflict resolution for ambiguous patterns
- Optimized for incremental parsing and error recovery

## Highlight queries

Tree-sitter highlight queries are in `queries/highlights.scm` and are also copied to `extras/vim/queries/snail/highlights.scm` for Neovim integration.

## License

MIT - See the main Snail repository for details.
