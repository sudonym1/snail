#![allow(unsafe_op_in_unsafe_fn)]

use pyo3::exceptions::{PyException, PySyntaxError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};

use crate::{SnailError, lower_program, parse_program, python_source};

pub fn compile_snail_source(source: &str) -> Result<String, SnailError> {
    let program = parse_program(source)?;
    let module = lower_program(&program)?;
    Ok(python_source(&module))
}

fn compile_to_code(py: Python<'_>, source: &str, filename: &str) -> PyResult<PyObject> {
    let python = compile_snail_source(source).map_err(to_py_err)?;
    let builtins = PyModule::import_bound(py, "builtins")?;
    let compiled = builtins
        .getattr("compile")?
        .call1((python, filename, "exec"))?;
    Ok(compiled.into())
}

fn to_py_err(err: SnailError) -> PyErr {
    match err {
        SnailError::Parse(err) => PyErr::new::<PySyntaxError, _>(err.to_string()),
        SnailError::Lower(err) => PyErr::new::<PyException, _>(err.to_string()),
    }
}

#[pyfunction]
#[allow(unsafe_op_in_unsafe_fn)]
pub fn compile_snail(py: Python<'_>, source: &str, filename: Option<&str>) -> PyResult<PyObject> {
    let filename = filename.unwrap_or("<snail>");
    compile_to_code(py, source, filename)
}

#[pyfunction]
#[allow(unsafe_op_in_unsafe_fn)]
pub fn exec_snail(py: Python<'_>, source: &str, filename: Option<&str>) -> PyResult<PyObject> {
    let filename = filename.unwrap_or("<snail>");
    let code = compile_to_code(py, source, filename)?;
    let globals = PyDict::new_bound(py);
    PyModule::import_bound(py, "builtins")?
        .getattr("exec")?
        .call1((code, &globals, &globals))?;
    Ok(globals.into())
}

#[pyfunction]
#[allow(unsafe_op_in_unsafe_fn)]
pub fn translate_snail(source: &str) -> PyResult<String> {
    compile_snail_source(source).map_err(to_py_err)
}

#[pyfunction]
pub fn install_import_hook(py: Python<'_>) -> PyResult<()> {
    const IMPORTER_SOURCE: &str = r#"
import importlib.abc
import importlib.util
import pathlib
import sys

from snail import compile_snail


class SnailLoader(importlib.abc.SourceLoader):
    def __init__(self, path):
        self.path = pathlib.Path(path)

    def get_filename(self, fullname):
        return str(self.path)

    def get_data(self, path):
        return self.path.read_bytes()

    def source_to_code(self, data, path, _opt=None):
        source = data.decode('utf-8')
        return compile_snail(source, str(self.path))


class SnailFinder(importlib.abc.MetaPathFinder):
    def find_spec(self, fullname, path=None, target=None):
        module_name = fullname.rsplit('.', 1)[-1]
        search = path or sys.path
        candidate = module_name + '.snail'
        for entry in search:
            potential = pathlib.Path(entry) / candidate
            if potential.is_file():
                loader = SnailLoader(potential)
                return importlib.util.spec_from_file_location(fullname, potential, loader=loader)
        return None


def install():
    for finder in sys.meta_path:
        if isinstance(finder, SnailFinder):
            return
    sys.meta_path.insert(0, SnailFinder())
"#;

    let importer =
        PyModule::from_code_bound(py, IMPORTER_SOURCE, "snail_importer.py", "snail_importer")?;
    importer.getattr("install")?.call0()?;
    Ok(())
}

#[pymodule]
pub fn snail(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compile_snail, m)?)?;
    m.add_function(wrap_pyfunction!(exec_snail, m)?)?;
    m.add_function(wrap_pyfunction!(translate_snail, m)?)?;
    m.add_function(wrap_pyfunction!(install_import_hook, m)?)?;
    Ok(())
}

pub fn register_in_python(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let module = PyModule::new_bound(py, "snail")?;
    snail(py, &module)?;

    let sys = PyModule::import_bound(py, "sys")?;
    sys.getattr("modules")?.set_item("snail", &module)?;
    Ok(module)
}
