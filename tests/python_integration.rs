use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use pyo3::prelude::*;
use pyo3::types::PyDict;
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
fn executes_examples_all_syntax() {
    pyo3::prepare_freethreaded_python();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/all_syntax.snail");
    let source = fs::read_to_string(&path).expect("read example source");

    Python::with_gil(|py| {
        let globals = exec_snail(py, &source, Some("examples/all_syntax.snail"))
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
}
