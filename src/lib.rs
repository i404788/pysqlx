#![cfg_attr(feature = "intrinsics", feature(core_intrinsics))]

use futures::lock::Mutex;
use std::{pin::Pin, str::FromStr, sync::OnceLock};

use eyre::Result;
use futures::TryStreamExt;
use futures_core::stream::BoxStream;
use hashbrown::HashMap;
use pyo3::{
    ffi::PyTypeObject,
    intern,
    prelude::*,
    types::{PyBytes, PyDict, PyInt, PyString, PyType},
    PyTypeInfo,
};
use sqlx::{
    any::{AnyConnectOptions, AnyRow, AnyValue},
    AnyPool, Executor, Row, ValueRef,
};

#[macro_use]
mod str;
pub(crate) mod typeref;

use str::unicode_from_str;
use typeref::NONE;

struct PyTypeLut<T: Clone> {
    type_lut: dashmap::DashMap<*mut PyTypeObject, T>,
}

impl<T: Clone> PyTypeLut<T> {
    fn new() -> Self {
        PyTypeLut {
            type_lut: dashmap::DashMap::new(),
        }
    }

    fn add_type_explicit(&self, ptype: Bound<'_, PyType>, associated: T) {
        self.type_lut.insert(ptype.as_type_ptr(), associated);
    }

    fn get_or_index(&self, ptype: Bound<'_, PyType>) -> Result<T, ()> {
        if let Some(v) = self.type_lut.get(&ptype.as_type_ptr()) {
            return Ok(v.clone());
        }

        // Iter over type_lut to find a is_subclass match and store that type into the lut
        // For performance reasons we copy the info from the found type directly to the new type
        //  this means changing the associated type of a root-class will not overwrite derived classes (as they were effectively cached)
        for kv in self.type_lut.iter() {
            let stype = unsafe { PyType::from_borrowed_type_ptr(ptype.py(), kv.key().clone()) };
            if ptype.is_subclass(&stype).unwrap() {
                self.type_lut
                    .insert(ptype.as_type_ptr(), kv.value().clone());
                return Ok(kv.value().clone());
            }
        }

        Err(())
    }
}

// SAFETY: this is based on the assumption (ref: https://github.com/PyO3/pyo3/discussions/3104) that PyTypeObject ptrs are static and everliving
unsafe impl<T: Clone> Send for PyTypeLut<T> {}
unsafe impl<T: Clone> Sync for PyTypeLut<T> {}

#[derive(Clone)]
enum TypeAffinity {
    Integer,
    Text,
    Blob,
    Real,
    Numeric,
}

#[derive(Clone)]
struct SqlType {
    affinity: TypeAffinity,
    nullable: bool,
}

static PY_TYPE_LUT: OnceLock<PyTypeLut<SqlType>> = OnceLock::new();

struct TypeDef {
    sql_type: &'static str, // Blob, varchar, etc
}

fn try_get_root_sql_type<'py>(anno: &Bound<'py, PyAny>) -> Result<SqlType> {
    let types_mod = anno.py().import(intern!(anno.py(), "types"))?;
    let typing_mod = anno.py().import(intern!(anno.py(), "typing"))?;

    let union_type = types_mod.getattr(intern!(anno.py(), "UnionType"))?;
    let union_typing = typing_mod.getattr(intern!(anno.py(), "_UnionGenericAlias"))?;

    if let Ok(v) = anno.getattr(intern!(anno.py(), "__class__")) {
        if v.is_exact_instance(&union_type) || v.is_exact_instance(&union_typing) {
            let tuple_types = v.getattr(intern!(anno.py(), "__args__"))?;
            todo!("if <type> | None, where <type> is primitive, use Nullable");
        }
    }

    Err(eyre::eyre!("No valid sqltype found for {:?}", anno))
}

// TODO: impl try_from<PyAny> (type annotation) for TypeDef

struct RegisteredModel {
    primary_key: String,
    schema: HashMap<String, TypeDef>,
}

#[pyclass]
struct SqlxDb {
    conn: AnyPool,
    registered_models: HashMap<String, RegisteredModel>,
}

#[pyclass]
struct SqlxRow(AnyRow);

#[pymethods]
impl SqlxRow {
    fn __getitem__<'py>(&self, key: Bound<'py, PyString>) -> *mut pyo3::ffi::PyObject {
        // TODO key can technically be either string or int (for column index)
        match self.0.try_get_raw(key.to_str().unwrap()) {
            Ok(v) => {
                let v = ValueRef::to_owned(&v).to_owned();
                match v.kind {
                    sqlx::any::AnyValueKind::Bool(b) => unsafe {
                        pyo3::ffi::PyBool_FromLong(b as i64)
                    },
                    sqlx::any::AnyValueKind::SmallInt(a) => unsafe {
                        pyo3::ffi::PyLong_FromLongLong(a as i64)
                    },
                    sqlx::any::AnyValueKind::Integer(a) => unsafe {
                        pyo3::ffi::PyLong_FromLongLong(a as i64)
                    },
                    sqlx::any::AnyValueKind::BigInt(a) => unsafe {
                        pyo3::ffi::PyLong_FromLongLong(a)
                    },
                    sqlx::any::AnyValueKind::Real(v) => unsafe {
                        pyo3::ffi::PyFloat_FromDouble(v as f64)
                    },
                    sqlx::any::AnyValueKind::Double(v) => unsafe {
                        pyo3::ffi::PyFloat_FromDouble(v)
                    },
                    sqlx::any::AnyValueKind::Text(v) => unicode_from_str(&v),
                    sqlx::any::AnyValueKind::Blob(v) => unsafe {
                        pyo3::ffi::PyBytes_FromStringAndSize(
                            v.as_ptr() as *const i8,
                            v.len() as isize,
                        )
                    },
                    sqlx::any::AnyValueKind::Null(_) => use_immortal!(NONE),
                    _ => todo!("Unknown value kind"),
                }
            }
            Err(_) => todo!(),
        }
    }
}

