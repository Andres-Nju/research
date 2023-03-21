fn test_1(){
    let name = match oid.as_slice_less_safe() {
        &[0x55, 0x04, 0x03] => "CN",
        &[0x55, 0x04, 0x05] => "serialNumber",
        &[0x55, 0x04, 0x06] => "C",
        &[0x55, 0x04, 0x07] => "L",
        &[0x55, 0x04, 0x08] => "ST",
        &[0x55, 0x04, 0x09] => "STREET",
        &[0x55, 0x04, 0x0a] => "O",
        &[0x55, 0x04, 0x0b] => "OU",
        _ => unreachable!("unhandled x500 attr")
    };

    let str_value = match valuety {
        // PrintableString, UTF8String, TeletexString or IA5String
        0x0c | 0x13 | 0x14 | 0x16 => std::str::from_utf8(value.as_slice_less_safe()).unwrap(),
        _ => unreachable!("unhandled x500 value type")
    };
}