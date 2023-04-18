extern "Rust" { fn foo(x: u8, ...); }   //~ ERROR E0045
                                        //~| NOTE variadics require C calling conventions

fn main() {
}
