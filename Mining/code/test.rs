struct A{
    a: i32,
}
impl A{
    fn test(&self){
        println!("a = {}", self.a);
    }
}

fn main(){
    let a: usize = 4;
    let b: u32 = 5;
    let c = a as u64;
}