#[pyclass]
struct SqlxStreamRequest {
    query: Pin<String>,
    // TODO: mutex is a bit slow for something that isn't expected to be multi-threaded, maybe futex? (guard needs to be Send for py async)
    stream: Option<Mutex<BoxStream<'static, Result<AnyRow, sqlx::Error>>>>,
}

impl SqlxStreamRequest {
    fn new(query: impl Into<String>, pool: &AnyPool) -> Self {
        let mut me = Self {
            query: Pin::new(query.into()),
            stream: None,
        };
        me.run(pool);
        me
    }

    fn run(&mut self, pool: &AnyPool) {
        // SAFETY: this is what we, in the business, call a "lie"; while the borrow lifetime is invalid the query should exists as long as the stream exists
        //  Since Pin<String> should exists as long SqlStreamRequest exists
        let query_str = unsafe {
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                self.query.as_ptr(),
                self.query.len(),
            ))
        };
        // Eqv to as_str().trustmybro() (unstable #![feature(str_as_str)])
        // let query_str: &'e str = unsafe { core::mem::transmute(self.query.as_str()) };
        self.stream.replace(Mutex::new(pool.fetch(query_str)));
    }

    async fn next_row(&mut self) -> Result<AnyRow, sqlx::Error> {
        let stream = self.stream.as_mut().expect("Stream was not initialized");
        stream
            .lock()
            .await
            .try_next()
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }
}

#[pymethods]
impl SqlxStreamRequest {
    async fn next(&mut self) -> Option<SqlxRow> {
        let row = self.next_row().await.ok()?;

        Some(SqlxRow(row))
        // TODO: convert row to Opaque PyObject
    }
}

#[pymethods]
impl SqlxDb {
    #[new]
    fn new(connection_str: &str) -> Self {
        let pool_options =
            AnyConnectOptions::from_str(connection_str).expect("Failed to parse connection string");

        // TODO(perf): share the type_lut to be global lut (reason for dashmap)
        SqlxDb {
            conn: AnyPool::connect_lazy_with(pool_options),
            registered_models: HashMap::new(),
        }
    }

    fn start_query<'py>(&mut self, query: Bound<'py, PyAny>) -> PyResult<SqlxStreamRequest> {
        let query = query.downcast_into_exact::<PyString>()?;
        let query = query.to_str()?;
        let req = SqlxStreamRequest::new(query, &self.conn);

        Ok(req)
    }

    fn register_model<'py>(&mut self, model: &Bound<'py, PyAny>) -> PyResult<bool> {
        let annotations = model
            .getattr(intern!(model.py(), "__annotations__"))
            .expect("Expected model type to have fields accessible in `__annotations__` (msgspec or pydantic)");
        let annotations: &Bound<'py, PyDict> = annotations.downcast()?;

        for (k, v) in annotations.iter() {
            todo!("Get PyType of various desired types/typings (e.g. UnionType, GenericAlias, primitives) by PyType::as_type_ptr into a HashMap to know how to handle them");

            println!("k={:?} v={:?}", k, v);
            let type_name = v
                .downcast::<PyType>()
                .expect("Not type")
                .qualname()
                .unwrap()
                .to_string();
            match type_name.as_str() {
                "int" => println!("{k} is an Integer"),
                "bool" => todo!("bool definition"),
                "float" => todo!("float definition"),
                "str" => todo!("str definition"),
                "dict" => todo!("dict definition"),
                "bytes" => todo!("Bytes definition"),
                "Annotated" => todo!("Recurse with options"),
                _ => unimplemented!("{type_name:?} not added yet"),
            }
            // todo!("Read type metadata")
        }
        todo!("Reflect model to extract types")
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn pysqlx(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    sqlx::any::install_default_drivers();
    typeref::init_typerefs();

    let lut = PY_TYPE_LUT.get_or_init(|| PyTypeLut::new());

    lut.add_type_explicit(
        PyInt::type_object(py),
        SqlType {
            affinity: TypeAffinity::Integer,
            nullable: false,
        },
    );

    lut.add_type_explicit(
        PyInt::type_object(py),
        SqlType {
            affinity: TypeAffinity::Integer,
            nullable: false,
        },
    );
    // Dicts will be encoded with msgspack or similar (todo: specify encoding method?)
    lut.add_type_explicit(
        PyDict::type_object(py),
        SqlType {
            affinity: TypeAffinity::Blob,
            nullable: false,
        },
    );
    lut.add_type_explicit(
        PyBytes::type_object(py),
        SqlType {
            affinity: TypeAffinity::Blob,
            nullable: false,
        },
    );

    m.add_class::<SqlxDb>()?;
    m.add_class::<SqlxRow>()?;
    m.add_class::<SqlxStreamRequest>()?;

    Ok(())
}
