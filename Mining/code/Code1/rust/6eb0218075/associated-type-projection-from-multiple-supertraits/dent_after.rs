fn dent<C:BoxCar>(c: C, color: C::Color) {
    //~^ ERROR ambiguous associated type `Color` in bounds of `C`
    //~| NOTE ambiguous associated type `Color`
    //~| NOTE could derive from `Vehicle`
    //~| NOTE could derive from `Box`
}
