/*
 * Copyright 2022, The Cozo Project Authors.
 *
 * This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
 * If a copy of the MPL was not distributed with this file,
 * You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![allow(deprecated)]

use std::collections::{BTreeMap, BTreeSet};

use miette::{IntoDiagnostic, Report, Result};
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::IntoPyObjectExt;
type PyObject = Py<PyAny>;
use pyo3::types::{PyBool, PyByteArray, PyBytes, PyDict, PyList, PyModule, PyString, PyTuple};
use serde_json::json;

use cozo::*;

fn py_to_rows(ob: &Bound<'_, PyAny>) -> PyResult<Vec<Vec<DataValue>>> {
    let rows = ob.extract::<Vec<Vec<Bound<'_, PyAny>>>>()?;
    let res: Vec<Vec<DataValue>> = rows
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|el| py_to_value(&el))
                .collect::<PyResult<_>>()
        })
        .collect::<PyResult<_>>()?;
    Ok(res)
}

fn report2py(r: Report) -> PyErr {
    PyException::new_err(r.to_string())
}

fn py_to_named_rows(ob: &Bound<'_, PyAny>) -> PyResult<NamedRows> {
    let d = ob.cast::<PyDict>()?;
    let rows = d
        .get_item("rows")?
        .ok_or_else(|| PyException::new_err("named rows must contain 'rows'"))?;
    let rows = py_to_rows(&rows)?;
    let headers = d
        .get_item("headers")?
        .ok_or_else(|| PyException::new_err("named rows must contain 'headers'"))?;
    let headers = headers.extract::<Vec<String>>()?;
    Ok(NamedRows::new(headers, rows))
}

fn py_to_value(ob: &Bound<'_, PyAny>) -> PyResult<DataValue> {
    Ok(if ob.is_none() {
        DataValue::Null
    } else if let Ok(b) = ob.cast::<PyBool>() {
        DataValue::from(b.is_true())
    } else if let Ok(i) = ob.extract::<i64>() {
        DataValue::from(i)
    } else if let Ok(f) = ob.extract::<f64>() {
        DataValue::from(f)
    } else if let Ok(s) = ob.extract::<String>() {
        DataValue::from(s)
    } else if let Ok(b) = ob.cast::<PyBytes>() {
        DataValue::Bytes(b.as_bytes().to_vec())
    } else if let Ok(b) = ob.cast::<PyByteArray>() {
        DataValue::Bytes(unsafe { b.as_bytes() }.to_vec())
    } else if let Ok(l) = ob.cast::<PyTuple>() {
        let mut coll = Vec::with_capacity(l.len());
        for el in l {
            let el = py_to_value(&el)?;
            coll.push(el)
        }
        DataValue::List(Box::new(coll))
    } else if let Ok(l) = ob.cast::<PyList>() {
        let mut coll = Vec::with_capacity(l.len());
        for el in l {
            let el = py_to_value(&el)?;
            coll.push(el)
        }
        DataValue::List(Box::new(coll))
    } else if let Ok(d) = ob.cast::<PyDict>() {
        let mut coll = serde_json::Map::default();
        for (k, v) in d {
            let k = serde_json::Value::from(py_to_value(&k)?);
            let k = match k {
                serde_json::Value::String(s) => s,
                s => s.to_string(),
            };
            let v = serde_json::Value::from(py_to_value(&v)?);
            coll.insert(k, v);
        }
        DataValue::Json(Box::new(JsonData(json!(coll))))
    } else {
        return Err(PyException::new_err(format!(
            "Cannot convert {ob} into Cozo value"
        )));
    })
}

fn convert_params(ob: &Bound<'_, PyDict>) -> PyResult<BTreeMap<String, DataValue>> {
    let mut ret = BTreeMap::new();
    for (k, v) in ob {
        let k: String = k.extract()?;
        let v = py_to_value(&v)?;
        ret.insert(k, v);
    }
    Ok(ret)
}

fn options_to_py<'py>(
    opts: BTreeMap<String, DataValue>,
    py: Python<'py>,
) -> PyResult<Bound<'py, PyDict>> {
    let ret = PyDict::new(py);

    for (k, v) in opts {
        let val = value_to_py(v, py)?;
        ret.set_item(k, val)?;
    }
    Ok(ret)
}

fn json_to_py(val: serde_json::Value, py: Python<'_>) -> PyResult<PyObject> {
    match val {
        serde_json::Value::Null => Ok(py.None()),
        serde_json::Value::Bool(b) => b.into_py_any(py),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_py_any(py)
            } else if let Some(f) = n.as_f64() {
                f.into_py_any(py)
            } else {
                Ok(py.None())
            }
        }
        serde_json::Value::String(s) => s.into_py_any(py),
        serde_json::Value::Array(a) => {
            let vs = a
                .into_iter()
                .map(|v| json_to_py(v, py))
                .collect::<PyResult<Vec<_>>>()?;
            vs.into_py_any(py)
        }
        serde_json::Value::Object(o) => {
            let d = PyDict::new(py);
            for (k, v) in o {
                d.set_item(k, json_to_py(v, py)?)?;
            }
            Ok(d.into())
        }
    }
}

fn value_to_py(val: DataValue, py: Python<'_>) -> PyResult<PyObject> {
    match val {
        DataValue::Null => Ok(py.None()),
        DataValue::Bool(b) => b.into_py_any(py),
        DataValue::Num(num) => match num {
            Num::Int(i) => i.into_py_any(py),
            Num::Float(f) => f.into_py_any(py),
        },
        DataValue::Str(s) => s.as_str().into_py_any(py),
        DataValue::Bytes(b) => Ok(PyBytes::new(py, &b).into()),
        DataValue::Uuid(uuid) => uuid.0.to_string().into_py_any(py),
        DataValue::Regex(rx) => rx.0.as_str().into_py_any(py),
        DataValue::List(l) => {
            let vs = l
                .into_iter()
                .map(|v| value_to_py(v, py))
                .collect::<PyResult<Vec<_>>>()?;
            vs.into_py_any(py)
        }
        DataValue::Set(l) => {
            let vs = l
                .into_iter()
                .map(|v| value_to_py(v, py))
                .collect::<PyResult<Vec<_>>>()?;
            vs.into_py_any(py)
        }
        DataValue::Validity(vld) => {
            let parts = [
                vld.timestamp.0 .0.into_py_any(py)?,
                vld.is_assert.0.into_py_any(py)?,
            ];
            parts.into_py_any(py)
        }
        DataValue::Bot => Ok(py.None()),
        DataValue::Vec(v) => match *v {
            Vector::F32(a) => {
                let vs = a
                    .into_iter()
                    .map(|v| v.into_py_any(py))
                    .collect::<PyResult<Vec<_>>>()?;
                vs.into_py_any(py)
            }
            Vector::F64(a) => {
                let vs = a
                    .into_iter()
                    .map(|v| v.into_py_any(py))
                    .collect::<PyResult<Vec<_>>>()?;
                vs.into_py_any(py)
            }
        },
        DataValue::Json(j) => json_to_py(j.0, py),
    }
}

fn rows_to_py_rows<R>(rows: Vec<R>, py: Python<'_>) -> PyResult<PyObject>
where
    R: IntoIterator<Item = DataValue>,
{
    let outer = rows
        .into_iter()
        .map(|row| {
            let inner = row
                .into_iter()
                .map(|val| value_to_py(val, py))
                .collect::<PyResult<Vec<_>>>()?;
            inner.into_py_any(py)
        })
        .collect::<PyResult<Vec<_>>>()?;
    outer.into_py_any(py)
}

fn named_rows_to_py(named_rows: NamedRows, py: Python<'_>) -> PyResult<PyObject> {
    let rows = rows_to_py_rows(named_rows.rows, py)?;
    let headers = named_rows.headers.into_py_any(py)?;
    let next = match named_rows.next {
        None => py.None(),
        Some(nxt) => named_rows_to_py(*nxt, py)?,
    };
    BTreeMap::from([("rows", rows), ("headers", headers), ("next", next)]).into_py_any(py)
}

#[pyclass]
struct CozoDbPy {
    db: Option<DbInstance>,
}

#[pyclass]
struct CozoDbMulTx {
    tx: MultiTransaction,
}

const DB_CLOSED_MSG: &str = r##"{"ok":false,"message":"database closed"}"##;

#[pymethods]
impl CozoDbPy {
    #[new]
    fn new(engine: &str, path: &str, options: &str) -> PyResult<Self> {
        match DbInstance::new(engine, path, options) {
            Ok(db) => Ok(Self { db: Some(db) }),
            Err(err) => Err(PyException::new_err(format!("{err:?}"))),
        }
    }
    pub fn run_script(
        &self,
        py: Python<'_>,
        query: &str,
        params: &Bound<'_, PyDict>,
        immutable: bool,
    ) -> PyResult<PyObject> {
        if let Some(db) = &self.db {
            let params = convert_params(params)?;
            match py.detach(|| {
                db.run_script(
                    query,
                    params,
                    if immutable {
                        ScriptMutability::Immutable
                    } else {
                        ScriptMutability::Mutable
                    },
                )
            }) {
                Ok(rows) => Ok(named_rows_to_py(rows, py)?),
                Err(err) => {
                    let reports = format_error_as_json(err, Some(query)).to_string();
                    let json_mod = py.import("json")?;
                    let loads_fn = json_mod.getattr("loads")?;
                    let args = PyTuple::new(py, [PyString::new(py, &reports)])?;
                    let msg = loads_fn.call1(args)?;
                    Err(PyException::new_err(msg.into_py_any(py)?))
                }
            }
        } else {
            Err(PyException::new_err(DB_CLOSED_MSG))
        }
    }
    pub fn register_callback(&self, rel: &str, callback: &Bound<'_, PyAny>) -> PyResult<u32> {
        if let Some(db) = &self.db {
            let cb: Py<PyAny> = callback.clone().unbind();
            let (id, ch) = db.register_callback(rel, None);
            rayon::spawn(move || {
                for (op, new, old) in ch {
                    Python::attach(|py| {
                        let op = PyString::new(py, op.as_str());
                        let new_py = match rows_to_py_rows(new.rows, py) {
                            Ok(n) => n,
                            Err(err) => {
                                eprintln!("{}", err);
                                return;
                            }
                        };
                        let old_py = match rows_to_py_rows(old.rows, py) {
                            Ok(o) => o,
                            Err(err) => {
                                eprintln!("{}", err);
                                return;
                            }
                        };
                        let args = PyTuple::new(
                            py,
                            [op.into_any(), new_py.into_bound(py), old_py.into_bound(py)],
                        );
                        let callable = cb.bind(py);
                        match args {
                            Ok(args) => {
                                if let Err(err) = callable.call1(args) {
                                    eprintln!("{}", err);
                                }
                            }
                            Err(err) => {
                                eprintln!("{}", err);
                            }
                        }
                    })
                }
            });
            Ok(id)
        } else {
            Err(PyException::new_err(DB_CLOSED_MSG))
        }
    }
    pub fn register_fixed_rule(
        &self,
        name: String,
        arity: usize,
        callback: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        if let Some(db) = &self.db {
            let cb: Py<PyAny> = callback.clone().unbind();
            let rule_impl = SimpleFixedRule::new(arity, move |inputs, options| -> Result<_> {
                Python::attach(|py| -> Result<NamedRows> {
                    let py_inputs_vec = inputs
                        .into_iter()
                        .map(|nr| rows_to_py_rows(nr.rows, py))
                        .collect::<PyResult<Vec<_>>>()
                        .into_diagnostic()?;
                    let py_inputs = PyList::new(py, py_inputs_vec).into_diagnostic()?;
                    let py_opts = options_to_py(options, py).into_diagnostic()?;
                    let args = PyTuple::new(py, vec![py_inputs.into_any(), py_opts.into_any()])
                        .into_diagnostic()?;
                    let res = cb.bind(py).call1(args).into_diagnostic()?;
                    Ok(NamedRows::new(vec![], py_to_rows(&res).into_diagnostic()?))
                })
            });
            db.register_fixed_rule(name, rule_impl).map_err(report2py)
        } else {
            Err(PyException::new_err(DB_CLOSED_MSG))
        }
    }
    pub fn unregister_callback(&self, id: u32) -> bool {
        if let Some(db) = &self.db {
            db.unregister_callback(id)
        } else {
            false
        }
    }
    pub fn unregister_fixed_rule(&self, name: &str) -> PyResult<bool> {
        if let Some(db) = &self.db {
            match db.unregister_fixed_rule(name) {
                Ok(b) => Ok(b),
                Err(err) => Err(PyException::new_err(err.to_string())),
            }
        } else {
            Ok(false)
        }
    }
    pub fn export_relations(&self, py: Python<'_>, relations: Vec<String>) -> PyResult<PyObject> {
        if let Some(db) = &self.db {
            let res = match py.detach(|| db.export_relations(relations.iter())) {
                Ok(res) => res,
                Err(err) => return Err(PyException::new_err(err.to_string())),
            };
            let ret = PyDict::new(py);
            for (k, v) in res {
                ret.set_item(k, named_rows_to_py(v, py)?)?;
            }
            Ok(ret.into())
        } else {
            Err(PyException::new_err(DB_CLOSED_MSG.to_string()))
        }
    }
    pub fn import_relations(&self, py: Python<'_>, data: &Bound<'_, PyDict>) -> PyResult<()> {
        if let Some(db) = &self.db {
            let mut arg = BTreeMap::new();
            for (k, v) in data.iter() {
                let k = k.extract::<String>()?;
                let vals = py_to_named_rows(&v)?;
                arg.insert(k, vals);
            }
            py.detach(|| db.import_relations(arg)).map_err(report2py)
        } else {
            Err(PyException::new_err(DB_CLOSED_MSG.to_string()))
        }
    }
    pub fn backup(&self, py: Python<'_>, path: &str) -> PyResult<()> {
        if let Some(db) = &self.db {
            py.detach(|| db.backup_db(path)).map_err(report2py)
        } else {
            Err(PyException::new_err(DB_CLOSED_MSG.to_string()))
        }
    }
    pub fn restore(&self, py: Python<'_>, path: &str) -> PyResult<()> {
        if let Some(db) = &self.db {
            py.detach(|| db.restore_backup(path)).map_err(report2py)
        } else {
            Err(PyException::new_err(DB_CLOSED_MSG.to_string()))
        }
    }
    pub fn import_from_backup(
        &self,
        py: Python<'_>,
        in_file: &str,
        relations: Vec<String>,
    ) -> PyResult<()> {
        if let Some(db) = &self.db {
            py.detach(|| db.import_from_backup(in_file, &relations))
                .map_err(report2py)
        } else {
            Err(PyException::new_err(DB_CLOSED_MSG.to_string()))
        }
    }
    pub fn close(&mut self) -> bool {
        self.db.take().is_some()
    }
    pub fn multi_transact(&self, write: bool) -> PyResult<CozoDbMulTx> {
        if let Some(db) = &self.db {
            Ok(CozoDbMulTx {
                tx: db.multi_transaction(write),
            })
        } else {
            Err(PyException::new_err(DB_CLOSED_MSG.to_string()))
        }
    }
}

#[pymethods]
impl CozoDbMulTx {
    pub fn abort(&self) -> PyResult<()> {
        self.tx
            .abort()
            .map_err(|err| PyException::new_err(err.to_string()))
    }
    pub fn commit(&self) -> PyResult<()> {
        self.tx
            .commit()
            .map_err(|err| PyException::new_err(err.to_string()))
    }
    pub fn run_script(
        &self,
        py: Python<'_>,
        query: &str,
        params: &Bound<'_, PyDict>,
    ) -> PyResult<PyObject> {
        let params = convert_params(params)?;
        match py.detach(|| self.tx.run_script(query, params)) {
            Ok(rows) => Ok(named_rows_to_py(rows, py)?),
            Err(err) => {
                let reports = format_error_as_json(err, Some(query)).to_string();
                let json_mod = py.import("json")?;
                let loads_fn = json_mod.getattr("loads")?;
                let args = PyTuple::new(py, [PyString::new(py, &reports)])?;
                let msg = loads_fn.call1(args)?;
                Err(PyException::new_err(msg.into_py_any(py)?))
            }
        }
    }
}

#[pyfunction]
fn eval_expressions(
    py: Python<'_>,
    query: &str,
    params: &Bound<'_, PyDict>,
    bindings: &Bound<'_, PyDict>,
) -> PyResult<PyObject> {
    let params = convert_params(params)?;
    let bindings = convert_params(bindings)?;
    match evaluate_expressions(query, &params, &bindings) {
        Ok(v) => Ok(value_to_py(v, py)?),
        Err(err) => {
            let reports = format_error_as_json(err, Some(query)).to_string();
            let json_mod = py.import("json")?;
            let loads_fn = json_mod.getattr("loads")?;
            let args = PyTuple::new(py, [PyString::new(py, &reports)])?;
            let msg = loads_fn.call1(args)?;
            Err(PyException::new_err(msg.into_py_any(py)?))
        }
    }
}

#[pyfunction]
fn variables(
    _py: Python<'_>,
    query: &str,
    params: &Bound<'_, PyDict>,
) -> PyResult<BTreeSet<String>> {
    let params = convert_params(params)?;
    match get_variables(query, &params) {
        Ok(rows) => Ok(rows),
        Err(err) => {
            let reports = format_error_as_json(err, Some(query)).to_string();
            let json_mod = _py.import("json")?;
            let loads_fn = json_mod.getattr("loads")?;
            let args = PyTuple::new(_py, [PyString::new(_py, &reports)])?;
            let msg = loads_fn.call1(args)?;
            Err(PyException::new_err(msg.into_py_any(_py)?))
        }
    }
}

#[pymodule]
fn cozo_embedded(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<CozoDbPy>()?;
    m.add_class::<CozoDbMulTx>()?;
    m.add_function(wrap_pyfunction!(eval_expressions, m)?)?;
    m.add_function(wrap_pyfunction!(variables, m)?)?;
    Ok(())
}
