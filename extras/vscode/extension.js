"use strict";

const fs = require("fs");
const path = require("path");
const vscode = require("vscode");
const Parser = require("web-tree-sitter");

const tokenTypes = [
  "comment",
  "string",
  "keyword",
  "number",
  "regexp",
  "operator",
  "namespace",
  "class",
  "type",
  "function",
  "method",
  "macro",
  "variable",
  "parameter",
  "property",
];

const tokenModifiers = ["definition", "readonly", "defaultLibrary"];

const tokenTypeIndex = new Map(tokenTypes.map((name, index) => [name, index]));
const tokenModifierIndex = new Map(
  tokenModifiers.map((name, index) => [name, index])
);

const legend = new vscode.SemanticTokensLegend(tokenTypes, tokenModifiers);

const captureMap = new Map([
  ["comment", { type: "comment" }],
  ["string", { type: "string" }],
  ["string.regex", { type: "regexp" }],
  ["string.escape", { type: "string" }],
  ["string.special", { type: "string" }],
  ["embedded", { type: "string" }],
  ["number", { type: "number" }],
  ["boolean", { type: "keyword" }],
  [
    "constant.builtin",
    { type: "variable", modifiers: ["defaultLibrary", "readonly"] },
  ],
  ["keyword", { type: "keyword" }],
  ["keyword.conditional", { type: "keyword" }],
  ["keyword.repeat", { type: "keyword" }],
  ["keyword.exception", { type: "keyword" }],
  ["keyword.function", { type: "keyword" }],
  ["keyword.import", { type: "keyword" }],
  ["keyword.operator", { type: "operator" }],
  ["keyword.directive", { type: "keyword" }],
  ["operator", { type: "operator" }],
  ["operator.pipeline", { type: "operator" }],
  ["operator.special", { type: "operator" }],
  ["function.definition", { type: "function", modifiers: ["definition"] }],
  ["function.call", { type: "function" }],
  [
    "function.method.definition",
    { type: "method", modifiers: ["definition"] },
  ],
  ["function.method.call", { type: "method" }],
  ["type.definition", { type: "class", modifiers: ["definition"] }],
  ["variable.parameter", { type: "parameter" }],
  ["variable.builtin", { type: "variable", modifiers: ["defaultLibrary"] }],
  ["variable", { type: "variable" }],
  ["module", { type: "namespace" }],
  ["function.macro", { type: "macro" }],
  ["property", { type: "property" }],
]);

const capturePriority = new Map([
  ["function.method.definition", 300],
  ["function.definition", 260],
  ["type.definition", 250],
  ["function.method.call", 230],
  ["function.call", 220],
  ["variable.parameter", 210],
  ["variable.builtin", 200],
  ["constant.builtin", 200],
  ["module", 190],
  ["variable", 10],
]);

const parserState = {
  ready: null,
  language: null,
  query: null,
  initError: null,
};

async function ensureParser(context) {
  if (parserState.ready) {
    return parserState.ready;
  }

  parserState.ready = (async () => {
    const runtimePath = context.asAbsolutePath(
      path.join("node_modules", "web-tree-sitter", "tree-sitter.wasm")
    );
    await Parser.init({
      locateFile() {
        return runtimePath;
      },
    });

    const languagePath = context.asAbsolutePath(
      path.join("assets", "tree-sitter-snail.wasm")
    );
    if (!fs.existsSync(languagePath)) {
      throw new Error(
        "Missing Tree-sitter wasm: build extras/vscode/assets/tree-sitter-snail.wasm"
      );
    }
    parserState.language = await Parser.Language.load(languagePath);

    const queryPath = context.asAbsolutePath(
      path.join("queries", "highlights.scm")
    );
    if (!fs.existsSync(queryPath)) {
      throw new Error("Missing highlight queries at extras/vscode/queries");
    }
    const querySource = fs.readFileSync(queryPath, "utf8");
    parserState.query = parserState.language.query(querySource);
  })().catch((err) => {
    parserState.ready = null;
    parserState.initError = err;
    throw err;
  });

  return parserState.ready;
}

function mapCapture(name) {
  if (captureMap.has(name)) {
    return captureMap.get(name);
  }
  if (name.startsWith("keyword.")) {
    return { type: "keyword" };
  }
  if (name.startsWith("string.")) {
    return { type: "string" };
  }
  if (name.startsWith("operator.")) {
    return { type: "operator" };
  }
  if (name.startsWith("punctuation.")) {
    return null;
  }
  return null;
}

function modifierBits(modifiers) {
  let bits = 0;
  for (const modifier of modifiers || []) {
    const index = tokenModifierIndex.get(modifier);
    if (typeof index === "number") {
      bits |= 1 << index;
    }
  }
  return bits;
}

function pushToken(builder, document, token) {
  const typeIndex = tokenTypeIndex.get(token.type);
  if (typeof typeIndex !== "number") {
    return;
  }

  const modifiers = modifierBits(token.modifiers);
  const start = token.start;
  const end = token.end;

  if (start.row === end.row) {
    const length = end.column - start.column;
    if (length > 0) {
      builder.push(start.row, start.column, length, typeIndex, modifiers);
    }
    return;
  }

  for (let line = start.row; line <= end.row; line += 1) {
    const lineText = document.lineAt(line).text;
    const lineStart = line === start.row ? start.column : 0;
    const lineEnd = line === end.row ? end.column : lineText.length;
    const length = lineEnd - lineStart;
    if (length > 0) {
      builder.push(line, lineStart, length, typeIndex, modifiers);
    }
  }
}

class SnailSemanticTokensProvider {
  constructor(context) {
    this.context = context;
  }

  async provideDocumentSemanticTokens(document, token) {
    try {
      await ensureParser(this.context);
    } catch (err) {
      if (!parserState.initError) {
        parserState.initError = err;
      }
      return new vscode.SemanticTokensBuilder(legend).build();
    }

    if (!parserState.language || !parserState.query) {
      return new vscode.SemanticTokensBuilder(legend).build();
    }

    const parser = new Parser();
    parser.setLanguage(parserState.language);

    const tree = parser.parse(document.getText());
    const captures = parserState.query.captures(tree.rootNode);

    const tokensByRange = new Map();

    for (const capture of captures) {
      if (token && token.isCancellationRequested) {
        return new vscode.SemanticTokensBuilder(legend).build();
      }

      const mapping = mapCapture(capture.name);
      if (!mapping) {
        continue;
      }

      const node = capture.node;
      if (!node || node.startIndex === node.endIndex) {
        continue;
      }

      const key = `${node.startIndex}:${node.endIndex}`;
      const priority = capturePriority.get(capture.name) || 0;
      const existing = tokensByRange.get(key);
      if (!existing || priority > existing.priority) {
        tokensByRange.set(key, {
          start: node.startPosition,
          end: node.endPosition,
          type: mapping.type,
          modifiers: mapping.modifiers || [],
          priority,
        });
      }
    }

    const tokens = Array.from(tokensByRange.values()).sort((a, b) => {
      if (a.start.row !== b.start.row) {
        return a.start.row - b.start.row;
      }
      return a.start.column - b.start.column;
    });

    const builder = new vscode.SemanticTokensBuilder(legend);
    for (const tokenItem of tokens) {
      pushToken(builder, document, tokenItem);
    }

    return builder.build();
  }
}

async function activate(context) {
  const provider = new SnailSemanticTokensProvider(context);
  context.subscriptions.push(
    vscode.languages.registerDocumentSemanticTokensProvider(
      { language: "snail" },
      provider,
      legend
    )
  );
}

function deactivate() {}

module.exports = {
  activate,
  deactivate,
};
