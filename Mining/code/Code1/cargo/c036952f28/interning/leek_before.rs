pub fn leek(s: String) -> &'static str {
    let boxed = s.into_boxed_str();
    let ptr = boxed.as_ptr();
    let len = boxed.len();
    mem::forget(boxed);
    unsafe {
        let slice = slice::from_raw_parts(ptr, len);
        str::from_utf8_unchecked(slice)
    }
}

lazy_static! {
    static ref STRING_CASHE: RwLock<HashSet<&'static str>> =
        RwLock::new(HashSet::new());
}

#[derive(Eq, PartialEq, Clone, Copy)]
pub struct InternedString {
    ptr: *const u8,
    len: usize,
}
