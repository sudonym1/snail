use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use tempfile::TempDir;

use snail::{exec_snail, register_in_python};

#[test]
fn executes_snail_code_via_python_api() {
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let module = register_in_python(py).expect("snail module should register");
        let locals = PyDict::new_bound(py);
        locals.set_item("snail_module", &module).unwrap();

        py.run_bound(
            "import sys\nsys.modules['snail'] = snail_module\nfrom snail import exec_snail\nns = exec_snail('import math\\nresult = math.sqrt(16)')\nvalue = ns['result']",
            None,
            Some(&locals),
        )
        .expect("python should run snail code");

        let value_obj = locals
            .get_item("value")
            .expect("value lookup should succeed")
            .expect("value should be present");
        let value: f64 = value_obj.extract::<f64>().unwrap();
        assert_eq!(value, 4.0);
    });
}

#[test]
fn future_braces_import_is_noop() {
    pyo3::prepare_freethreaded_python();
    let source = "from __future__ import braces\nvalue = 1";

    Python::with_gil(|py| {
        let globals = exec_snail(py, source, Some("<future-braces>"), None, None)
            .expect("source should execute");
        let globals = globals
            .bind(py)
            .downcast::<PyDict>()
            .expect("globals should be a dict");
        let value: i64 = globals
            .get_item("value")
            .expect("value lookup should succeed")
            .expect("value should be present")
            .extract()
            .expect("value should be int");
        assert_eq!(value, 1);
    });
}

#[test]
fn interpolates_fstrings_in_string_literals() {
    pyo3::prepare_freethreaded_python();
    let source = r#"
name = "snail"
greeting = "hello {name}"
"#;

    Python::with_gil(|py| {
        let globals = exec_snail(py, source, Some("<fstring-test>"), None, None)
            .expect("source should execute");
        let globals = globals
            .bind(py)
            .downcast::<PyDict>()
            .expect("globals should be a dict");

        let greeting: String = globals
            .get_item("greeting")
            .expect("greeting lookup should succeed")
            .expect("greeting should be present")
            .extract()
            .expect("greeting should be string");
        assert_eq!(greeting, "hello snail");
    });
}

#[test]
fn import_hook_loads_snail_files() {
    pyo3::prepare_freethreaded_python();
    let temp = TempDir::new().expect("temp dir");
    let mut module_path = PathBuf::from(temp.path());
    module_path.push("example.snail");
    fs::write(
        &module_path,
        "import math\ndef compute() { return math.sqrt(9) }\nresult = compute()",
    )
    .expect("write snail file");

    Python::with_gil(|py| {
        let module = register_in_python(py).expect("snail module should register");
        let locals = PyDict::new_bound(py);
        locals.set_item("snail_module", &module).unwrap();
        locals
            .set_item("module_dir", temp.path().to_string_lossy().to_string())
            .unwrap();

        py.run_bound(
            r#"
import sys
import importlib

sys.modules['snail'] = snail_module
from snail import install_import_hook
install_import_hook()
sys.path.insert(0, module_dir)
import example
value = example.result
"#,
            None,
            Some(&locals),
        )
        .expect("python should import snail module");

        let value_obj = locals
            .get_item("value")
            .expect("value lookup should succeed")
            .expect("value should be present");
        let value: f64 = value_obj.extract::<f64>().unwrap();
        assert_eq!(value, 3.0);
    });
}

