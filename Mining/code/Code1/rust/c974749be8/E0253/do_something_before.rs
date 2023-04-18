        fn do_something();
    }
}

use foo::MyTrait::do_something;
    //~^ ERROR E0253
    //~|NOTE not directly importable

fn main() {}
