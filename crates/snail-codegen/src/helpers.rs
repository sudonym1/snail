use snail_lower::*;

// Vendored jmespath library
const JMESPATH_EXCEPTIONS: &str = include_str!("../../../vendored/jmespath/exceptions.py");
const JMESPATH_COMPAT: &str = include_str!("../../../vendored/jmespath/compat.py");
const JMESPATH_AST: &str = include_str!("../../../vendored/jmespath/ast.py");
const JMESPATH_LEXER: &str = include_str!("../../../vendored/jmespath/lexer.py");
const JMESPATH_FUNCTIONS: &str = include_str!("../../../vendored/jmespath/functions.py");
const JMESPATH_VISITOR: &str = include_str!("../../../vendored/jmespath/visitor.py");
const JMESPATH_PARSER: &str = include_str!("../../../vendored/jmespath/parser.py");
const JMESPATH_INIT: &str = include_str!("../../../vendored/jmespath/__init__.py");

/// Helper trait for types that can write helper code
pub trait HelperWriter {
    fn write_line(&mut self, line: &str);
    fn indent(&self) -> usize;
    fn set_indent(&mut self, indent: usize);
}

/// Write the snail try helper function
pub fn write_snail_try_helper<W: HelperWriter>(writer: &mut W) {
    writer.write_line(&format!(
        "def {}(expr_fn, fallback_fn=None):",
        SNAIL_TRY_HELPER
    ));
    writer.set_indent(writer.indent() + 1);
    writer.write_line("try:");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("return expr_fn()");
    writer.set_indent(writer.indent() - 1);
    writer.write_line(&format!("except Exception as {}:", SNAIL_EXCEPTION_VAR));
    writer.set_indent(writer.indent() + 1);
    writer.write_line("if fallback_fn is None:");
    writer.set_indent(writer.indent() + 1);
    writer.write_line(&format!(
        "fallback_member = getattr({}, \"__fallback__\", None)",
        SNAIL_EXCEPTION_VAR
    ));
    writer.write_line("if callable(fallback_member):");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("return fallback_member()");
    writer.set_indent(writer.indent() - 1);
    writer.write_line(&format!("return {}", SNAIL_EXCEPTION_VAR));
    writer.set_indent(writer.indent() - 1);
    writer.write_line(&format!("return fallback_fn({})", SNAIL_EXCEPTION_VAR));
    writer.set_indent(writer.indent() - 2);
}

/// Write the snail regex helper functions
pub fn write_snail_regex_helpers<W: HelperWriter>(writer: &mut W) {
    writer.write_line("import re");
    writer.write_line("");
    writer.write_line(&format!("def {}(value, pattern):", SNAIL_REGEX_SEARCH));
    writer.set_indent(writer.indent() + 1);
    writer.write_line("return re.search(pattern, value)");
    writer.set_indent(writer.indent() - 1);
    writer.write_line("");
    writer.write_line(&format!("def {}(pattern):", SNAIL_REGEX_COMPILE));
    writer.set_indent(writer.indent() + 1);
    writer.write_line("return re.compile(pattern)");
    writer.set_indent(writer.indent() - 1);
}

