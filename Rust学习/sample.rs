#[derive(Debug, Default, Copy, Clone)] 
struct A{
    a:i32,
    b:char
}

#[derive(Debug, Default, Copy, Clone)] 
struct B{
    a:A,
    b:char
}
// fn main (){
//     let x = 10;
//     println!("addr of x = {:p}", &x); 

//     let a1 = A{ a:10, b:'b'};
//     println!("addr of a1 = {:p}", &a1); 
//     println!("a1 = {:?}", a1);   
//     //浅拷贝
//     let a2 = a1;
//     println!("a1 = {:?}", a1);   
//     println!("a2 = {:?}", a2);   

//     let b1 = B{ a:a1, b:'b'};
//     println!("b1 = {:?}", b1);
//     //浅拷贝
//     let b2 = b1;
//     println!("b1 = {:?}", b1);   
//     println!("b2 = {:?}", b2);   

//     let s1 = "123";
//     let s2 = s1;
//     println!("s1 = {:?}", s1);  
//     println!("s2 = {:?}", s2);  
//     println!("addr of s1 = {:p}", &(*s1)); 

//     let box1 =  Box::new(321);
//     println!("addr of box1 = {:p}", &(*box1)); 
// }   


//demo2
fn takes_ownership(some_string: String) -> String { // some_string comes into scope
    println!("{}", some_string);
    some_string
} 
    
fn makes_copy(some_integer: i32) { // some_integer comes into scope
    println!("{}", some_integer);
} // Here, some_integer goes out of scope. Nothing special happens.
    


use std::rc::Rc;
fn main(){
    let x = Rc::new(10);
    let y1 = x.clone();
    let y2 = x.clone();
    //let y4 = y2.clone();
    println!("{:?}", Rc::strong_count(&x));
    let w = Rc::downgrade(&x);
    let y3 = &*x;
    println!("{:p}", y3);
    println!("{}", 100 - *x);
}
 