fn transmute<T, U>(t: T) -> U {
    let Helper::U(u) = Helper::T(t, []); //~ ERROR refutable pattern in local binding: `T(_, _)` not covered
    u
}
