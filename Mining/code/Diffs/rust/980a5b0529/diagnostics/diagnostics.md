File_Code/rust/980a5b0529/diagnostics/diagnostics_after.rs --- Rust
303 E0492: r##"                                                                                                                                              303 E0492: r##"
304 A borrow of a constant containing interior mutability was attempted. Erroneous                                                                           304 A borrow of a constant containing interior mutability was attempted. Erroneous
305 code example:                                                                                                                                            305 code example:
306                                                                                                                                                          306 
307 ```compile_fail,E0492                                                                                                                                    307 ```compile_fail,E0492
308 use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT};                                                                                                 308 use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT};
309                                                                                                                                                          309 
310 const A: AtomicUsize = ATOMIC_USIZE_INIT;                                                                                                                310 const A: AtomicUsize = ATOMIC_USIZE_INIT;
311 static B: &'static AtomicUsize = &A;                                                                                                                     311 static B: &'static AtomicUsize = &A;
312 // error: cannot borrow a constant which may contain interior mutability,                                                                                312 // error: cannot borrow a constant which may contain interior mutability,
313 //        create a static instead                                                                                                                        313 //        create a static instead
314 ```                                                                                                                                                      314 ```
315                                                                                                                                                          315 
316 A `const` represents a constant value that should never change. If one takes                                                                             316 A `const` represents a constant value that should never change. If one takes
317 a `&` reference to the constant, then one is taking a pointer to some memory                                                                             317 a `&` reference to the constant, then one is taking a pointer to some memory
318 location containing the value. Normally this is perfectly fine: most values                                                                              318 location containing the value. Normally this is perfectly fine: most values
319 can't be changed via a shared `&` pointer, but interior mutability would allow                                                                           319 can't be changed via a shared `&` pointer, but interior mutability would allow
320 it. That is, a constant value could be mutated. On the other hand, a `static` is                                                                         320 it. That is, a constant value could be mutated. On the other hand, a `static` is
321 explicitly a single memory location, which can be mutated at will.                                                                                       321 explicitly a single memory location, which can be mutated at will.
322                                                                                                                                                          322 
323 So, in order to solve this error, either use statics which are `Sync`:                                                                                   323 So, in order to solve this error, either use statics which are `Sync`:
324                                                                                                                                                          324 
325 ```                                                                                                                                                      325 ```
326 use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT};                                                                                                 326 use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT};
327                                                                                                                                                          327 
328 static A: AtomicUsize = ATOMIC_USIZE_INIT;                                                                                                               328 static A: AtomicUsize = ATOMIC_USIZE_INIT;
329 static B: &'static AtomicUsize = &A; // ok!                                                                                                              329 static B: &'static AtomicUsize = &A; // ok!
330 ```                                                                                                                                                      330 ```
331                                                                                                                                                          331 
332 You can also have this error while using a cell type:                                                                                                    332 You can also have this error while using a cell type:
333                                                                                                                                                          333 
334 ```compile_fail,E0492                                                                                                                                    334 ```compile_fail,E0492
335 #![feature(const_fn)]                                                                                                                                    335 #![feature(const_fn)]
336                                                                                                                                                          336 
337 use std::cell::Cell;                                                                                                                                     337 use std::cell::Cell;
338                                                                                                                                                          338 
339 const A: Cell<usize> = Cell::new(1);                                                                                                                     339 const A: Cell<usize> = Cell::new(1);
340 const B: &'static Cell<usize> = &A;                                                                                                                      340 const B: &'static Cell<usize> = &A;
341 // error: cannot borrow a constant which may contain interior mutability,                                                                                341 // error: cannot borrow a constant which may contain interior mutability,
342 //        create a static instead                                                                                                                        342 //        create a static instead
343                                                                                                                                                          343 
344 // or:                                                                                                                                                   344 // or:
345 struct C { a: Cell<usize> }                                                                                                                              345 struct C { a: Cell<usize> }
346                                                                                                                                                          346 
347 const D: C = C { a: Cell::new(1) };                                                                                                                      347 const D: C = C { a: Cell::new(1) };
348 const E: &'static Cell<usize> = &D.a; // error                                                                                                           348 const E: &'static Cell<usize> = &D.a; // error
349                                                                                                                                                          349 
350 // or:                                                                                                                                                   350 // or:
351 const F: &'static C = &D; // error                                                                                                                       351 const F: &'static C = &D; // error
352 ```                                                                                                                                                      352 ```
353                                                                                                                                                          353 
354 This is because cell types do operations that are not thread-safe. Due to this,                                                                          354 This is because cell types do operations that are not thread-safe. Due to this,
355 they don't implement Sync and thus can't be placed in statics. In this                                                                                   355 they don't implement Sync and thus can't be placed in statics. In this
356 case, `StaticMutex` would work just fine, but it isn't stable yet:                                                                                       356 case, `StaticMutex` would work just fine, but it isn't stable yet:
357 https://doc.rust-lang.org/nightly/std/sync/struct.StaticMutex.html                                                                                       357 https://doc.rust-lang.org/nightly/std/sync/struct.StaticMutex.html
358                                                                                                                                                          358 
359 However, if you still wish to use these types, you can achieve this by an unsafe                                                                         359 However, if you still wish to use these types, you can achieve this by an unsafe
360 wrapper:                                                                                                                                                 360 wrapper:
361                                                                                                                                                          361 
362 ```                                                                                                                                                      362 ```
363 #![feature(const_fn)]                                                                                                                                    363 #![feature(const_fn)]
364                                                                                                                                                          364 
365 use std::cell::Cell;                                                                                                                                     365 use std::cell::Cell;
366 use std::marker::Sync;                                                                                                                                   366 use std::marker::Sync;
367                                                                                                                                                          367 
368 struct NotThreadSafe<T> {                                                                                                                                368 struct NotThreadSafe<T> {
369     value: Cell<T>,                                                                                                                                      369     value: Cell<T>,
370 }                                                                                                                                                        370 }
371                                                                                                                                                          371 
372 unsafe impl<T> Sync for NotThreadSafe<T> {}                                                                                                              372 unsafe impl<T> Sync for NotThreadSafe<T> {}
373                                                                                                                                                          373 
374 static A: NotThreadSafe<usize> = NotThreadSafe { value : Cell::new(1) };                                                                                 374 static A: NotThreadSafe<usize> = NotThreadSafe { value : Cell::new(1) };
375 static B: &'static NotThreadSafe<usize> = &A; // ok!                                                                                                     375 static B: &'static NotThreadSafe<usize> = &A; // ok!
376 ```                                                                                                                                                      376 ```
377                                                                                                                                                          377 
378 Remember this solution is unsafe! You will have to ensure that accesses to the                                                                           378 Remember this solution is unsafe! You will have to ensure that accesses to the
379 cell are synchronized.                                                                                                                                   379 cell are synchronized.
380 "##,                                                                                                                                                     380 "##,

