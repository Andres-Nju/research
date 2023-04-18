fn paint<C:BoxCar>(c: C, d: C::Color) {
    //~^ ERROR ambiguous associated type `Color` in bounds of `C`
    //~| NOTE ambiguous associated type `Color`
    //~| NOTE could derive from `Vehicle`
    //~| NOTE could derive from `Box`
}
