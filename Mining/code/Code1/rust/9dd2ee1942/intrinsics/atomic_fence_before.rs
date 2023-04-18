    pub fn atomic_fence();
    pub fn atomic_fence_acq();
    pub fn atomic_fence_rel();
    pub fn atomic_fence_acqrel();

    /// A compiler-only memory barrier.
    ///
    /// Memory accesses will never be reordered across this barrier by the
    /// compiler, but no instructions will be emitted for it. This is
    /// appropriate for operations on the same thread that may be preempted,
    /// such as when interacting with signal handlers.
    pub fn atomic_singlethreadfence();
    pub fn atomic_singlethreadfence_acq();
    pub fn atomic_singlethreadfence_rel();
    pub fn atomic_singlethreadfence_acqrel();

    /// Magic intrinsic that derives its meaning from attributes
    /// attached to the function.
    ///
    /// For example, dataflow uses this to inject static assertions so
    /// that `rustc_peek(potentially_uninitialized)` would actually
    /// double-check that dataflow did indeed compute that it is
    /// uninitialized at that point in the control flow.
    pub fn rustc_peek<T>(_: T) -> T;

    /// Aborts the execution of the process.
    pub fn abort() -> !;

    /// Tells LLVM that this point in the code is not reachable, enabling
    /// further optimizations.
    ///
    /// NB: This is very different from the `unreachable!()` macro: Unlike the
    /// macro, which panics when it is executed, it is *undefined behavior* to
    /// reach code marked with this function.
    pub fn unreachable() -> !;

    /// Informs the optimizer that a condition is always true.
    /// If the condition is false, the behavior is undefined.
    ///
    /// No code is generated for this intrinsic, but the optimizer will try
    /// to preserve it (and its condition) between passes, which may interfere
    /// with optimization of surrounding code and reduce performance. It should
    /// not be used if the invariant can be discovered by the optimizer on its
    /// own, or if it does not enable any significant optimizations.
    pub fn assume(b: bool);

    /// Hints to the compiler that branch condition is likely to be true.
    /// Returns the value passed to it.
    ///
    /// Any use other than with `if` statements will probably not have an effect.
    pub fn likely(b: bool) -> bool;

    /// Hints to the compiler that branch condition is likely to be false.
    /// Returns the value passed to it.
    ///
    /// Any use other than with `if` statements will probably not have an effect.
    pub fn unlikely(b: bool) -> bool;

    /// Executes a breakpoint trap, for inspection by a debugger.
    pub fn breakpoint();

    /// The size of a type in bytes.
    ///
    /// More specifically, this is the offset in bytes between successive
    /// items of the same type, including alignment padding.
    pub fn size_of<T>() -> usize;

    /// Moves a value to an uninitialized memory location.
    ///
    /// Drop glue is not run on the destination.
    pub fn move_val_init<T>(dst: *mut T, src: T);

    pub fn min_align_of<T>() -> usize;
    pub fn pref_align_of<T>() -> usize;

    pub fn size_of_val<T: ?Sized>(_: &T) -> usize;
    pub fn min_align_of_val<T: ?Sized>(_: &T) -> usize;

    /// Gets a static string slice containing the name of a type.
    pub fn type_name<T: ?Sized>() -> &'static str;

    /// Gets an identifier which is globally unique to the specified type. This
    /// function will return the same value for a type regardless of whichever
    /// crate it is invoked in.
    pub fn type_id<T: ?Sized + 'static>() -> u64;

    /// Creates a value initialized to zero.
    ///
    /// `init` is unsafe because it returns a zeroed-out datum,
    /// which is unsafe unless T is `Copy`.  Also, even if T is
    /// `Copy`, an all-zero value may not correspond to any legitimate
    /// state for the type in question.
    pub fn init<T>() -> T;

    /// Creates an uninitialized value.
    ///
    /// `uninit` is unsafe because there is no guarantee of what its
    /// contents are. In particular its drop-flag may be set to any
    /// state, which means it may claim either dropped or
    /// undropped. In the general case one must use `ptr::write` to
    /// initialize memory previous set to the result of `uninit`.
    pub fn uninit<T>() -> T;

    /// Reinterprets the bits of a value of one type as another type.
    ///
    /// Both types must have the same size. Neither the original, nor the result,
    /// may be an [invalid value](../../nomicon/meet-safe-and-unsafe.html).
    ///
    /// `transmute` is semantically equivalent to a bitwise move of one type
    /// into another. It copies the bits from the source value into the
    /// destination value, then forgets the original. It's equivalent to C's
    /// `memcpy` under the hood, just like `transmute_copy`.
    ///
    /// `transmute` is **incredibly** unsafe. There are a vast number of ways to
    /// cause [undefined behavior][ub] with this function. `transmute` should be
    /// the absolute last resort.
    ///
    /// The [nomicon](../../nomicon/transmutes.html) has additional
    /// documentation.
    ///
    /// [ub]: ../../reference/behavior-considered-undefined.html
    ///
    /// # Examples
    ///
    /// There are a few things that `transmute` is really useful for.
    ///
    /// Getting the bitpattern of a floating point type (or, more generally,
    /// type punning, when `T` and `U` aren't pointers):
    ///
    /// ```
    /// let bitpattern = unsafe {
    ///     std::mem::transmute::<f32, u32>(1.0)
    /// };
    /// assert_eq!(bitpattern, 0x3F800000);
    /// ```
    ///
    /// Turning a pointer into a function pointer. This is *not* portable to
    /// machines where function pointers and data pointers have different sizes.
    ///
    /// ```
    /// fn foo() -> i32 {
    ///     0
    /// }
    /// let pointer = foo as *const ();
    /// let function = unsafe {
    ///     std::mem::transmute::<*const (), fn() -> i32>(pointer)
    /// };
    /// assert_eq!(function(), 0);
    /// ```
    ///
    /// Extending a lifetime, or shortening an invariant lifetime. This is
    /// advanced, very unsafe Rust!
    ///
    /// ```
    /// struct R<'a>(&'a i32);
    /// unsafe fn extend_lifetime<'b>(r: R<'b>) -> R<'static> {
    ///     std::mem::transmute::<R<'b>, R<'static>>(r)
    /// }
    ///
    /// unsafe fn shorten_invariant_lifetime<'b, 'c>(r: &'b mut R<'static>)
    ///                                              -> &'b mut R<'c> {
    ///     std::mem::transmute::<&'b mut R<'static>, &'b mut R<'c>>(r)
    /// }
    /// ```
    ///
    /// # Alternatives
    ///
    /// Don't despair: many uses of `transmute` can be achieved through other means.
    /// Below are common applications of `transmute` which can be replaced with safer
    /// constructs.
    ///
    /// Turning a pointer into a `usize`:
    ///
    /// ```
    /// let ptr = &0;
    /// let ptr_num_transmute = unsafe {
    ///     std::mem::transmute::<&i32, usize>(ptr)
    /// };
    ///
    /// // Use an `as` cast instead
    /// let ptr_num_cast = ptr as *const i32 as usize;
    /// ```
    ///
    /// Turning a `*mut T` into an `&mut T`:
    ///
    /// ```
    /// let ptr: *mut i32 = &mut 0;
    /// let ref_transmuted = unsafe {
    ///     std::mem::transmute::<*mut i32, &mut i32>(ptr)
    /// };
    ///
    /// // Use a reborrow instead
    /// let ref_casted = unsafe { &mut *ptr };
    /// ```
    ///
    /// Turning an `&mut T` into an `&mut U`:
    ///
    /// ```
    /// let ptr = &mut 0;
    /// let val_transmuted = unsafe {
    ///     std::mem::transmute::<&mut i32, &mut u32>(ptr)
    /// };
    ///
    /// // Now, put together `as` and reborrowing - note the chaining of `as`
    /// // `as` is not transitive
    /// let val_casts = unsafe { &mut *(ptr as *mut i32 as *mut u32) };
    /// ```
    ///
    /// Turning an `&str` into an `&[u8]`:
    ///
    /// ```
    /// // this is not a good way to do this.
    /// let slice = unsafe { std::mem::transmute::<&str, &[u8]>("Rust") };
    /// assert_eq!(slice, &[82, 117, 115, 116]);
    ///
    /// // You could use `str::as_bytes`
    /// let slice = "Rust".as_bytes();
    /// assert_eq!(slice, &[82, 117, 115, 116]);
    ///
    /// // Or, just use a byte string, if you have control over the string
    /// // literal
    /// assert_eq!(b"Rust", &[82, 117, 115, 116]);
    /// ```
    ///
    /// Turning a `Vec<&T>` into a `Vec<Option<&T>>`:
    ///
    /// ```
    /// let store = [0, 1, 2, 3];
    /// let mut v_orig = store.iter().collect::<Vec<&i32>>();
    ///
    /// // Using transmute: this is Undefined Behavior, and a bad idea.
    /// // However, it is no-copy.
    /// let v_transmuted = unsafe {
    ///     std::mem::transmute::<Vec<&i32>, Vec<Option<&i32>>>(
    ///         v_orig.clone())
    /// };
    ///
    /// // This is the suggested, safe way.
    /// // It does copy the entire vector, though, into a new array.
    /// let v_collected = v_orig.clone()
    ///                         .into_iter()
    ///                         .map(|r| Some(r))
    ///                         .collect::<Vec<Option<&i32>>>();
    ///
    /// // The no-copy, unsafe way, still using transmute, but not UB.
    /// // This is equivalent to the original, but safer, and reuses the
    /// // same Vec internals. Therefore the new inner type must have the
    /// // exact same size, and the same or lesser alignment, as the old
    /// // type. The same caveats exist for this method as transmute, for
    /// // the original inner type (`&i32`) to the converted inner type
    /// // (`Option<&i32>`), so read the nomicon pages linked above.
    /// let v_from_raw = unsafe {
    ///     Vec::from_raw_parts(v_orig.as_mut_ptr(),
    ///                         v_orig.len(),
    ///                         v_orig.capacity())
    /// };
    /// std::mem::forget(v_orig);
    /// ```
    ///
    /// Implementing `split_at_mut`:
    ///
    /// ```
    /// use std::{slice, mem};
    ///
    /// // There are multiple ways to do this; and there are multiple problems
    /// // with the following, transmute, way.
    /// fn split_at_mut_transmute<T>(slice: &mut [T], mid: usize)
    ///                              -> (&mut [T], &mut [T]) {
    ///     let len = slice.len();
    ///     assert!(mid <= len);
    ///     unsafe {
    ///         let slice2 = mem::transmute::<&mut [T], &mut [T]>(slice);
    ///         // first: transmute is not typesafe; all it checks is that T and
    ///         // U are of the same size. Second, right here, you have two
    ///         // mutable references pointing to the same memory.
    ///         (&mut slice[0..mid], &mut slice2[mid..len])
    ///     }
    /// }
    ///
    /// // This gets rid of the typesafety problems; `&mut *` will *only* give
    /// // you an `&mut T` from an `&mut T` or `*mut T`.
    /// fn split_at_mut_casts<T>(slice: &mut [T], mid: usize)
    ///                          -> (&mut [T], &mut [T]) {
    ///     let len = slice.len();
    ///     assert!(mid <= len);
    ///     unsafe {
    ///         let slice2 = &mut *(slice as *mut [T]);
    ///         // however, you still have two mutable references pointing to
    ///         // the same memory.
    ///         (&mut slice[0..mid], &mut slice2[mid..len])
    ///     }
    /// }
    ///
    /// // This is how the standard library does it. This is the best method, if
    /// // you need to do something like this
    /// fn split_at_stdlib<T>(slice: &mut [T], mid: usize)
    ///                       -> (&mut [T], &mut [T]) {
    ///     let len = slice.len();
    ///     assert!(mid <= len);
    ///     unsafe {
    ///         let ptr = slice.as_mut_ptr();
    ///         // This now has three mutable references pointing at the same
    ///         // memory. `slice`, the rvalue ret.0, and the rvalue ret.1.
    ///         // `slice` is never used after `let ptr = ...`, and so one can
    ///         // treat it as "dead", and therefore, you only have two real
    ///         // mutable slices.
    ///         (slice::from_raw_parts_mut(ptr, mid),
    ///          slice::from_raw_parts_mut(ptr.offset(mid as isize), len - mid))
    ///     }
    /// }
    /// ```
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn transmute<T, U>(e: T) -> U;

    /// Returns `true` if the actual type given as `T` requires drop
    /// glue; returns `false` if the actual type provided for `T`
    /// implements `Copy`.
    ///
    /// If the actual type neither requires drop glue nor implements
    /// `Copy`, then may return `true` or `false`.
    pub fn needs_drop<T>() -> bool;

    /// Calculates the offset from a pointer.
    ///
    /// This is implemented as an intrinsic to avoid converting to and from an
    /// integer, since the conversion would throw away aliasing information.
    ///
    /// # Safety
    ///
    /// Both the starting and resulting pointer must be either in bounds or one
    /// byte past the end of an allocated object. If either pointer is out of
    /// bounds or arithmetic overflow occurs then any further use of the
    /// returned value will result in undefined behavior.
    pub fn offset<T>(dst: *const T, offset: isize) -> *const T;

    /// Calculates the offset from a pointer, potentially wrapping.
    ///
    /// This is implemented as an intrinsic to avoid converting to and from an
    /// integer, since the conversion inhibits certain optimizations.
    ///
    /// # Safety
    ///
    /// Unlike the `offset` intrinsic, this intrinsic does not restrict the
    /// resulting pointer to point into or one byte past the end of an allocated
    /// object, and it wraps with two's complement arithmetic. The resulting
    /// value is not necessarily valid to be used to actually access memory.
    pub fn arith_offset<T>(dst: *const T, offset: isize) -> *const T;

    /// Copies `count * size_of<T>` bytes from `src` to `dst`. The source
    /// and destination may *not* overlap.
    ///
    /// `copy_nonoverlapping` is semantically equivalent to C's `memcpy`.
    ///
    /// # Safety
    ///
    /// Beyond requiring that the program must be allowed to access both regions
    /// of memory, it is Undefined Behavior for source and destination to
    /// overlap. Care must also be taken with the ownership of `src` and
    /// `dst`. This method semantically moves the values of `src` into `dst`.
    /// However it does not drop the contents of `dst`, or prevent the contents
    /// of `src` from being dropped or used.
    ///
    /// # Examples
    ///
    /// A safe swap function:
    ///
    /// ```
    /// use std::mem;
    /// use std::ptr;
    ///
    /// # #[allow(dead_code)]
    /// fn swap<T>(x: &mut T, y: &mut T) {
    ///     unsafe {
    ///         // Give ourselves some scratch space to work with
    ///         let mut t: T = mem::uninitialized();
    ///
    ///         // Perform the swap, `&mut` pointers never alias
    ///         ptr::copy_nonoverlapping(x, &mut t, 1);
    ///         ptr::copy_nonoverlapping(y, x, 1);
    ///         ptr::copy_nonoverlapping(&t, y, 1);
    ///
    ///         // y and t now point to the same thing, but we need to completely forget `tmp`
    ///         // because it's no longer relevant.
    ///         mem::forget(t);
    ///     }
    /// }
    /// ```
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn copy_nonoverlapping<T>(src: *const T, dst: *mut T, count: usize);

    /// Copies `count * size_of<T>` bytes from `src` to `dst`. The source
    /// and destination may overlap.
    ///
    /// `copy` is semantically equivalent to C's `memmove`.
    ///
    /// # Safety
    ///
    /// Care must be taken with the ownership of `src` and `dst`.
    /// This method semantically moves the values of `src` into `dst`.
    /// However it does not drop the contents of `dst`, or prevent the contents of `src`
    /// from being dropped or used.
    ///
    /// # Examples
    ///
    /// Efficiently create a Rust vector from an unsafe buffer:
    ///
    /// ```
    /// use std::ptr;
    ///
    /// # #[allow(dead_code)]
    /// unsafe fn from_buf_raw<T>(ptr: *const T, elts: usize) -> Vec<T> {
    ///     let mut dst = Vec::with_capacity(elts);
    ///     dst.set_len(elts);
    ///     ptr::copy(ptr, dst.as_mut_ptr(), elts);
    ///     dst
    /// }
    /// ```
    ///
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn copy<T>(src: *const T, dst: *mut T, count: usize);

    /// Invokes memset on the specified pointer, setting `count * size_of::<T>()`
    /// bytes of memory starting at `dst` to `val`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::ptr;
    ///
    /// let mut vec = vec![0; 4];
    /// unsafe {
    ///     let vec_ptr = vec.as_mut_ptr();
    ///     ptr::write_bytes(vec_ptr, b'a', 2);
    /// }
    /// assert_eq!(vec, [b'a', b'a', 0, 0]);
    /// ```
    #[stable(feature = "rust1", since = "1.0.0")]
    pub fn write_bytes<T>(dst: *mut T, val: u8, count: usize);

    /// Equivalent to the appropriate `llvm.memcpy.p0i8.0i8.*` intrinsic, with
    /// a size of `count` * `size_of::<T>()` and an alignment of
    /// `min_align_of::<T>()`
    ///
    /// The volatile parameter is set to `true`, so it will not be optimized out
    /// unless size is equal to zero.
    pub fn volatile_copy_nonoverlapping_memory<T>(dst: *mut T, src: *const T,
                                                  count: usize);
    /// Equivalent to the appropriate `llvm.memmove.p0i8.0i8.*` intrinsic, with
    /// a size of `count` * `size_of::<T>()` and an alignment of
    /// `min_align_of::<T>()`
    ///
    /// The volatile parameter is set to `true`, so it will not be optimized out
    /// unless size is equal to zero..
    pub fn volatile_copy_memory<T>(dst: *mut T, src: *const T, count: usize);
    /// Equivalent to the appropriate `llvm.memset.p0i8.*` intrinsic, with a
    /// size of `count` * `size_of::<T>()` and an alignment of
    /// `min_align_of::<T>()`.
    ///
    /// The volatile parameter is set to `true`, so it will not be optimized out
    /// unless size is equal to zero.
    pub fn volatile_set_memory<T>(dst: *mut T, val: u8, count: usize);

    /// Perform a volatile load from the `src` pointer.
    /// The stabilized version of this intrinsic is
    /// [`std::ptr::read_volatile`](../../std/ptr/fn.read_volatile.html).
    pub fn volatile_load<T>(src: *const T) -> T;
    /// Perform a volatile store to the `dst` pointer.
    /// The stabilized version of this intrinsic is
    /// [`std::ptr::write_volatile`](../../std/ptr/fn.write_volatile.html).
    pub fn volatile_store<T>(dst: *mut T, val: T);

    /// Returns the square root of an `f32`
    pub fn sqrtf32(x: f32) -> f32;
    /// Returns the square root of an `f64`
    pub fn sqrtf64(x: f64) -> f64;

    /// Raises an `f32` to an integer power.
    pub fn powif32(a: f32, x: i32) -> f32;
    /// Raises an `f64` to an integer power.
    pub fn powif64(a: f64, x: i32) -> f64;

    /// Returns the sine of an `f32`.
    pub fn sinf32(x: f32) -> f32;
    /// Returns the sine of an `f64`.
    pub fn sinf64(x: f64) -> f64;

    /// Returns the cosine of an `f32`.
    pub fn cosf32(x: f32) -> f32;
    /// Returns the cosine of an `f64`.
    pub fn cosf64(x: f64) -> f64;

    /// Raises an `f32` to an `f32` power.
    pub fn powf32(a: f32, x: f32) -> f32;
    /// Raises an `f64` to an `f64` power.
    pub fn powf64(a: f64, x: f64) -> f64;

    /// Returns the exponential of an `f32`.
    pub fn expf32(x: f32) -> f32;
    /// Returns the exponential of an `f64`.
    pub fn expf64(x: f64) -> f64;

    /// Returns 2 raised to the power of an `f32`.
    pub fn exp2f32(x: f32) -> f32;
    /// Returns 2 raised to the power of an `f64`.
    pub fn exp2f64(x: f64) -> f64;

    /// Returns the natural logarithm of an `f32`.
    pub fn logf32(x: f32) -> f32;
    /// Returns the natural logarithm of an `f64`.
    pub fn logf64(x: f64) -> f64;

    /// Returns the base 10 logarithm of an `f32`.
    pub fn log10f32(x: f32) -> f32;
    /// Returns the base 10 logarithm of an `f64`.
    pub fn log10f64(x: f64) -> f64;

    /// Returns the base 2 logarithm of an `f32`.
    pub fn log2f32(x: f32) -> f32;
    /// Returns the base 2 logarithm of an `f64`.
    pub fn log2f64(x: f64) -> f64;

    /// Returns `a * b + c` for `f32` values.
    pub fn fmaf32(a: f32, b: f32, c: f32) -> f32;
    /// Returns `a * b + c` for `f64` values.
    pub fn fmaf64(a: f64, b: f64, c: f64) -> f64;

    /// Returns the absolute value of an `f32`.
    pub fn fabsf32(x: f32) -> f32;
    /// Returns the absolute value of an `f64`.
    pub fn fabsf64(x: f64) -> f64;

    /// Copies the sign from `y` to `x` for `f32` values.
    pub fn copysignf32(x: f32, y: f32) -> f32;
    /// Copies the sign from `y` to `x` for `f64` values.
    pub fn copysignf64(x: f64, y: f64) -> f64;

    /// Returns the largest integer less than or equal to an `f32`.
    pub fn floorf32(x: f32) -> f32;
    /// Returns the largest integer less than or equal to an `f64`.
    pub fn floorf64(x: f64) -> f64;

    /// Returns the smallest integer greater than or equal to an `f32`.
    pub fn ceilf32(x: f32) -> f32;
    /// Returns the smallest integer greater than or equal to an `f64`.
    pub fn ceilf64(x: f64) -> f64;

    /// Returns the integer part of an `f32`.
    pub fn truncf32(x: f32) -> f32;
    /// Returns the integer part of an `f64`.
    pub fn truncf64(x: f64) -> f64;

    /// Returns the nearest integer to an `f32`. May raise an inexact floating-point exception
    /// if the argument is not an integer.
    pub fn rintf32(x: f32) -> f32;
    /// Returns the nearest integer to an `f64`. May raise an inexact floating-point exception
    /// if the argument is not an integer.
    pub fn rintf64(x: f64) -> f64;

    /// Returns the nearest integer to an `f32`.
    pub fn nearbyintf32(x: f32) -> f32;
    /// Returns the nearest integer to an `f64`.
    pub fn nearbyintf64(x: f64) -> f64;

    /// Returns the nearest integer to an `f32`. Rounds half-way cases away from zero.
    pub fn roundf32(x: f32) -> f32;
    /// Returns the nearest integer to an `f64`. Rounds half-way cases away from zero.
    pub fn roundf64(x: f64) -> f64;

    /// Float addition that allows optimizations based on algebraic rules.
    /// May assume inputs are finite.
    pub fn fadd_fast<T>(a: T, b: T) -> T;

    /// Float subtraction that allows optimizations based on algebraic rules.
    /// May assume inputs are finite.
    pub fn fsub_fast<T>(a: T, b: T) -> T;

    /// Float multiplication that allows optimizations based on algebraic rules.
    /// May assume inputs are finite.
    pub fn fmul_fast<T>(a: T, b: T) -> T;

    /// Float division that allows optimizations based on algebraic rules.
    /// May assume inputs are finite.
    pub fn fdiv_fast<T>(a: T, b: T) -> T;

    /// Float remainder that allows optimizations based on algebraic rules.
    /// May assume inputs are finite.
    pub fn frem_fast<T>(a: T, b: T) -> T;


    /// Returns the number of bits set in an integer type `T`
    pub fn ctpop<T>(x: T) -> T;

    /// Returns the number of leading unset bits (zeroes) in an integer type `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(core_intrinsics)]
    ///
    /// use std::intrinsics::ctlz;
    ///
    /// let x = 0b0001_1100_u8;
    /// let num_leading = unsafe { ctlz(x) };
    /// assert_eq!(num_leading, 3);
    /// ```
    ///
    /// An `x` with value `0` will return the bit width of `T`.
    ///
    /// ```
    /// #![feature(core_intrinsics)]
    ///
    /// use std::intrinsics::ctlz;
    ///
    /// let x = 0u16;
    /// let num_leading = unsafe { ctlz(x) };
    /// assert_eq!(num_leading, 16);
    /// ```
    pub fn ctlz<T>(x: T) -> T;

    /// Like `ctlz`, but extra-unsafe as it returns `undef` when
    /// given an `x` with value `0`.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(core_intrinsics)]
    ///
    /// use std::intrinsics::ctlz_nonzero;
    ///
    /// let x = 0b0001_1100_u8;
    /// let num_leading = unsafe { ctlz_nonzero(x) };
    /// assert_eq!(num_leading, 3);
    /// ```
    pub fn ctlz_nonzero<T>(x: T) -> T;

    /// Returns the number of trailing unset bits (zeroes) in an integer type `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(core_intrinsics)]
    ///
    /// use std::intrinsics::cttz;
    ///
    /// let x = 0b0011_1000_u8;
    /// let num_trailing = unsafe { cttz(x) };
    /// assert_eq!(num_trailing, 3);
    /// ```
    ///
    /// An `x` with value `0` will return the bit width of `T`:
    ///
    /// ```
    /// #![feature(core_intrinsics)]
    ///
    /// use std::intrinsics::cttz;
    ///
    /// let x = 0u16;
    /// let num_trailing = unsafe { cttz(x) };
    /// assert_eq!(num_trailing, 16);
    /// ```
    pub fn cttz<T>(x: T) -> T;

    /// Like `cttz`, but extra-unsafe as it returns `undef` when
    /// given an `x` with value `0`.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(core_intrinsics)]
    ///
    /// use std::intrinsics::cttz_nonzero;
    ///
    /// let x = 0b0011_1000_u8;
    /// let num_trailing = unsafe { cttz_nonzero(x) };
    /// assert_eq!(num_trailing, 3);
    /// ```
    pub fn cttz_nonzero<T>(x: T) -> T;

    /// Reverses the bytes in an integer type `T`.
    pub fn bswap<T>(x: T) -> T;

    /// Performs checked integer addition.
    /// The stabilized versions of this intrinsic are available on the integer
    /// primitives via the `overflowing_add` method. For example,
    /// [`std::u32::overflowing_add`](../../std/primitive.u32.html#method.overflowing_add)
    pub fn add_with_overflow<T>(x: T, y: T) -> (T, bool);

    /// Performs checked integer subtraction
    /// The stabilized versions of this intrinsic are available on the integer
    /// primitives via the `overflowing_sub` method. For example,
    /// [`std::u32::overflowing_sub`](../../std/primitive.u32.html#method.overflowing_sub)
    pub fn sub_with_overflow<T>(x: T, y: T) -> (T, bool);

    /// Performs checked integer multiplication
    /// The stabilized versions of this intrinsic are available on the integer
    /// primitives via the `overflowing_mul` method. For example,
    /// [`std::u32::overflowing_mul`](../../std/primitive.u32.html#method.overflowing_mul)
    pub fn mul_with_overflow<T>(x: T, y: T) -> (T, bool);

    /// Performs an unchecked division, resulting in undefined behavior
    /// where y = 0 or x = `T::min_value()` and y = -1
    pub fn unchecked_div<T>(x: T, y: T) -> T;
    /// Returns the remainder of an unchecked division, resulting in
    /// undefined behavior where y = 0 or x = `T::min_value()` and y = -1
    pub fn unchecked_rem<T>(x: T, y: T) -> T;

    /// Performs an unchecked left shift, resulting in undefined behavior when
    /// y < 0 or y >= N, where N is the width of T in bits.
    pub fn unchecked_shl<T>(x: T, y: T) -> T;
    /// Performs an unchecked right shift, resulting in undefined behavior when
    /// y < 0 or y >= N, where N is the width of T in bits.
    pub fn unchecked_shr<T>(x: T, y: T) -> T;

    /// Returns (a + b) mod 2<sup>N</sup>, where N is the width of T in bits.
    /// The stabilized versions of this intrinsic are available on the integer
    /// primitives via the `wrapping_add` method. For example,
    /// [`std::u32::wrapping_add`](../../std/primitive.u32.html#method.wrapping_add)
    pub fn overflowing_add<T>(a: T, b: T) -> T;
    /// Returns (a - b) mod 2<sup>N</sup>, where N is the width of T in bits.
    /// The stabilized versions of this intrinsic are available on the integer
    /// primitives via the `wrapping_sub` method. For example,
    /// [`std::u32::wrapping_sub`](../../std/primitive.u32.html#method.wrapping_sub)
    pub fn overflowing_sub<T>(a: T, b: T) -> T;
    /// Returns (a * b) mod 2<sup>N</sup>, where N is the width of T in bits.
    /// The stabilized versions of this intrinsic are available on the integer
    /// primitives via the `wrapping_mul` method. For example,
    /// [`std::u32::wrapping_mul`](../../std/primitive.u32.html#method.wrapping_mul)
    pub fn overflowing_mul<T>(a: T, b: T) -> T;

    /// Returns the value of the discriminant for the variant in 'v',
    /// cast to a `u64`; if `T` has no discriminant, returns 0.
    pub fn discriminant_value<T>(v: &T) -> u64;

    /// Rust's "try catch" construct which invokes the function pointer `f` with
    /// the data pointer `data`.
    ///
    /// The third pointer is a target-specific data pointer which is filled in
    /// with the specifics of the exception that occurred. For examples on Unix
    /// platforms this is a `*mut *mut T` which is filled in by the compiler and
    /// on MSVC it's `*mut [usize; 2]`. For more information see the compiler's
    /// source as well as std's catch implementation.
    pub fn try(f: fn(*mut u8), data: *mut u8, local_ptr: *mut u8) -> i32;

    /// Computes the byte offset that needs to be applied to `ptr` in order to
    /// make it aligned to `align`.
    /// If it is not possible to align `ptr`, the implementation returns
    /// `usize::max_value()`.
    ///
    /// There are no guarantees whatsover that offsetting the pointer will not
    /// overflow or go beyond the allocation that `ptr` points into.
    /// It is up to the caller to ensure that the returned offset is correct
    /// in all terms other than alignment.
    ///
    /// # Examples
    ///
    /// Accessing adjacent `u8` as `u16`
    ///
    /// ```
    /// # #![feature(core_intrinsics)]
    /// # fn foo(n: usize) {
    /// # use std::intrinsics::align_offset;
    /// # use std::mem::align_of;
    /// # unsafe {
    /// let x = [5u8, 6u8, 7u8, 8u8, 9u8];
    /// let ptr = &x[n] as *const u8;
    /// let offset = align_offset(ptr as *const (), align_of::<u16>());
    /// if offset < x.len() - n - 1 {
    ///     let u16_ptr = ptr.offset(offset as isize) as *const u16;
    ///     assert_ne!(*u16_ptr, 500);
    /// } else {
    ///     // while the pointer can be aligned via `offset`, it would point
    ///     // outside the allocation
    /// }
    /// # } }
    /// ```
    #[cfg(not(stage0))]
    pub fn align_offset(ptr: *const (), align: usize) -> usize;
}

