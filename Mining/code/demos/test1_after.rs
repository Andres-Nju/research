fn test_1()   {
    let name = match oid.as_slice_less_safe() {
        _ => unreachable!("unhandled x500 attr {:?}", oid)
    };
    let dtr = match valuety {
        0x0c | 0x13 | 0x14 | 0x16 => std::str::from_utf8(value.as_slice_less_safe()).unwrap(),
        0x0c | 0x13 | 0x14 | 0x16 => std::str::from_utf8(value.as_slice_less_safe()).unwrap(),
        0x0c | 0x13 | 0x14 | 0x16 => std::str::from_utf8(value.as_slice_less_safe()).unwrap(),
        0x0c | 0x13 | 0x14 | 0x16 => std::str::from_utf8(value.as_slice_less_safe()).unwrap(),
        _ => 
        unreachable!
        ("unhandled x500 value type {:?}", 
        valuety)
    };
} 
