use pyo3::prelude::*;
use pyo3::types::{PyList, PyTuple};
use snail_ast::SourceSpan;
use snail_error::LowerError;

pub struct AstBuilder<'py> {
    py: Python<'py>,
    ast: Bound<'py, PyModule>,
}

impl<'py> AstBuilder<'py> {
    pub fn new(py: Python<'py>) -> PyResult<Self> {
        Ok(Self {
            py,
            ast: py.import_bound("ast")?,
        })
    }

    pub fn py(&self) -> Python<'py> {
        self.py
    }

    pub fn module(&self, body: Vec<PyObject>, span: &SourceSpan) -> PyResult<PyObject> {
        let type_ignores = PyList::empty_bound(self.py);
        self.call_node(
            "Module",
            vec![
                PyList::new_bound(self.py, body).into_py(self.py),
                type_ignores.into_py(self.py),
            ],
            span,
        )
    }

    pub fn load_ctx(&self) -> PyResult<PyObject> {
        Ok(self.ast.getattr("Load")?.call0()?.into_py(self.py))
    }

    pub fn store_ctx(&self) -> PyResult<PyObject> {
        Ok(self.ast.getattr("Store")?.call0()?.into_py(self.py))
    }

    pub fn del_ctx(&self) -> PyResult<PyObject> {
        Ok(self.ast.getattr("Del")?.call0()?.into_py(self.py))
    }

    pub fn op(&self, name: &str) -> PyResult<PyObject> {
        Ok(self.ast.getattr(name)?.call0()?.into_py(self.py))
    }

    pub fn call_node(
        &self,
        name: &str,
        args: Vec<PyObject>,
        span: &SourceSpan,
    ) -> PyResult<PyObject> {
        let tuple = PyTuple::new_bound(self.py, args);
        let node = self.ast.getattr(name)?.call1(tuple)?;
        set_location(&node, span)?;
        Ok(node.into_py(self.py))
    }

    pub fn call_node_no_loc(&self, name: &str, args: Vec<PyObject>) -> PyResult<PyObject> {
        let tuple = PyTuple::new_bound(self.py, args);
        let node = self.ast.getattr(name)?.call1(tuple)?;
        Ok(node.into_py(self.py))
    }
}

pub fn set_location(node: &Bound<'_, PyAny>, span: &SourceSpan) -> PyResult<()> {
    node.setattr("lineno", span.start.line)?;
    node.setattr("col_offset", span.start.column)?;
    node.setattr("end_lineno", span.end.line)?;
    node.setattr("end_col_offset", span.end.column)?;
    Ok(())
}

pub fn py_err_to_lower(err: PyErr) -> LowerError {
    LowerError::new(err.to_string())
}
