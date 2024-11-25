use pyo3::ffi::{PyObject, PyTypeObject};
use std::ptr::null_mut;

pub static mut DEFAULT: *mut PyObject = null_mut();
pub static mut OPTION: *mut PyObject = null_mut();

pub static mut NONE: *mut PyObject = null_mut();
pub static mut TRUE: *mut PyObject = null_mut();
pub static mut FALSE: *mut PyObject = null_mut();
pub static mut EMPTY_UNICODE: *mut PyObject = null_mut();

pub static mut BYTES_TYPE: *mut PyTypeObject = null_mut();
pub static mut BYTEARRAY_TYPE: *mut PyTypeObject = null_mut();
pub static mut MEMORYVIEW_TYPE: *mut PyTypeObject = null_mut();
pub static mut STR_TYPE: *mut PyTypeObject = null_mut();
pub static mut INT_TYPE: *mut PyTypeObject = null_mut();
pub static mut BOOL_TYPE: *mut PyTypeObject = null_mut();
pub static mut NONE_TYPE: *mut PyTypeObject = null_mut();
pub static mut FLOAT_TYPE: *mut PyTypeObject = null_mut();
pub static mut LIST_TYPE: *mut PyTypeObject = null_mut();
pub static mut DICT_TYPE: *mut PyTypeObject = null_mut();
pub static mut DATETIME_TYPE: *mut PyTypeObject = null_mut();
pub static mut DATE_TYPE: *mut PyTypeObject = null_mut();
pub static mut TIME_TYPE: *mut PyTypeObject = null_mut();
pub static mut TUPLE_TYPE: *mut PyTypeObject = null_mut();
pub static mut UUID_TYPE: *mut PyTypeObject = null_mut();
pub static mut ENUM_TYPE: *mut PyTypeObject = null_mut();
pub static mut FIELD_TYPE: *mut PyTypeObject = null_mut();
pub static mut FRAGMENT_TYPE: *mut PyTypeObject = null_mut();

// pub static mut NUMPY_TYPES: OnceBox<Option<NonNull<NumpyTypes>>> = OnceBox::new();

#[cfg(Py_3_9)]
pub static mut ZONEINFO_TYPE: *mut PyTypeObject = null_mut();

pub static mut UTCOFFSET_METHOD_STR: *mut PyObject = null_mut();
pub static mut NORMALIZE_METHOD_STR: *mut PyObject = null_mut();
pub static mut CONVERT_METHOD_STR: *mut PyObject = null_mut();
pub static mut DST_STR: *mut PyObject = null_mut();

pub static mut DICT_STR: *mut PyObject = null_mut();
pub static mut DATACLASS_FIELDS_STR: *mut PyObject = null_mut();
pub static mut SLOTS_STR: *mut PyObject = null_mut();
pub static mut FIELD_TYPE_STR: *mut PyObject = null_mut();
pub static mut ARRAY_STRUCT_STR: *mut PyObject = null_mut();
pub static mut DTYPE_STR: *mut PyObject = null_mut();
pub static mut DESCR_STR: *mut PyObject = null_mut();
pub static mut VALUE_STR: *mut PyObject = null_mut();
pub static mut INT_ATTR_STR: *mut PyObject = null_mut();

static INIT: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

#[cold]
#[cfg_attr(feature = "optimize", optimize(size))]
pub fn init_typerefs() {
    INIT.get_or_init(_init_typerefs_impl);
}

#[cold]
#[cfg_attr(feature = "optimize", optimize(size))]
fn _init_typerefs_impl() -> bool {
    unsafe {
        // debug_assert!(crate::opt::MAX_OPT < u16::MAX as i32);

        // assert!(crate::deserialize::KEY_MAP
        //     .set(crate::deserialize::KeyMap::default())
        //     .is_ok());
        // FRAGMENT_TYPE = orjson_fragmenttype_new();
        // PyDateTime_IMPORT();
        NONE = pyo3::ffi::Py_None();
        TRUE = pyo3::ffi::Py_True();
        FALSE = pyo3::ffi::Py_False();
        EMPTY_UNICODE = pyo3::ffi::PyUnicode_New(0, 255);
        // STR_TYPE = (*EMPTY_UNICODE).ob_type;
        // BYTES_TYPE = (*PyBytes_FromStringAndSize("".as_ptr() as *const c_char, 0)).ob_type;

        // {
        //     let bytearray = PyByteArray_FromStringAndSize("".as_ptr() as *const c_char, 0);
        //     BYTEARRAY_TYPE = (*bytearray).ob_type;

        //     let memoryview = PyMemoryView_FromObject(bytearray);
        //     MEMORYVIEW_TYPE = (*memoryview).ob_type;
        //     Py_DECREF(memoryview);
        //     Py_DECREF(bytearray);
        // }

        // DICT_TYPE = (*PyDict_New()).ob_type;
        // LIST_TYPE = (*PyList_New(0)).ob_type;
        // TUPLE_TYPE = (*PyTuple_New(0)).ob_type;
        // NONE_TYPE = (*NONE).ob_type;
        // BOOL_TYPE = (*TRUE).ob_type;
        // INT_TYPE = (*PyLong_FromLongLong(0)).ob_type;
        // FLOAT_TYPE = (*PyFloat_FromDouble(0.0)).ob_type;
        // DATETIME_TYPE = look_up_datetime_type();
        // DATE_TYPE = look_up_date_type();
        // TIME_TYPE = look_up_time_type();
        // UUID_TYPE = look_up_uuid_type();
        // ENUM_TYPE = look_up_enum_type();
        // FIELD_TYPE = look_up_field_type();

        // #[cfg(Py_3_9)]
        // {
        //     ZONEINFO_TYPE = look_up_zoneinfo_type();
        // }

        // INT_ATTR_STR = PyUnicode_InternFromString("int\0".as_ptr() as *const c_char);
        // UTCOFFSET_METHOD_STR = PyUnicode_InternFromString("utcoffset\0".as_ptr() as *const c_char);
        // NORMALIZE_METHOD_STR = PyUnicode_InternFromString("normalize\0".as_ptr() as *const c_char);
        // CONVERT_METHOD_STR = PyUnicode_InternFromString("convert\0".as_ptr() as *const c_char);
        // DST_STR = PyUnicode_InternFromString("dst\0".as_ptr() as *const c_char);
        // DICT_STR = PyUnicode_InternFromString("__dict__\0".as_ptr() as *const c_char);
        // DATACLASS_FIELDS_STR =
        //     PyUnicode_InternFromString("__dataclass_fields__\0".as_ptr() as *const c_char);
        // SLOTS_STR = PyUnicode_InternFromString("__slots__\0".as_ptr() as *const c_char);
        // FIELD_TYPE_STR = PyUnicode_InternFromString("_field_type\0".as_ptr() as *const c_char);
        // ARRAY_STRUCT_STR =
        //     PyUnicode_InternFromString("__array_struct__\0".as_ptr() as *const c_char);
        // DTYPE_STR = PyUnicode_InternFromString("dtype\0".as_ptr() as *const c_char);
        // DESCR_STR = PyUnicode_InternFromString("descr\0".as_ptr() as *const c_char);
        // VALUE_STR = PyUnicode_InternFromString("value\0".as_ptr() as *const c_char);
        // DEFAULT = PyUnicode_InternFromString("default\0".as_ptr() as *const c_char);
        // OPTION = PyUnicode_InternFromString("option\0".as_ptr() as *const c_char);
        // JsonEncodeError = pyo3_ffi::PyExc_TypeError;
        // Py_INCREF(JsonEncodeError);
        // JsonDecodeError = look_up_json_exc();
    };
    true
}