/// Write the snail subprocess helper classes
pub fn write_snail_subprocess_helpers<W: HelperWriter>(writer: &mut W) {
    writer.write_line("import subprocess");
    writer.write_line("");

    // Write __SnailSubprocessCapture class
    writer.write_line(&format!("class {}:", SNAIL_SUBPROCESS_CAPTURE_CLASS));
    writer.set_indent(writer.indent() + 1);
    writer.write_line("def __init__(self, cmd):");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("self.cmd = cmd");
    writer.set_indent(writer.indent() - 1);
    writer.write_line("");
    writer.write_line("def __pipeline__(self, input_data):");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("try:");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("if input_data is None:");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("# No stdin - run normally");
    writer.write_line("completed = subprocess.run(self.cmd, shell=True, check=True, text=True, stdout=subprocess.PIPE)");
    writer.set_indent(writer.indent() - 1);
    writer.write_line("else:");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("# Pipe input to stdin");
    writer.write_line("if not isinstance(input_data, (str, bytes)):");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("input_data = str(input_data)");
    writer.set_indent(writer.indent() - 1);
    writer.write_line("completed = subprocess.run(self.cmd, shell=True, check=True, text=True, input=input_data, stdout=subprocess.PIPE)");
    writer.set_indent(writer.indent() - 1);
    writer.write_line("return completed.stdout.rstrip('\\n')");
    writer.set_indent(writer.indent() - 1);
    writer.write_line("except subprocess.CalledProcessError as exc:");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("def __fallback(exc=exc):");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("raise exc");
    writer.set_indent(writer.indent() - 1);
    writer.write_line("exc.__fallback__ = __fallback");
    writer.write_line("raise");
    writer.set_indent(writer.indent() - 3);
    writer.write_line("");

    // Write __SnailSubprocessStatus class
    writer.write_line(&format!("class {}:", SNAIL_SUBPROCESS_STATUS_CLASS));
    writer.set_indent(writer.indent() + 1);
    writer.write_line("def __init__(self, cmd):");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("self.cmd = cmd");
    writer.set_indent(writer.indent() - 1);
    writer.write_line("");
    writer.write_line("def __pipeline__(self, input_data):");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("try:");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("if input_data is None:");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("# No stdin - run normally");
    writer.write_line("subprocess.run(self.cmd, shell=True, check=True)");
    writer.set_indent(writer.indent() - 1);
    writer.write_line("else:");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("# Pipe input to stdin");
    writer.write_line("if not isinstance(input_data, (str, bytes)):");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("input_data = str(input_data)");
    writer.set_indent(writer.indent() - 1);
    writer.write_line(
        "subprocess.run(self.cmd, shell=True, check=True, text=True, input=input_data)",
    );
    writer.set_indent(writer.indent() - 1);
    writer.write_line("return 0");
    writer.set_indent(writer.indent() - 1);
    writer.write_line("except subprocess.CalledProcessError as exc:");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("def __fallback(exc=exc):");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("return exc.returncode");
    writer.set_indent(writer.indent() - 1);
    writer.write_line("exc.__fallback__ = __fallback");
    writer.write_line("raise");
    writer.set_indent(writer.indent() - 3);
}

