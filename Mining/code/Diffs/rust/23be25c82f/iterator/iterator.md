File_Code/rust/23be25c82f/iterator/iterator_after.rs --- Rust
204     /// This method will evaluate the iterator until its [`next`] returns                                                                                204     /// This method will call [`next`] repeatedly until [`None`] is encountered,
205     /// [`None`]. Once [`None`] is encountered, `count()` returns one less than the                                                                      ... 
206     /// number of times it called [`next`]. Note that [`next`] has to be called at                                                                       205     /// returning the number of times it saw [`Some`]. Note that [`next`] has to be
207     /// least once even if the iterator does not have any elements.                                                                                      206     /// called at least once even if the iterator does not have any elements.
208     ///                                                                                                                                                  207     ///
209     /// [`next`]: #tymethod.next                                                                                                                         208     /// [`next`]: #tymethod.next
210     /// [`None`]: ../../std/option/enum.Option.html#variant.None                                                                                         209     /// [`None`]: ../../std/option/enum.Option.html#variant.None
                                                                                                                                                             210     /// [`Some`]: ../../std/option/enum.Option.html#variant.Some

