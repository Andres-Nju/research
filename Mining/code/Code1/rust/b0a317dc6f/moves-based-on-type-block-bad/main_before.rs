fn main() {
    let s = S { x: box E::Bar(box 42) };
    loop {
        f(&s, |hellothere| {
            match hellothere.x { //~ ERROR cannot move out
                                 //~| cannot move out of borrowed content
                box E::Foo(_) => {}
                box E::Bar(x) => println!("{}", x.to_string()), 
                //~^ NOTE to prevent move
                box E::Baz => {}
            }
        })
    }
}
