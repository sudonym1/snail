use pyo3::prelude::*;
use pyo3::types::{PyList, PyTuple};
use snail_ast::SourceSpan;
use snail_error::LowerError;

pub struct AstBuilder<'py> {
    py: Python<'py>,
    ast: Bound<'py, PyModule>,
    needs_index_wrapper: bool,
}

impl<'py> AstBuilder<'py> {
    pub fn new(py: Python<'py>) -> PyResult<Self> {
        let version_info = py.import_bound("sys")?.getattr("version_info")?;
        let major: u8 = version_info.get_item(0)?.extract()?;
        let minor: u8 = version_info.get_item(1)?.extract()?;
        let needs_index_wrapper = major == 3 && minor < 9;
        Ok(Self {
            py,
            ast: py.import_bound("ast")?,
            needs_index_wrapper,
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

    pub fn wrap_index(&self, slice: PyObject, span: &SourceSpan) -> PyResult<PyObject> {
        if self.needs_index_wrapper {
            let slice_obj = slice.bind(self.py);
            if let Ok(slice_type) = self.ast.getattr("Slice")
                && slice_obj.is_instance(&slice_type)?
            {
                return Ok(slice);
            }
            if let Ok(ext_slice_type) = self.ast.getattr("ExtSlice")
                && slice_obj.is_instance(&ext_slice_type)?
            {
                return Ok(slice);
            }
            if let Ok(index_type) = self.ast.getattr("Index") {
                let index = index_type.call1((slice,))?;
                set_location(&index, span)?;
                return Ok(index.into_py(self.py));
            }
        }
        Ok(slice)
    }
}

pub fn set_location(node: &Bound<'_, PyAny>, span: &SourceSpan) -> PyResult<()> {
    node.setattr("lineno", span.start.line)?;
    node.setattr("col_offset", span.start.column.saturating_sub(1))?;
    node.setattr("end_lineno", span.end.line)?;
    node.setattr("end_col_offset", span.end.column.saturating_sub(1))?;
    Ok(())
}

pub fn py_err_to_lower(err: PyErr) -> LowerError {
    LowerError::new(err.to_string())
}
