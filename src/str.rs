use crate::typeref::EMPTY_UNICODE;

use pyo3::ffi::{PyASCIIObject, PyCompactUnicodeObject};

#[cfg(Py_3_12)]
macro_rules! use_immortal {
    ($op:expr) => {
        unsafe { $op }
    };
}

#[cfg(not(Py_3_12))]
macro_rules! use_immortal {
    ($op:expr) => {
        unsafe {
            pyo3::ffi::Py_INCREF($op);
            $op
        }
    };
}

#[cfg(feature = "intrinsics")]
macro_rules! unlikely {
    ($exp:expr) => {
        core::intrinsics::unlikely($exp)
    };
}

#[cfg(not(feature = "intrinsics"))]
macro_rules! unlikely {
    ($exp:expr) => {
        $exp
    };
}

#[allow(unused_macros)]
#[cfg(feature = "intrinsics")]
macro_rules! likely {
    ($exp:expr) => {
        core::intrinsics::likely($exp)
    };
}

#[allow(unused_macros)]
#[cfg(not(feature = "intrinsics"))]
macro_rules! likely {
    ($exp:expr) => {
        $exp
    };
}

macro_rules! assume {
    ($expr:expr) => {
        debug_assert!($expr);
        #[cfg(feature = "intrinsics")]
        unsafe {
            core::intrinsics::assume($expr);
        };
    };
}

#[inline(never)]
pub fn pyunicode_ascii(buf: *const u8, num_chars: usize) -> *mut pyo3::ffi::PyObject {
    unsafe {
        let ptr = pyo3::ffi::PyUnicode_New(num_chars as isize, 127);
        let data_ptr = ptr.cast::<PyASCIIObject>().offset(1) as *mut u8;
        core::ptr::copy_nonoverlapping(buf, data_ptr, num_chars);
        core::ptr::write(data_ptr.add(num_chars), 0);
        ptr
    }
}

#[cold]
#[inline(never)]
pub fn pyunicode_onebyte(buf: &str, num_chars: usize) -> *mut pyo3::ffi::PyObject {
    unsafe {
        let ptr = pyo3::ffi::PyUnicode_New(num_chars as isize, 255);
        let mut data_ptr = ptr.cast::<PyCompactUnicodeObject>().offset(1) as *mut u8;
        for each in buf.chars().fuse() {
            core::ptr::write(data_ptr, each as u8);
            data_ptr = data_ptr.offset(1);
        }
        core::ptr::write(data_ptr, 0);
        ptr
    }
}

#[inline(never)]
pub fn pyunicode_twobyte(buf: &str, num_chars: usize) -> *mut pyo3::ffi::PyObject {
    unsafe {
        let ptr = pyo3::ffi::PyUnicode_New(num_chars as isize, 65535);
        let mut data_ptr = ptr.cast::<PyCompactUnicodeObject>().offset(1) as *mut u16;
        for each in buf.chars().fuse() {
            core::ptr::write(data_ptr, each as u16);
            data_ptr = data_ptr.offset(1);
        }
        core::ptr::write(data_ptr, 0);
        ptr
    }
}

#[inline(never)]
pub fn pyunicode_fourbyte(buf: &str, num_chars: usize) -> *mut pyo3::ffi::PyObject {
    unsafe {
        let ptr = pyo3::ffi::PyUnicode_New(num_chars as isize, 1114111);
        let mut data_ptr = ptr.cast::<PyCompactUnicodeObject>().offset(1) as *mut u32;
        for each in buf.chars().fuse() {
            core::ptr::write(data_ptr, each as u32);
            data_ptr = data_ptr.offset(1);
        }
        core::ptr::write(data_ptr, 0);
        ptr
    }
}

#[inline(always)]
pub fn str_impl_kind_scalar(buf: &str, num_chars: usize) -> *mut pyo3::ffi::PyObject {
    unsafe {
        let len = buf.len();
        assume!(len > 0);

        if unlikely!(*(buf.as_bytes().as_ptr()) > 239) {
            return pyunicode_fourbyte(buf, num_chars);
        }

        if buf.as_bytes().iter().find(|v| **v > 239).is_some() {
            pyunicode_fourbyte(buf, num_chars)
        } else if buf.as_bytes().iter().find(|v| **v > 195).is_some() {
            pyunicode_twobyte(buf, num_chars)
        } else {
            pyunicode_onebyte(buf, num_chars)
        }
    }
}

#[inline(never)]
pub fn unicode_from_str(buf: &str) -> *mut pyo3::ffi::PyObject {
    if unlikely!(buf.is_empty()) {
        return use_immortal!(EMPTY_UNICODE);
    }
    let num_chars = bytecount::num_chars(buf.as_bytes());
    if buf.len() == num_chars {
        pyunicode_ascii(buf.as_ptr(), num_chars)
    } else {
        str_impl_kind_scalar(buf, num_chars)
    }
}
