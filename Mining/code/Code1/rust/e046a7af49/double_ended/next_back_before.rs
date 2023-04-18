    fn next_back(&mut self) -> Option<Self::Item>;

    /// Returns the `n`th element from the end of the iterator.
    ///
    /// This is essentially the reversed version of [`nth`]. Although like most indexing
    /// operations, the count starts from zero, so `nth_back(0)` returns the first value fro
    /// the end, `nth_back(1)` the second, and so on.
    ///
    /// Note that all elements between the end and the returned element will be
    /// consumed, including the returned element. This also means that calling
    /// `nth_back(0)` multiple times on the same iterator will return different
    /// elements.
    ///
    /// `nth_back()` will return [`None`] if `n` is greater than or equal to the length of the
    /// iterator.
    ///
    /// [`None`]: ../../std/option/enum.Option.html#variant.None
    /// [`nth`]: ../../std/iter/trait.Iterator.html#method.nth
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let a = [1, 2, 3];
    /// assert_eq!(a.iter().nth_back(2), Some(&1));
    /// ```
    ///
    /// Calling `nth_back()` multiple times doesn't rewind the iterator:
    ///
    /// ```
    /// let a = [1, 2, 3];
    ///
    /// let mut iter = a.iter();
    ///
    /// assert_eq!(iter.nth_back(1), Some(&2));
    /// assert_eq!(iter.nth_back(1), None);
    /// ```
    ///
    /// Returning `None` if there are less than `n + 1` elements:
    ///
    /// ```
    /// let a = [1, 2, 3];
    /// assert_eq!(a.iter().nth_back(10), None);
    /// ```
    #[inline]
    #[stable(feature = "iter_nth_back", since = "1.37.0")]
    fn nth_back(&mut self, mut n: usize) -> Option<Self::Item> {
        for x in self.rev() {
            if n == 0 { return Some(x) }
            n -= 1;
        }
        None
    }
