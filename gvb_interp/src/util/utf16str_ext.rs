use super::ascii_ext::AsciiExt;
use std::borrow::ToOwned;
use widestring::{Utf16Str, Utf16String};

pub trait Utf16StrExt: ToOwned {
  fn find_char(&self, c: char) -> Option<usize>;
  fn replace_char<S>(&self, c: char, repl: S) -> Self::Owned
  where
    S: AsRef<Self>;
  fn eq_ignore_ascii_case(&self, other: &Self) -> bool;
  fn to_ascii_uppercase(&self) -> Self::Owned;
  fn to_ascii_lowercase(&self) -> Self::Owned;
  fn make_ascii_uppercase(&mut self);
  fn make_ascii_lowercase(&mut self);
  fn ends_with_char(&self, c: char) -> bool;
  /// Returns if the string is composed of spaces only.
  fn is_blank(&self) -> bool;
  fn contains_char(&self, c: char) -> bool;
  fn count_char(&self, c: char) -> usize;
  fn first_line(&self) -> &Self;
  fn rfind_str(&self, other: &Self) -> Option<usize>;
}

impl Utf16StrExt for Utf16Str {
  fn find_char(&self, c: char) -> Option<usize> {
    for (i, c1) in self.char_indices() {
      if c1 == c {
        return Some(i);
      }
    }
    None
  }

  fn contains_char(&self, c: char) -> bool {
    self.chars().any(|x| x == c)
  }

  fn count_char(&self, c: char) -> usize {
    self.chars().filter(|&x| x == c).count()
  }

  fn replace_char<S>(&self, c: char, repl: S) -> Self::Owned
  where
    S: AsRef<Self>,
  {
    let mut result = Utf16String::new();
    let mut last_end = 0;
    let c_len = c.len_utf16();
    let repl = repl.as_ref();
    for i in utf16str_match_char_indices(self, c) {
      result.push_utfstr(unsafe { self.get_unchecked(last_end..i) });
      result.push_utfstr(repl);
      last_end = i + c_len;
    }
    result.push_utfstr(unsafe { self.get_unchecked(last_end..self.len()) });
    result
  }

  fn eq_ignore_ascii_case(&self, other: &Self) -> bool {
    if self.len() != other.len() {
      return false;
    }

    for (&a, &b) in std::iter::zip(self.as_slice(), other.as_slice()) {
      if a.to_ascii_lowercase() != b.to_ascii_lowercase() {
        return false;
      }
    }

    true
  }

  fn to_ascii_uppercase(&self) -> Self::Owned {
    let mut s = Utf16String::with_capacity(self.len());
    for c in self.chars() {
      s.push(c.to_ascii_uppercase());
    }
    s
  }

  fn to_ascii_lowercase(&self) -> Self::Owned {
    let mut s = Utf16String::with_capacity(self.len());
    for c in self.chars() {
      s.push(c.to_ascii_lowercase());
    }
    s
  }

  fn make_ascii_lowercase(&mut self) {
    for c in unsafe { self.as_mut_slice() } {
      *c = c.to_ascii_lowercase();
    }
  }

  fn make_ascii_uppercase(&mut self) {
    for c in unsafe { self.as_mut_slice() } {
      *c = c.to_ascii_uppercase();
    }
  }

  fn ends_with_char(&self, c: char) -> bool {
    let len = self.len();
    if c.len_utf16() == 2 {
      len > 1 && {
        let mut enc = [0; 0];
        c.encode_utf16(&mut enc);
        self.as_slice()[len - 2] == enc[0] && self.as_slice()[len - 1] == enc[1]
      }
    } else {
      len > 0 && self.as_slice()[len - 1] == c as u32 as u16
    }
  }

  fn is_blank(&self) -> bool {
    self.as_slice().iter().all(|&c| c == b' ' as u16)
  }

