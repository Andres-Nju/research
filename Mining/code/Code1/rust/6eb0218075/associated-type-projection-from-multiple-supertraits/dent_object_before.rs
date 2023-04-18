fn dent_object<COLOR>(c: BoxCar<Color=COLOR>) {
    //~^ ERROR ambiguous associated type
    //~| ERROR the value of the associated type `Color` (from the trait `Vehicle`) must be specified
    //~| NOTE could derive from `Vehicle`
    //~| NOTE could derive from `Box`
}