#[test]
fn executes_example_files() {
    pyo3::prepare_freethreaded_python();
    let examples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");

    let all_syntax_path = examples_dir.join("all_syntax.snail");
    let all_syntax = fs::read_to_string(&all_syntax_path).expect("read example source");

    Python::with_gil(|py| {
        let globals = exec_snail(
            py,
            &all_syntax,
            Some("examples/all_syntax.snail"),
            None,
            None,
        )
        .expect("example should execute");
        let globals = globals
            .bind(py)
            .downcast::<PyDict>()
            .expect("globals should be a dict");

        let total: i64 = globals
            .get_item("total")
            .expect("total lookup should succeed")
            .expect("total should be present")
            .extract()
            .expect("total should be int");
        assert_eq!(total, 6);

        let evens: Vec<i64> = globals
            .get_item("evens")
            .expect("evens lookup should succeed")
            .expect("evens should be present")
            .extract()
            .expect("evens should be list");
        assert_eq!(evens, vec![2, 4]);

        let lookup: HashMap<i64, i64> = globals
            .get_item("lookup")
            .expect("lookup lookup should succeed")
            .expect("lookup should be present")
            .extract()
            .expect("lookup should be dict");
        let mut keys: Vec<i64> = lookup.keys().copied().collect();
        keys.sort_unstable();
        assert_eq!(keys, vec![2, 3, 4]);
        assert_eq!(lookup.get(&2), Some(&4));
        assert_eq!(lookup.get(&3), Some(&6));
        assert_eq!(lookup.get(&4), Some(&8));
    });

    let awk_example_path = examples_dir.join("awk.snail");
    let awk_example = fs::read_to_string(&awk_example_path).expect("read awk example");

    Python::with_gil(|py| {
        let sys = PyModule::import_bound(py, "sys").expect("import sys");
        let io = PyModule::import_bound(py, "io").expect("import io");

        let stdin = io
            .getattr("StringIO")
            .expect("lookup StringIO")
            .call1(("alpha\nbeta\ndelta\n",))
            .expect("build stdin");
        let stdout = io
            .getattr("StringIO")
            .expect("lookup StringIO")
            .call0()
            .expect("build stdout");

        sys.setattr("stdin", &stdin).expect("install stdin");
        sys.setattr("stdout", &stdout).expect("install stdout");

        exec_snail(py, &awk_example, Some("examples/awk.snail"), None, None)
            .expect("awk example should execute");

        let output: String = stdout
            .call_method0("getvalue")
            .expect("extract output")
            .extract()
            .expect("output should be string");

        assert_eq!(
            output,
            "demo begin\n1\nalpha\n2\nbeta\nmatched: delta\n3\ndelta\ndemo end\n"
        );
    });
}

#[test]
fn compact_exception_prefers_fallback_lambda() {
    pyo3::prepare_freethreaded_python();
    let source = r#"
def fallback() {
    return "dunder"
}

def risky() {
    err = Exception("boom")
    err.__fallback__ = fallback
    raise err
}

preferred = risky() ? "lambda"
dunder_only = risky()?
"#;

    Python::with_gil(|py| {
        let globals = exec_snail(py, source, Some("<fallback-test>"), None, None)
            .expect("source should execute");
        let globals = globals
            .bind(py)
            .downcast::<PyDict>()
            .expect("globals should be a dict");

        let preferred: String = globals
            .get_item("preferred")
            .expect("preferred lookup should succeed")
            .expect("preferred should be present")
            .extract()
            .expect("preferred should be string");
        assert_eq!(preferred, "lambda");

        let dunder_only: String = globals
            .get_item("dunder_only")
            .expect("dunder_only lookup should succeed")
            .expect("dunder_only should be present")
            .extract()
            .expect("dunder_only should be string");
        assert_eq!(dunder_only, "dunder");
    });
}