  fn first_line(&self) -> &Self {
    if self.is_empty() {
      return self;
    }

    let i = self.find_char('\n').unwrap_or(self.len());
    if i > 0 && self.as_slice()[i - 1] == b'\r' as u16 {
      &self[..i - 1]
    } else {
      &self[..i]
    }
  }

  fn rfind_str(&self, needle: &Self) -> Option<usize> {
    if needle.is_empty() {
      return Some(self.len());
    }
    let mut searcher = UtfTwoWaySearcher::new(self.as_slice(), self.len());
    let is_long = searcher.memory == usize::MAX;
    // write out `true` and `false`, like `next_match`
    if is_long {
      searcher.next_back::<MatchOnly>(
        self.as_slice(),
        needle.as_slice(),
        true,
      ).map(|x| x.0)
    } else {
      searcher.next_back::<MatchOnly>(
        self.as_slice(),
        needle.as_slice(),
        false,
      ).map(|x| x.0)
    }
  }
}

struct CharIdxIter<'a> {
  s: &'a Utf16Str,
  c: char,
  offset: usize,
}

impl<'a> Iterator for CharIdxIter<'a> {
  type Item = usize;

  fn next(&mut self) -> Option<Self::Item> {
    match utf16str_find_char_from(self.s, self.c, self.offset) {
      Some(i) => {
        self.offset = i + 1;
        Some(i)
      }
      None => None,
    }
  }
}

fn utf16str_match_char_indices<'a>(
  s: &'a Utf16Str,
  c: char,
) -> impl Iterator<Item = usize> + 'a {
  CharIdxIter { s, c, offset: 0 }
}

fn utf16str_find_char_from(
  s: &Utf16Str,
  c: char,
  begin: usize,
) -> Option<usize> {
  for (i, c1) in s[begin..].char_indices() {
    if c1 == c {
      return Some(i + begin);
    }
  }
  None
}

macro_rules! match_u16c {
  ($exp:expr, $c:literal) => {{
    const C: Option<u16> = Some(($c as _));
    matches!($exp.copied(), C)
  }};
  ($exp:expr, $c1:literal | $c2:literal) => {{
    const C1: Option<u16> = Some(($c1 as _));
    const C2: Option<u16> = Some(($c2 as _));
    matches!($exp.copied(), C1 | C2)
  }};
  ($exp:expr, $c1:literal | $c2:literal | None) => {{
    const C1: Option<u16> = Some(($c1 as _));
    const C2: Option<u16> = Some(($c2 as _));
    matches!($exp.copied(), C1 | C2 | None)
  }};
}

/// The internal state of the two-way substring search algorithm.
#[derive(Clone, Debug)]
struct UtfTwoWaySearcher {
  // constants
  /// critical factorization index
  crit_pos: usize,
  /// critical factorization index for reversed needle
  crit_pos_back: usize,
  period: usize,
  /// `byteset` is an extension (not part of the two way algorithm);
  /// it's a 64-bit "fingerprint" where each set bit `j` corresponds
  /// to a (byte & 63) == j present in the needle.
  byteset: u64,

  // variables
  position: usize,
  end: usize,
  /// index into needle before which we have already matched
  memory: usize,
  /// index into needle after which we have already matched
  memory_back: usize,
}

