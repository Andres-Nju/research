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
fn main (){
    let x = 10;
    println!("addr of x = {:p}", &x); 

    let a1 = A{ a:10, b:'b'};
    println!("addr of a1 = {:p}", &a1); 
    println!("a1 = {:?}", a1);   
    //浅拷贝
    let a2 = a1;
    println!("a1 = {:?}", a1);   
    println!("a2 = {:?}", a2);   

    let b1 = B{ a:a1, b:'b'};
    println!("b1 = {:?}", b1);
    //浅拷贝
    let b2 = b1;
    println!("b1 = {:?}", b1);   
    println!("b2 = {:?}", b2);   

    let s1 = "123";
    let s2 = s1;
    println!("s1 = {:?}", s1);  
    println!("s2 = {:?}", s2);  
}   