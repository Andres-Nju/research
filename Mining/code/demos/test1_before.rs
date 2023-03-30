fn test_1(){
    let name = match oid.as_slice_less_safe() {
        _ => unreachable!("unhandled x500 attr")
    };

}