        fn do_something();
    }
}

use foo::MyTrait::do_something;
    //~^ ERROR E0253
    //~|NOTE cannot be imported directly

fn main() {}
