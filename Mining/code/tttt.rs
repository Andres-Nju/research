fn main(){
    let mut v = vec ![10 , 11];
    let v2 = & mut v;
    let vptr = & mut (* v2 )[1];
    println !("v[1] = {}", * vptr );
    Vec :: push (v2 , 12);
    println !("v[1] = {}", * vptr );
}