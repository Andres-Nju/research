
use std::fs::File;
use std::io::Read;
use std::fs;
use std::env;
use gag::BufferRedirect;

//read a source file and return the literal string of its content
fn read_file(file_name:&str) -> String {
    let mut x: File = File::open(file_name).unwrap();
    let mut s = String::new();
    x.read_to_string(&mut s);
    return s;
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let method_path = &args[1][..];
    let ast_path = &args[2][..];

    let res = fs::read_to_string(method_path);
    match res {
        Ok(content) => {
            //content为读入的method代码，需要将其转为ast后写入ast_path
            let ast = syn::parse_file(&content);
            match ast {
                Ok(ast_content) => {
                    let mut buf = BufferRedirect::stdout().unwrap();
                    println!("{:#?}", ast_content);
                    let mut output = String::new();
                    buf.read_to_string(&mut output).unwrap();
                    fs::write(ast_path, output);
                },
                Err(error) => {
                    println!("parse file {} failed", method_path);
                    return;
                }
            };
        },
        Err(error) => {
            println!("open file {} failed", method_path);
            return;
        }
    };


    
    
}
