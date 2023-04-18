fn main()
{
    fn h(x:i32) -> i32 {3*x}
    let mut vfnfer:Vec<Box<Any>> = vec![];
    vfnfer.push(box h);
    println!("{:?}",(vfnfer[0] as Fn)(3));
    //~^ ERROR the precise format of `Fn`-family traits'
    //~| ERROR E0243
    //~| NOTE expected 1 type arguments, found 0
    //~| ERROR the value of the associated type `Output` (from the trait `std::ops::FnOnce`)
    //~| NOTE in this expansion of println!
    //~| NOTE in this expansion of println!
    //~| NOTE in this expansion of println!
    //~| NOTE in this expansion of println!
}