impl UtfTwoWaySearcher {
  fn new(needle: &[u16], end: usize) -> Self {
    let (crit_pos_false, period_false) =
      Self::maximal_suffix(needle, false);
    let (crit_pos_true, period_true) =
      Self::maximal_suffix(needle, true);

    let (crit_pos, period) = if crit_pos_false > crit_pos_true {
      (crit_pos_false, period_false)
    } else {
      (crit_pos_true, period_true)
    };

    // A particularly readable explanation of what's going on here can be found
    // in Crochemore and Rytter's book "Text Algorithms", ch 13. Specifically
    // see the code for "Algorithm CP" on p. 323.
    //
    // What's going on is we have some critical factorization (u, v) of the
    // needle, and we want to determine whether u is a suffix of
    // &v[..period]. If it is, we use "Algorithm CP1". Otherwise we use
    // "Algorithm CP2", which is optimized for when the period of the needle
    // is large.
    if needle[..crit_pos] == needle[period..period + crit_pos] {
      // short period case -- the period is exact
      // compute a separate critical factorization for the reversed needle
      // x = u' v' where |v'| < period(x).
      //
      // This is sped up by the period being known already.
      // Note that a case like x = "acba" may be factored exactly forwards
      // (crit_pos = 1, period = 3) while being factored with approximate
      // period in reverse (crit_pos = 2, period = 2). We use the given
      // reverse factorization but keep the exact period.
      let crit_pos_back = needle.len()
        - std::cmp::max(
          Self::reverse_maximal_suffix(needle, period, false),
          Self::reverse_maximal_suffix(needle, period, true),
        );

      Self {
        crit_pos,
        crit_pos_back,
        period,
        byteset: Self::byteset_create(&needle[..period]),

        position: 0,
        end,
        memory: 0,
        memory_back: needle.len(),
      }
    } else {
      // long period case -- we have an approximation to the actual period,
      // and don't use memorization.
      //
      // Approximate the period by lower bound max(|u|, |v|) + 1.
      // The critical factorization is efficient to use for both forward and
      // reverse search.

      Self {
        crit_pos,
        crit_pos_back: crit_pos,
        period: std::cmp::max(crit_pos, needle.len() - crit_pos) + 1,
        byteset: Self::byteset_create(needle),

        position: 0,
        end,
        memory: usize::MAX, // Dummy value to signify that the period is long
        memory_back: usize::MAX,
      }
    }
  }

  #[inline]
  fn byteset_create(bytes: &[u16]) -> u64 {
    bytes.iter().fold(0, |a, &b| (1 << (b & 0x3f)) | a)
  }

  #[inline]
  fn byteset_contains(&self, byte: u16) -> bool {
    (self.byteset >> ((byte & 0x3f) as usize)) & 1 != 0
  }

