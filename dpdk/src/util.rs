use std::ffi::CString;

use itertools::Itertools;


pub fn str_to_c_string(s: impl AsRef<str>) -> CString {
    let string: &str = s.as_ref();
    let mut bytes = string.bytes().collect_vec();
    bytes.push(0);
    unsafe {
        CString::from_vec_unchecked(bytes)
    }
}

pub fn cast_str_to_i8_ptr<'a>(s: &'a str) -> *const i8 {
    s.as_ptr() as *const i8
}