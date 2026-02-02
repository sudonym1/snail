# Snail VS Code Extension (Tree-sitter)

This extension provides Tree-sitter based semantic highlighting for Snail in VS Code.

## Setup (local repo)

1. Install dependencies:

```bash
cd extras/vscode
npm install
```

2. Build the Tree-sitter WASM parser (requires `tree-sitter` CLI and Emscripten or Docker):

```bash
npm run build-wasm
```

If you want to force Docker for Emscripten:

```bash
SNAIL_TS_DOCKER=1 npm run build-wasm
```

3. Open the repo in VS Code and run the extension:

- Open the Command Palette: **Developer: Reload Window**
- Or launch from the Run and Debug view: **Run Extension**

Snail files (`.snail`) should now use Tree-sitter semantic highlighting.

## Regenerating highlights

This extension uses the Tree-sitter query file at `extras/vscode/queries/highlights.scm`.
Keep it in sync with `extras/tree-sitter-snail/queries/highlights.scm` when updating highlights.
