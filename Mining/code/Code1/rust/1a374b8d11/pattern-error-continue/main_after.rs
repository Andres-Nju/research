fn main() {
    match A::B(1, 2) {
        A::B(_, _, _) => (), //~ ERROR this pattern has 3 fields, but
        A::D(_) => (),       //~ ERROR this pattern has 1 field, but
        _ => ()
    }
    match 'c' {
        S { .. } => (),
        //~^ ERROR mismatched types
        //~| expected `char`
        //~| found `S`
        //~| expected char
        //~| found struct `S`

        _ => ()
    }
    f(true);
    //~^ ERROR mismatched types
    //~| expected `char`
    //~| found `bool`

    match () {
        E::V => {} //~ ERROR failed to resolve. Use of undeclared type or module `E`
    }
}