  // One of the main ideas of Two-Way is that we factorize the needle into
  // two halves, (u, v), and begin trying to find v in the haystack by scanning
  // left to right. If v matches, we try to match u by scanning right to left.
  // How far we can jump when we encounter a mismatch is all based on the fact
  // that (u, v) is a critical factorization for the needle.
  #[inline]
  fn next<S>(
    &mut self,
    haystack: &[u16],
    needle: &[u16],
    long_period: bool,
  ) -> S::Output
  where
    S: UtfTwoWayStrategy,
  {
    // `next()` uses `self.position` as its cursor
    let old_pos = self.position;
    let needle_last = needle.len() - 1;
    'search: loop {
      // Check that we have room to search in
      // position + needle_last can not overflow if we assume slices
      // are bounded by isize's range.
      let tail_byte = match haystack.get(self.position + needle_last) {
        Some(&b) => b,
        None => {
          self.position = haystack.len();
          return S::rejecting(old_pos, self.position);
        }
      };

      if S::use_early_reject() && old_pos != self.position {
        return S::rejecting(old_pos, self.position);
      }

      // Quickly skip by large portions unrelated to our substring
      if !self.byteset_contains(tail_byte) {
        self.position += needle.len();
        if !long_period {
          self.memory = 0;
        }
        continue 'search;
      }

      // See if the right part of the needle matches
      let start = if long_period {
        self.crit_pos
      } else {
        std::cmp::max(self.crit_pos, self.memory)
      };
      for i in start..needle.len() {
        if needle[i] != haystack[self.position + i] {
          self.position += i - self.crit_pos + 1;
          if !long_period {
            self.memory = 0;
          }
          continue 'search;
        }
      }

      // See if the left part of the needle matches
      let start = if long_period { 0 } else { self.memory };
      for i in (start..self.crit_pos).rev() {
        if needle[i] != haystack[self.position + i] {
          self.position += self.period;
          if !long_period {
            self.memory = needle.len() - self.period;
          }
          continue 'search;
        }
      }

      // We have found a match!
      let match_pos = self.position;

      // Note: add self.period instead of needle.len() to have overlapping matches
      self.position += needle.len();
      if !long_period {
        self.memory = 0; // set to needle.len() - self.period for overlapping matches
      }

      return S::matching(match_pos, match_pos + needle.len());
    }
  }

  // Follows the ideas in `next()`.
  //
  // The definitions are symmetrical, with period(x) = period(reverse(x))
  // and local_period(u, v) = local_period(reverse(v), reverse(u)), so if (u, v)
  // is a critical factorization, so is (reverse(v), reverse(u)).
  //
  // For the reverse case we have computed a critical factorization x = u' v'
  // (field `crit_pos_back`). We need |u| < period(x) for the forward case and
  // thus |v'| < period(x) for the reverse.
  //
  // To search in reverse through the haystack, we search forward through
  // a reversed haystack with a reversed needle, matching first u' and then v'.
  #[inline]
  fn next_back<S>(
    &mut self,
    haystack: &[u16],
    needle: &[u16],
    long_period: bool,
  ) -> S::Output
  where
    S: UtfTwoWayStrategy,
  {
    // `next_back()` uses `self.end` as its cursor -- so that `next()` and `next_back()`
    // are independent.
    let old_end = self.end;
    'search: loop {
      // Check that we have room to search in
      // end - needle.len() will wrap around when there is no more room,
      // but due to slice length limits it can never wrap all the way back
      // into the length of haystack.
      let front_byte = match haystack.get(self.end.wrapping_sub(needle.len())) {
        Some(&b) => b,
        None => {
          self.end = 0;
          return S::rejecting(0, old_end);
        }
      };

      if S::use_early_reject() && old_end != self.end {
        return S::rejecting(self.end, old_end);
      }

      // Quickly skip by large portions unrelated to our substring
      if !self.byteset_contains(front_byte) {
        self.end -= needle.len();
        if !long_period {
          self.memory_back = needle.len();
        }
        continue 'search;
      }

      // See if the left part of the needle matches
      let crit = if long_period {
        self.crit_pos_back
      } else {
        std::cmp::min(self.crit_pos_back, self.memory_back)
      };
      for i in (0..crit).rev() {
        if needle[i] != haystack[self.end - needle.len() + i] {
          self.end -= self.crit_pos_back - i;
          if !long_period {
            self.memory_back = needle.len();
          }
          continue 'search;
        }
      }

      // See if the right part of the needle matches
      let needle_end = if long_period {
        needle.len()
      } else {
        self.memory_back
      };
      for i in self.crit_pos_back..needle_end {
        if needle[i] != haystack[self.end - needle.len() + i] {
          self.end -= self.period;
          if !long_period {
            self.memory_back = self.period;
          }
          continue 'search;
        }
      }

      // We have found a match!
      let match_pos = self.end - needle.len();
      // Note: sub self.period instead of needle.len() to have overlapping matches
      self.end -= needle.len();
      if !long_period {
        self.memory_back = needle.len();
      }

      return S::matching(match_pos, match_pos + needle.len());
    }
  }

  // Compute the maximal suffix of `arr`.
  //
  // The maximal suffix is a possible critical factorization (u, v) of `arr`.
  //
  // Returns (`i`, `p`) where `i` is the starting index of v and `p` is the
  // period of v.
  //
  // `order_greater` determines if lexical order is `<` or `>`. Both
  // orders must be computed -- the ordering with the largest `i` gives
  // a critical factorization.
  //
  // For long period cases, the resulting period is not exact (it is too short).
  #[inline]
  fn maximal_suffix(arr: &[u16], order_greater: bool) -> (usize, usize) {
    let mut left = 0; // Corresponds to i in the paper
    let mut right = 1; // Corresponds to j in the paper
    let mut offset = 0; // Corresponds to k in the paper, but starting at 0
                        // to match 0-based indexing.
    let mut period = 1; // Corresponds to p in the paper

    while let Some(&a) = arr.get(right + offset) {
      // `left` will be inbounds when `right` is.
      let b = arr[left + offset];
      if (a < b && !order_greater) || (a > b && order_greater) {
        // Suffix is smaller, period is entire prefix so far.
        right += offset + 1;
        offset = 0;
        period = right - left;
      } else if a == b {
        // Advance through repetition of the current period.
        if offset + 1 == period {
          right += offset + 1;
          offset = 0;
        } else {
          offset += 1;
        }
      } else {
        // Suffix is larger, start over from current location.
        left = right;
        right += 1;
        offset = 0;
        period = 1;
      }
    }
    (left, period)
  }

  // Compute the maximal suffix of the reverse of `arr`.
  //
  // The maximal suffix is a possible critical factorization (u', v') of `arr`.
  //
  // Returns `i` where `i` is the starting index of v', from the back;
  // returns immediately when a period of `known_period` is reached.
  //
  // `order_greater` determines if lexical order is `<` or `>`. Both
  // orders must be computed -- the ordering with the largest `i` gives
  // a critical factorization.
  //
  // For long period cases, the resulting period is not exact (it is too short).
  fn reverse_maximal_suffix(
    arr: &[u16],
    known_period: usize,
    order_greater: bool,
  ) -> usize {
    let mut left = 0; // Corresponds to i in the paper
    let mut right = 1; // Corresponds to j in the paper
    let mut offset = 0; // Corresponds to k in the paper, but starting at 0
                        // to match 0-based indexing.
    let mut period = 1; // Corresponds to p in the paper
    let n = arr.len();

    while right + offset < n {
      let a = arr[n - (1 + right + offset)];
      let b = arr[n - (1 + left + offset)];
      if (a < b && !order_greater) || (a > b && order_greater) {
        // Suffix is smaller, period is entire prefix so far.
        right += offset + 1;
        offset = 0;
        period = right - left;
      } else if a == b {
        // Advance through repetition of the current period.
        if offset + 1 == period {
          right += offset + 1;
          offset = 0;
        } else {
          offset += 1;
        }
      } else {
        // Suffix is larger, start over from current location.
        left = right;
        right += 1;
        offset = 0;
        period = 1;
      }
      if period == known_period {
        break;
      }
    }
    debug_assert!(period <= known_period);
    left
  }
}