/// Write the vendored jmespath library
pub fn write_vendored_jmespath<W: HelperWriter>(writer: &mut W) {
    // Helper to escape Python source for embedding in a string
    fn escape_py_source(source: &str) -> String {
        source
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    }

    writer.write_line("# Vendored jmespath library (embedded to avoid external dependency)");
    writer.write_line("import sys");
    writer.write_line("if 'jmespath' not in sys.modules:");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("import types");
    writer.write_line("");

    // Create jmespath package
    writer.write_line("__jmespath = types.ModuleType('jmespath')");
    writer.write_line("__jmespath.__package__ = 'jmespath'");
    writer.write_line("__jmespath.__path__ = []");
    writer.write_line("sys.modules['jmespath'] = __jmespath");
    writer.write_line("");

    // Inject each submodule using compile+exec (in dependency order)
    writer.write_line("# Inject jmespath.compat (base module)");
    writer.write_line("__mod = types.ModuleType('jmespath.compat')");
    writer.write_line("__mod.__package__ = 'jmespath'");
    writer.write_line(&format!(
        "exec(compile(\"{}\", 'jmespath/compat.py', 'exec'), __mod.__dict__)",
        escape_py_source(JMESPATH_COMPAT)
    ));
    writer.write_line("sys.modules['jmespath.compat'] = __mod");
    writer.write_line("__jmespath.compat = __mod");
    writer.write_line("");

    writer.write_line("# Inject jmespath.exceptions");
    writer.write_line("__mod = types.ModuleType('jmespath.exceptions')");
    writer.write_line("__mod.__package__ = 'jmespath'");
    writer.write_line(&format!(
        "exec(compile(\"{}\", 'jmespath/exceptions.py', 'exec'), __mod.__dict__)",
        escape_py_source(JMESPATH_EXCEPTIONS)
    ));
    writer.write_line("sys.modules['jmespath.exceptions'] = __mod");
    writer.write_line("__jmespath.exceptions = __mod");
    writer.write_line("");

    writer.write_line("# Inject jmespath.ast");
    writer.write_line("__mod = types.ModuleType('jmespath.ast')");
    writer.write_line("__mod.__package__ = 'jmespath'");
    writer.write_line(&format!(
        "exec(compile(\"{}\", 'jmespath/ast.py', 'exec'), __mod.__dict__)",
        escape_py_source(JMESPATH_AST)
    ));
    writer.write_line("sys.modules['jmespath.ast'] = __mod");
    writer.write_line("__jmespath.ast = __mod");
    writer.write_line("");

    writer.write_line("# Inject jmespath.lexer");
    writer.write_line("__mod = types.ModuleType('jmespath.lexer')");
    writer.write_line("__mod.__package__ = 'jmespath'");
    writer.write_line(&format!(
        "exec(compile(\"{}\", 'jmespath/lexer.py', 'exec'), __mod.__dict__)",
        escape_py_source(JMESPATH_LEXER)
    ));
    writer.write_line("sys.modules['jmespath.lexer'] = __mod");
    writer.write_line("__jmespath.lexer = __mod");
    writer.write_line("");

    writer.write_line("# Inject jmespath.functions");
    writer.write_line("__mod = types.ModuleType('jmespath.functions')");
    writer.write_line("__mod.__package__ = 'jmespath'");
    writer.write_line(&format!(
        "exec(compile(\"{}\", 'jmespath/functions.py', 'exec'), __mod.__dict__)",
        escape_py_source(JMESPATH_FUNCTIONS)
    ));
    writer.write_line("sys.modules['jmespath.functions'] = __mod");
    writer.write_line("__jmespath.functions = __mod");
    writer.write_line("");

    writer.write_line("# Inject jmespath.visitor");
    writer.write_line("__mod = types.ModuleType('jmespath.visitor')");
    writer.write_line("__mod.__package__ = 'jmespath'");
    writer.write_line(&format!(
        "exec(compile(\"{}\", 'jmespath/visitor.py', 'exec'), __mod.__dict__)",
        escape_py_source(JMESPATH_VISITOR)
    ));
    writer.write_line("sys.modules['jmespath.visitor'] = __mod");
    writer.write_line("__jmespath.visitor = __mod");
    writer.write_line("");

    writer.write_line("# Inject jmespath.parser");
    writer.write_line("__mod = types.ModuleType('jmespath.parser')");
    writer.write_line("__mod.__package__ = 'jmespath'");
    writer.write_line(&format!(
        "exec(compile(\"{}\", 'jmespath/parser.py', 'exec'), __mod.__dict__)",
        escape_py_source(JMESPATH_PARSER)
    ));
    writer.write_line("sys.modules['jmespath.parser'] = __mod");
    writer.write_line("__jmespath.parser = __mod");
    writer.write_line("");

    writer.write_line("# Inject jmespath main module");
    writer.write_line(&format!(
        "exec(compile(\"{}\", 'jmespath/__init__.py', 'exec'), __jmespath.__dict__)",
        escape_py_source(JMESPATH_INIT)
    ));
    writer.write_line("");

    writer.set_indent(writer.indent() - 1);
    writer.write_line("");
}