#[test]
fn compound_expressions_sequence_and_work_with_fallbacks() {
    pyo3::prepare_freethreaded_python();
    let source = r#"
calls = []

def push(value) {
    calls.append(value)
    return value
}

def boom() {
    raise ValueError("boom")
}

last_value = (push(1); push(2); push(3))
recovered = (push("before"); boom()) ? "handled"
swallowed = (boom(); push("never"))?
"#;

    Python::with_gil(|py| {
        let globals = exec_snail(py, source, Some("<compound-expr>"), None, None)
            .expect("source should execute");
        let globals = globals
            .bind(py)
            .downcast::<PyDict>()
            .expect("globals should be a dict");

        let calls = globals
            .get_item("calls")
            .expect("calls lookup should succeed")
            .expect("calls should be present");
        let calls_repr: String = calls.str().unwrap().extract().unwrap();
        assert_eq!(calls_repr, "[1, 2, 3, 'before']");

        let last_value: i64 = globals
            .get_item("last_value")
            .expect("last_value lookup should succeed")
            .expect("last_value should be present")
            .extract()
            .expect("last_value should be int");
        assert_eq!(last_value, 3);

        let recovered: String = globals
            .get_item("recovered")
            .expect("recovered lookup should succeed")
            .expect("recovered should be present")
            .extract()
            .expect("recovered should be string");
        assert_eq!(recovered, "handled");

        let swallowed = globals
            .get_item("swallowed")
            .expect("swallowed lookup should succeed")
            .expect("swallowed should be present");
        let swallowed_type: String = swallowed
            .getattr("__class__")
            .unwrap()
            .getattr("__name__")
            .unwrap()
            .extract()
            .unwrap();
        assert_eq!(swallowed_type, "ValueError");
    });
}

#[test]
fn snail_callables_are_python_callables() {
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let module = register_in_python(py).expect("snail module should register");
        let locals = PyDict::new_bound(py);
        locals.set_item("snail_module", &module).unwrap();

        py.run_bound(
            "import sys\nsys.modules['snail'] = snail_module\nfrom snail import exec_snail\nns = exec_snail('def add(x, y) { return x + y }\\nclass Greeter {\\n    def greet(self, name) { return \"hi \" + name }\\n}')\nvalue = ns['add'](1, 2)\ngreeting = ns['Greeter']().greet('Snail')\nname = ns['__name__']",
            None,
            Some(&locals),
        )
        .expect("python should call snail values");

        let value_obj = locals
            .get_item("value")
            .expect("value lookup should succeed")
            .expect("value should be present");
        let value: i64 = value_obj.extract().unwrap();
        assert_eq!(value, 3);

        let greeting_obj = locals
            .get_item("greeting")
            .expect("greeting lookup should succeed")
            .expect("greeting should be present");
        let greeting: String = greeting_obj.extract().unwrap();
        assert_eq!(greeting, "hi Snail");

        let name_obj = locals
            .get_item("name")
            .expect("name lookup should succeed")
            .expect("name should be present");
        let name: String = name_obj.extract().unwrap();
        assert_eq!(name, "__snail__");
    });
}

#[test]
fn mixes_snail_and_python_modules() {
    pyo3::prepare_freethreaded_python();
    let temp = TempDir::new().expect("temp dir");
    let mut python_path = PathBuf::from(temp.path());
    python_path.push("helper.py");
    fs::write(
        &python_path,
        "def greeting(name):\n    return f\"hello {name}\"",
    )
    .expect("write python module");

    let mut snail_path = PathBuf::from(temp.path());
    snail_path.push("worker.snail");
    fs::write(
        &snail_path,
        "import helper\ndef build() { return helper.greeting(\"snail\") }\nvalue = build()",
    )
    .expect("write snail module");

    Python::with_gil(|py| {
        let module = register_in_python(py).expect("snail module should register");
        let locals = PyDict::new_bound(py);
        locals.set_item("snail_module", &module).unwrap();
        locals
            .set_item("module_dir", temp.path().to_string_lossy().to_string())
            .unwrap();

        py.run_bound(
            r#"
import sys
sys.modules['snail'] = snail_module
from snail import install_import_hook
install_import_hook()
sys.path.insert(0, module_dir)
import worker
import helper
snail_value = worker.value
python_value = helper.greeting("python")
"#,
            None,
            Some(&locals),
        )
        .expect("python should mix snail and python modules");

        let snail_value_obj = locals
            .get_item("snail_value")
            .expect("snail value lookup should succeed")
            .expect("snail value should be present");
        let snail_value: String = snail_value_obj.extract().unwrap();
        assert_eq!(snail_value, "hello snail");

        let python_value_obj = locals
            .get_item("python_value")
            .expect("python value lookup should succeed")
            .expect("python value should be present");
        let python_value: String = python_value_obj.extract().unwrap();
        assert_eq!(python_value, "hello python");
    });
}
