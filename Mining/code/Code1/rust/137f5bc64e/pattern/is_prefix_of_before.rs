    fn is_prefix_of(self, haystack: &'a str) -> bool {
        match self.into_searcher(haystack).next() {
            SearchStep::Match(0, _) => true,
            _ => false,
        }
    }

    /// Checks whether the pattern matches at the back of the haystack
    #[inline]
    fn is_suffix_of(self, haystack: &'a str) -> bool
        where Self::Searcher: ReverseSearcher<'a>
    {
        match self.into_searcher(haystack).next_back() {
            SearchStep::Match(_, j) if haystack.len() == j => true,
            _ => false,
        }
    }
}

// Searcher

/// Result of calling `Searcher::next()` or `ReverseSearcher::next_back()`.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum SearchStep {
    /// Expresses that a match of the pattern has been found at
    /// `haystack[a..b]`.
    Match(usize, usize),
    /// Expresses that `haystack[a..b]` has been rejected as a possible match
    /// of the pattern.
    ///
    /// Note that there might be more than one `Reject` between two `Match`es,
    /// there is no requirement for them to be combined into one.
    Reject(usize, usize),
    /// Expresses that every byte of the haystack has been visited, ending
    /// the iteration.
    Done
}

/// A searcher for a string pattern.
///
/// This trait provides methods for searching for non-overlapping
/// matches of a pattern starting from the front (left) of a string.
///
/// It will be implemented by associated `Searcher`
/// types of the `Pattern` trait.
///
/// The trait is marked unsafe because the indices returned by the
/// `next()` methods are required to lie on valid utf8 boundaries in
/// the haystack. This enables consumers of this trait to
/// slice the haystack without additional runtime checks.
pub unsafe trait Searcher<'a> {
    /// Getter for the underlying string to be searched in
    ///
    /// Will always return the same `&str`
    fn haystack(&self) -> &'a str;

    /// Performs the next search step starting from the front.
    ///
    /// - Returns `Match(a, b)` if `haystack[a..b]` matches the pattern.
    /// - Returns `Reject(a, b)` if `haystack[a..b]` can not match the
    ///   pattern, even partially.
    /// - Returns `Done` if every byte of the haystack has been visited
    ///
    /// The stream of `Match` and `Reject` values up to a `Done`
    /// will contain index ranges that are adjacent, non-overlapping,
    /// covering the whole haystack, and laying on utf8 boundaries.
    ///
    /// A `Match` result needs to contain the whole matched pattern,
    /// however `Reject` results may be split up into arbitrary
    /// many adjacent fragments. Both ranges may have zero length.
    ///
    /// As an example, the pattern `"aaa"` and the haystack `"cbaaaaab"`
    /// might produce the stream
    /// `[Reject(0, 1), Reject(1, 2), Match(2, 5), Reject(5, 8)]`
    fn next(&mut self) -> SearchStep;

    /// Find the next `Match` result. See `next()`
    ///
    /// Unlike next(), there is no guarantee that the returned ranges
    /// of this and next_reject will overlap. This will return (start_match, end_match),
    /// where start_match is the index of where the match begins, and end_match is
    /// the index after the end of the match.
    #[inline]
    fn next_match(&mut self) -> Option<(usize, usize)> {
        loop {
            match self.next() {
                SearchStep::Match(a, b) => return Some((a, b)),
                SearchStep::Done => return None,
                _ => continue,
            }
        }
    }

    /// Find the next `Reject` result. See `next()` and `next_match()`
    ///
    /// Unlike next(), there is no guarantee that the returned ranges
    /// of this and next_match will overlap.
    #[inline]
    fn next_reject(&mut self) -> Option<(usize, usize)> {
        loop {
            match self.next() {
                SearchStep::Reject(a, b) => return Some((a, b)),
                SearchStep::Done => return None,
                _ => continue,
            }
        }
    }
}

/// A reverse searcher for a string pattern.
///
/// This trait provides methods for searching for non-overlapping
/// matches of a pattern starting from the back (right) of a string.
///
/// It will be implemented by associated `Searcher`
/// types of the `Pattern` trait if the pattern supports searching
/// for it from the back.
///
/// The index ranges returned by this trait are not required
/// to exactly match those of the forward search in reverse.
///
/// For the reason why this trait is marked unsafe, see them
/// parent trait `Searcher`.
pub unsafe trait ReverseSearcher<'a>: Searcher<'a> {
    /// Performs the next search step starting from the back.
    ///
    /// - Returns `Match(a, b)` if `haystack[a..b]` matches the pattern.
    /// - Returns `Reject(a, b)` if `haystack[a..b]` can not match the
    ///   pattern, even partially.
    /// - Returns `Done` if every byte of the haystack has been visited
    ///
    /// The stream of `Match` and `Reject` values up to a `Done`
    /// will contain index ranges that are adjacent, non-overlapping,
    /// covering the whole haystack, and laying on utf8 boundaries.
    ///
    /// A `Match` result needs to contain the whole matched pattern,
    /// however `Reject` results may be split up into arbitrary
    /// many adjacent fragments. Both ranges may have zero length.
    ///
    /// As an example, the pattern `"aaa"` and the haystack `"cbaaaaab"`
    /// might produce the stream
    /// `[Reject(7, 8), Match(4, 7), Reject(1, 4), Reject(0, 1)]`
    fn next_back(&mut self) -> SearchStep;