/// Write the structured accessor helper classes and functions
pub fn write_structured_accessor_helpers<W: HelperWriter>(writer: &mut W) {
    write_vendored_jmespath(writer);
    writer.write_line("import jmespath");
    writer.write_line("import json as _json");
    writer.write_line("import sys as _sys");
    writer.write_line("");

    // Write __SnailStructuredAccessor class
    writer.write_line(&format!("class {}:", SNAIL_STRUCTURED_ACCESSOR_CLASS));
    writer.set_indent(writer.indent() + 1);
    writer.write_line("def __init__(self, query):");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("self.query = query");
    writer.set_indent(writer.indent() - 1);
    writer.write_line("");
    writer.write_line("def __pipeline__(self, obj):");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("if not hasattr(obj, '__structured__'):");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("raise TypeError(f\"Pipeline target must implement __structured__, got {type(obj).__name__}\")");
    writer.set_indent(writer.indent() - 1);
    writer.write_line("return obj.__structured__(self.query)");
    writer.set_indent(writer.indent() - 2);
    writer.write_line("");

    // Write __SnailJsonObject class
    writer.write_line(&format!("class {}:", SNAIL_JSON_OBJECT_CLASS));
    writer.set_indent(writer.indent() + 1);
    writer.write_line("def __init__(self, data):");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("self.data = data");
    writer.set_indent(writer.indent() - 1);
    writer.write_line("");
    writer.write_line("def __structured__(self, query):");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("return jmespath.search(query, self.data)");
    writer.set_indent(writer.indent() - 1);
    writer.write_line("");
    writer.write_line("def __repr__(self):");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("return _json.dumps(self.data, indent=2)");
    writer.set_indent(writer.indent() - 2);
    writer.write_line("");

    // Write __SnailJsonPipelineWrapper class
    writer.write_line(&format!("class {}:", SNAIL_JSON_PIPELINE_WRAPPER_CLASS));
    writer.set_indent(writer.indent() + 1);
    writer.write_line(
        "\"\"\"Wrapper for json() to support pipeline operator without blocking stdin.\"\"\"",
    );
    writer.write_line("");
    writer.write_line("def __pipeline__(self, input):");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("\"\"\"Called when used in a pipeline: x | json()\"\"\"");
    writer.write_line("return json(input)");
    writer.set_indent(writer.indent() - 1);
    writer.write_line("");
    writer.write_line("def __structured__(self, query):");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("\"\"\"Called when used with structured accessor: json() | $[query]\"\"\"");
    writer.write_line("data = json(_sys.stdin)");
    writer.write_line("return data.__structured__(query)");
    writer.set_indent(writer.indent() - 1);
    writer.write_line("");
    writer.write_line("def __repr__(self):");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("\"\"\"When printed, consume stdin and display parsed JSON.\"\"\"");
    writer.write_line("data = json(_sys.stdin)");
    writer.write_line("return repr(data)");
    writer.set_indent(writer.indent() - 2);
    writer.write_line("");

    // Write json() function
    writer.write_line("def json(input=None):");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("\"\"\"Parse JSON from various input sources.\"\"\"");
    writer.write_line("# Return wrapper when called with no arguments for pipeline support");
    writer.write_line("if input is None:");
    writer.set_indent(writer.indent() + 1);
    writer.write_line(&format!("return {}()", SNAIL_JSON_PIPELINE_WRAPPER_CLASS));
    writer.set_indent(writer.indent() - 1);
    writer.write_line("");
    writer.write_line("# Handle different input types");
    writer.write_line("if isinstance(input, str):");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("# Try parsing as JSON string first");
    writer.write_line("try:");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("data = _json.loads(input)");
    writer.set_indent(writer.indent() - 1);
    writer.write_line("except _json.JSONDecodeError:");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("# Fall back to file path");
    writer.write_line("with open(input, 'r') as f:");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("data = _json.load(f)");
    writer.set_indent(writer.indent() - 3);
    writer.write_line("elif hasattr(input, 'read'):");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("# File-like object (including sys.stdin)");
    writer.write_line("content = input.read()");
    writer.write_line("if isinstance(content, bytes):");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("content = content.decode('utf-8')");
    writer.set_indent(writer.indent() - 1);
    writer.write_line("data = _json.loads(content)");
    writer.set_indent(writer.indent() - 1);
    writer.write_line("elif isinstance(input, (dict, list, int, float, bool, type(None))):");
    writer.set_indent(writer.indent() + 1);
    writer.write_line("# Already JSON-native type");
    writer.write_line("data = input");
    writer.set_indent(writer.indent() - 1);
    writer.write_line("else:");
    writer.set_indent(writer.indent() + 1);
    writer.write_line(
        "raise TypeError(f\"json() input must be JSON-compatible, got {type(input).__name__}\")",
    );
    writer.set_indent(writer.indent() - 1);
    writer.write_line("");
    writer.write_line(&format!("return {}(data)", SNAIL_JSON_OBJECT_CLASS));
    writer.set_indent(writer.indent() - 1);
}
