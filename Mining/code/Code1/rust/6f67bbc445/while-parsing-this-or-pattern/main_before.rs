fn main() {
    match Some(42) {
        Some(42) | .=. => {} //~ ERROR expected pattern, found `.`
        //~^ while parsing this or-pattern staring here
        //~| NOTE expected pattern
    }
}
