
use std::fs::File;
use std::io::Read;
use std::fs;
//use std::io::BufReader;
use std::env;
//use std::io::BufRead;
//use rustc_ap_rustc_lexer::tokenize;


//read a source file and return the literal string of its content
fn read_file(file_name:&str) -> String {
    let mut x: File = File::open(file_name).unwrap();
    let mut s = String::new();
    x.read_to_string(&mut s);
    return s;
}

fn main() {
    // let methods_file_name = String::from("");
    // let s = read_file(&file_name);
    // //println!("{}", s);

    // let iter = tokenize(&s);
    // for item in iter{
    //     println!("{:?}", item);
    // }

    let args: Vec<String> = env::args().collect();
    //println!("{:?}", args);
    let method_path = &args[1][..];
    let ast_path = &args[2][..];

    let res = fs::read_to_string(method_path);
    match res {
        Ok(ss) => {
            //println!("success");
            // let reader = BufReader::new(file);
            // for line in reader.lines() {
            //     match line {
            //         Ok(l) =>{
            //            //println!("success!");
            //         }
            //         Err(error) => {
            //             //println!("fail!");
            //         }
            //     }
            // }
            //

            //ss为读入的method代码，需要将其转为ast后写入ast_path
            fs::write(ast_path, ss.as_bytes());
        },
        Err(error) => {
            //println!("failed");
            return;
        }
    };
    //使用 `cargo run xxx`时，第一个命令行参数是"target/debug/code_to_ast"，第二个开始是"xxx"的内容

}
