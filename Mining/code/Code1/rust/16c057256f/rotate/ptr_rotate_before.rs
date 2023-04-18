pub unsafe fn ptr_rotate<T>(mut left: usize, mid: *mut T, mut right: usize) {
    loop {
        let delta = cmp::min(left, right);
        if delta <= RawArray::<T>::cap() {
            break;
        }

        ptr::swap_nonoverlapping(
            mid.offset(-(left as isize)),
            mid.offset((right-delta) as isize),
            delta);

        if left <= right {
            right -= delta;
        } else {
            left -= delta;
        }
    }

    let rawarray = RawArray::new();
    let buf = rawarray.ptr();

    let dim = mid.offset(-(left as isize)).offset(right as isize);
    if left <= right {
        ptr::copy_nonoverlapping(mid.offset(-(left as isize)), buf, left);
        ptr::copy(mid, mid.offset(-(left as isize)), right);
        ptr::copy_nonoverlapping(buf, dim, left);
    }
    else {
        ptr::copy_nonoverlapping(mid, buf, right);
        ptr::copy(mid.offset(-(left as isize)), dim, left);
        ptr::copy_nonoverlapping(buf, mid.offset(-(left as isize)), right);
    }
}
