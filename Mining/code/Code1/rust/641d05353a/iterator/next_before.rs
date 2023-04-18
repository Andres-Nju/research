    fn next(&mut self) -> Option<Self::Item>;

    /// Returns the bounds on the remaining length of the iterator.
    ///
    /// Specifically, `size_hint()` returns a tuple where the first element
    /// is the lower bound, and the second element is the upper bound.
    ///
    /// The second half of the tuple that is returned is an [`Option`]`<`[`usize`]`>`.
    /// A [`None`] here means that either there is no known upper bound, or the
    /// upper bound is larger than [`usize`].
    ///
    /// # Implementation notes
    ///
    /// It is not enforced that an iterator implementation yields the declared
    /// number of elements. A buggy iterator may yield less than the lower bound
    /// or more than the upper bound of elements.
    ///
    /// `size_hint()` is primarily intended to be used for optimizations such as
    /// reserving space for the elements of the iterator, but must not be
    /// trusted to e.g. omit bounds checks in unsafe code. An incorrect
    /// implementation of `size_hint()` should not lead to memory safety
    /// violations.
    ///
    /// That said, the implementation should provide a correct estimation,
    /// because otherwise it would be a violation of the trait's protocol.
    ///
    /// The default implementation returns `(0, None)` which is correct for any
    /// iterator.
    ///
    /// [`usize`]: ../../std/primitive.usize.html
    /// [`Option`]: ../../std/option/enum.Option.html
    /// [`None`]: ../../std/option/enum.Option.html#variant.None
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let a = [1, 2, 3];
    /// let iter = a.iter();
    ///
    /// assert_eq!((3, Some(3)), iter.size_hint());
    /// ```
    ///
    /// A more complex example:
    ///
    /// ```
    /// // The even numbers from zero to ten.
    /// let iter = (0..10).filter(|x| x % 2 == 0);
    ///
    /// // We might iterate from zero to ten times. Knowing that it's five
    /// // exactly wouldn't be possible without executing filter().
    /// assert_eq!((0, Some(10)), iter.size_hint());
    ///
    /// // Let's add one five more numbers with chain()
    /// let iter = (0..10).filter(|x| x % 2 == 0).chain(15..20);
    ///
    /// // now both bounds are increased by five
    /// assert_eq!((5, Some(15)), iter.size_hint());
    /// ```
    ///
    /// Returning `None` for an upper bound:
    ///
    /// ```
    /// // an infinite iterator has no upper bound
    /// let iter = 0..;
    ///
    /// assert_eq!((0, None), iter.size_hint());
    /// ```
    #[inline]
    #[stable(feature = "rust1", since = "1.0.0")]
    fn size_hint(&self) -> (usize, Option<usize>) { (0, None) }