// TwoWayStrategy allows the algorithm to either skip non-matches as quickly
// as possible, or to work in a mode where it emits Rejects relatively quickly.
trait UtfTwoWayStrategy {
  type Output;
  fn use_early_reject() -> bool;
  fn rejecting(a: usize, b: usize) -> Self::Output;
  fn matching(a: usize, b: usize) -> Self::Output;
}

/// Skip to match intervals as quickly as possible
enum MatchOnly {}

impl UtfTwoWayStrategy for MatchOnly {
  type Output = Option<(usize, usize)>;

  #[inline]
  fn use_early_reject() -> bool {
    false
  }
  #[inline]
  fn rejecting(_a: usize, _b: usize) -> Self::Output {
    None
  }
  #[inline]
  fn matching(a: usize, b: usize) -> Self::Output {
    Some((a, b))
  }
}

/// Emit Rejects regularly
enum RejectAndMatch {}

impl UtfTwoWayStrategy for RejectAndMatch {
  type Output = UtfSearchStep;

  #[inline]
  fn use_early_reject() -> bool {
    true
  }
  #[inline]
  fn rejecting(a: usize, b: usize) -> Self::Output {
    UtfSearchStep::Reject(a, b)
  }
  #[inline]
  fn matching(a: usize, b: usize) -> Self::Output {
    UtfSearchStep::Match(a, b)
  }
}

/// Result of calling [`Searcher::next()`] or [`ReverseSearcher::next_back()`].
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum UtfSearchStep {
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
    Done,
}