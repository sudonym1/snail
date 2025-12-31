use std::fs;
use std::path::PathBuf;

use pyo3::prelude::*;
use pyo3::types::PyDict;
use tempfile::TempDir;

use snail::register_in_python;

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
