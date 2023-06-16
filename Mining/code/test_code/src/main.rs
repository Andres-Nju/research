use libc::{
    off_t,
    ioctl,
    c_ulong,
};
use std::io;
use std::cell::Cell;
pub fn set_nonblocking(nonblocking: bool) -> i32 {
    let mut nonblocking = nonblocking as libc::c_ulong;
    unsafe { libc::ioctl(1,libc::FIONBIO, &mut nonblocking) }
}
fn main(){
    
}