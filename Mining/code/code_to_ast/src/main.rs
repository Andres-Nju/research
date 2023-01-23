
use std::fs::File;
use std::io::Read;
use rustc_ap_rustc_lexer::tokenize;


//read a source file and return the literal string of its content
fn read_file(file_name:&str) -> String {
    let mut x: File = File::open(file_name).unwrap();
    let mut s = String::new();
    x.read_to_string(&mut s);
    //Some Text;hello你好;
    return s;
}

fn main() {
    let file_name = String::from("../../../../Compiler-for-C--/main.rs");
    let s = read_file(&file_name);
    //println!("{}", s);

    let iter = tokenize(&s);
    for item in iter{
        println!("{:?}", item);
    }
}
