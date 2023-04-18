    fn f<T>(self)
        where T<Bogus = Foo>: A;
        //~^ ERROR associated type bindings are not allowed here [E0229]
        //~| NOTE associated type not allowed here
}

fn main() {}
