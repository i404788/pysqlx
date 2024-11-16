use std::{
    cell::OnceCell,
    str::FromStr,
    sync::{Arc, OnceLock},
};

use eyre::Result;
use hashbrown::HashMap;
use pyo3::{
    ffi::PyTypeObject,
    intern,
    prelude::*,
    types::{PyBytes, PyDict, PyInt, PyType},
    PyTypeInfo,
};
use sqlx::{
    any::{AnyConnectOptions, AnyPoolOptions},
    AnyConnection, AnyPool, Row,
};

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
            // TODO: if <type> | None, where <type> is primitive, use Nullable
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

/// Formats the sum of two numbers as string.
#[pyfunction]
fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
    Ok((a + b).to_string())
}

/// A Python module implemented in Rust.
#[pymodule]
fn pysqlx(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    sqlx::any::install_default_drivers();

    let lut = PY_TYPE_LUT.get_or_init(|| PyTypeLut::new());

    lut.add_type_explicit(
        PyInt::type_object(py),
        SqlType {
            affinity: TypeAffinity::Integer,
        },
    );
    lut.add_type_explicit(
        PyInt::type_object(py),
        SqlType {
            affinity: TypeAffinity::Integer,
        },
    );
    // Dicts will be encoded with msgspack or similar (todo: specify encoding method?)
    lut.add_type_explicit(
        PyDict::type_object(py),
        SqlType {
            affinity: TypeAffinity::Blob,
        },
    );
    lut.add_type_explicit(
        PyBytes::type_object(py),
        SqlType {
            affinity: TypeAffinity::Blob,
        },
    );

    m.add_class::<SqlxDb>()?;
    m.add_function(wrap_pyfunction!(sum_as_string, m)?)?;
    Ok(())
}