    /// Find the next `Match` result. See `next_back()`
    #[inline]
    fn next_match_back(&mut self) -> Option<(usize, usize)>{
        loop {
            match self.next_back() {
                SearchStep::Match(a, b) => return Some((a, b)),
                SearchStep::Done => return None,
                _ => continue,
            }
        }
    }

    /// Find the next `Reject` result. See `next_back()`
    #[inline]
    fn next_reject_back(&mut self) -> Option<(usize, usize)>{
        loop {
            match self.next_back() {
                SearchStep::Reject(a, b) => return Some((a, b)),
                SearchStep::Done => return None,
                _ => continue,
            }
        }
    }
}

/// A marker trait to express that a `ReverseSearcher`
/// can be used for a `DoubleEndedIterator` implementation.
///
/// For this, the impl of `Searcher` and `ReverseSearcher` need
/// to follow these conditions:
///
/// - All results of `next()` need to be identical
///   to the results of `next_back()` in reverse order.
/// - `next()` and `next_back()` need to behave as
///   the two ends of a range of values, that is they
///   can not "walk past each other".
///
/// # Examples
///
/// `char::Searcher` is a `DoubleEndedSearcher` because searching for a
/// `char` only requires looking at one at a time, which behaves the same
/// from both ends.
///
/// `(&str)::Searcher` is not a `DoubleEndedSearcher` because
/// the pattern `"aa"` in the haystack `"aaa"` matches as either
/// `"[aa]a"` or `"a[aa]"`, depending from which side it is searched.
pub trait DoubleEndedSearcher<'a>: ReverseSearcher<'a> {}


/////////////////////////////////////////////////////////////////////////////
// Impl for char
/////////////////////////////////////////////////////////////////////////////

/// Associated type for `<char as Pattern<'a>>::Searcher`.
#[derive(Clone, Debug)]
pub struct CharSearcher<'a> {
    haystack: &'a str,
    // safety invariant: `finger`/`finger_back` must be a valid utf8 byte index of `haystack`
    // This invariant can be broken *within* next_match and next_match_back, however
    // they must exit with fingers on valid code point boundaries.

    /// `finger` is the current byte index of the forward search.
    /// Imagine that it exists before the byte at its index, i.e.
    /// haystack[finger] is the first byte of the slice we must inspect during
    /// forward searching
    finger: usize,
    /// `finger_back` is the current byte index of the reverse search.
    /// Imagine that it exists after the byte at its index, i.e.
    /// haystack[finger_back - 1] is the last byte of the slice we must inspect during
    /// forward searching (and thus the first byte to be inspected when calling next_back())
    finger_back: usize,
    /// The character being searched for
    needle: char,

    // safety invariant: `utf8_size` must be less than 5
    /// The number of bytes `needle` takes up when encoded in utf8
    utf8_size: usize,
    /// A utf8 encoded copy of the `needle`
    utf8_encoded: [u8; 4],
}

unsafe impl<'a> Searcher<'a> for CharSearcher<'a> {
    #[inline]
    fn haystack(&self) -> &'a str {
        self.haystack
    }
    #[inline]
    fn next(&mut self) -> SearchStep {
        let old_finger = self.finger;
        let slice = unsafe { self.haystack.get_unchecked(old_finger..self.finger_back) };
        let mut iter = slice.chars();
        let old_len = iter.iter.len();
        if let Some(ch) = iter.next() {
            // add byte offset of current character
            // without re-encoding as utf-8
            self.finger += old_len - iter.iter.len();
            if ch == self.needle {
                SearchStep::Match(old_finger, self.finger)
            } else {
                SearchStep::Reject(old_finger, self.finger)
            }
        } else {
            SearchStep::Done
        }
    }
    #[inline]
    fn next_match(&mut self) -> Option<(usize, usize)> {
        loop {
            // get the haystack after the last character found
            let bytes = if let Some(slice) = self.haystack.as_bytes()
                                                 .get(self.finger..self.finger_back) {
                slice
            } else {
                return None;
            };
            // the last byte of the utf8 encoded needle
            let last_byte = unsafe { *self.utf8_encoded.get_unchecked(self.utf8_size - 1) };
            if let Some(index) = memchr::memchr(last_byte, bytes) {
                // The new finger is the index of the byte we found,
                // plus one, since we memchr'd for the last byte of the character.
                //
                // Note that this doesn't always give us a finger on a UTF8 boundary.
                // If we *didn't* find our character
                // we may have indexed to the non-last byte of a 3-byte or 4-byte character.
                // We can't just skip to the next valid starting byte because a character like
                // ꁁ (U+A041 YI SYLLABLE PA), utf-8 `EA 81 81` will have us always find
                // the second byte when searching for the third.
                //
                // However, this is totally okay. While we have the invariant that
                // self.finger is on a UTF8 boundary, this invariant is not relid upon
                // within this method (it is relied upon in CharSearcher::next()).
                //
                // We only exit this method when we reach the end of the string, or if we
                // find something. When we find something the `finger` will be set
                // to a UTF8 boundary.
                self.finger += index + 1;
                if self.finger >= self.utf8_size {
                    let found_char = self.finger - self.utf8_size;
                    if let Some(slice) = self.haystack.as_bytes().get(found_char..self.finger) {
                        if slice == &self.utf8_encoded[0..self.utf8_size] {
                            return Some((found_char, self.finger));
                        }
                    }
                }
            } else {
                // found nothing, exit
                self.finger = self.finger_back;
                return None;
            }
        }
    }

    // let next_reject use the default implementation from the Searcher trait
}