#[cfg(stage0)]
/// Computes the byte offset that needs to be applied to `ptr` in order to
/// make it aligned to `align`.
/// If it is not possible to align `ptr`, the implementation returns
/// `usize::max_value()`.
///
/// There are no guarantees whatsover that offsetting the pointer will not
/// overflow or go beyond the allocation that `ptr` points into.
/// It is up to the caller to ensure that the returned offset is correct
/// in all terms other than alignment.
///
/// # Examples
///
/// Accessing adjacent `u8` as `u16`
///
/// ```
/// # #![feature(core_intrinsics)]
/// # fn foo(n: usize) {
/// # use std::intrinsics::align_offset;
/// # use std::mem::align_of;
/// # unsafe {
/// let x = [5u8, 6u8, 7u8, 8u8, 9u8];
/// let ptr = &x[n] as *const u8;
/// let offset = align_offset(ptr as *const (), align_of::<u16>());
/// if offset < x.len() - n - 1 {
///     let u16_ptr = ptr.offset(offset as isize) as *const u16;
///     assert_ne!(*u16_ptr, 500);
/// } else {
///     // while the pointer can be aligned via `offset`, it would point
///     // outside the allocation
/// }
/// # } }
/// ```
pub unsafe fn align_offset(ptr: *const (), align: usize) -> usize {
    let offset = ptr as usize % align;
    if offset == 0 {
        0
    } else {
        align - offset
    }
}
