//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::{iter, ops};

/// An unattributed editable text fragment.
pub trait Span: Clone {
    /// Get the length of the span.
    ///
    /// The definition of the length varies from one implementation to another.
    /// For `String`, it's defined to be a number of bytes.
    fn len(&self) -> usize;

    fn append(&mut self, o: &Self);
}

impl Span for String {
    fn len(&self) -> usize {
        self.as_str().len()
    }

    fn append(&mut self, o: &Self) {
        *self += o;
    }
}

/// Provides a reference to a subrange of a span (text fragment).
pub trait Subspan: Span {
    type Output: ?Sized;

    /// Get a reference to a subrange of the span.
    fn subspan(&self, range: ops::Range<usize>) -> &Self::Output;
}

impl Subspan for String {
    type Output = str;

    fn subspan(&self, range: ops::Range<usize>) -> &Self::Output {
        &self[range]
    }
}

/// A character position in `Text`. Gets invalidated whenever the originating
/// `Text` is modified.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct Cursor {
    /// Index into `Text::spans`, in range `[0, spans.len()]`.
    span_index: usize,

    /// A character position in `Span`, in range `[0, Span::len() - 1]`. Must be
    /// `0` if `span_index == spans.len()`.
    char_index: usize,
}

impl Cursor {
    pub fn begin() -> Self {
        Self {
            span_index: 0,
            char_index: 0,
        }
    }
}

/// Text with character attributes. Conceptually equivalent to `Vec<(S, A)>`.
///
/// # Type parameters
///
///  - `S`: Unattributed textual contents, each instance given an uniform set of
///    atributes. `String` would be a good candidate for this parameter.
///  - `A`: Attributes.
///
#[derive(Debug, PartialEq, Eq, Clone, Hash, Default)]
pub struct Text<S, A> {
    spans: Vec<(S, A)>,
    len: usize,
}

impl<S, A> Text<S, A> {
    pub fn new() -> Self {
        Self {
            spans: Vec::new(),
            len: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            spans: Vec::with_capacity(capacity),
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    /// Construct a `Cursor` pointing the beginning of the text. Identical to
    /// `Cursor::begin()`.
    pub fn begin(&self) -> Cursor {
        Cursor::begin()
    }

    /// Construct a `Cursor` pointing the end of the text.
    pub fn end(&self) -> Cursor {
        Cursor {
            span_index: self.spans.len(),
            char_index: 0,
        }
    }
}

impl<S: Span, A> Text<S, A> {
    /// Append a span of attributed text contents.
    pub fn push(&mut self, text: S, attr: A) {
        let len = text.len();
        self.spans.push((text, attr));
        self.len += len;
    }

    /// Iterate through `Run`s in this `Text`.
    pub fn runs<'a>(&'a self) -> impl Iterator<Item = Run<&'a S, &'a A>> + 'a {
        use itertools::unfold;
        unfold(
            (0, self.spans.iter().enumerate()),
            |&mut (ref mut position, ref mut iter)| {
                iter.next().map(|(span_index, &(ref span, ref attribute))| {
                    let len = span.len();
                    let pos_range = *position..*position + len;
                    *position += span.len();
                    Run {
                        span,
                        attribute,
                        cursor: Cursor {
                            span_index,
                            char_index: 0,
                        }..Cursor {
                            span_index: span_index + 1,
                            char_index: 0,
                        },
                        position: pos_range,
                    }
                })
            },
        )
    }

    /// Construct a `Cursor` by offseting a given one. Returns `None` if the
    /// result is out of bounds.
    pub fn offset(&self, mut cursor: Cursor, offs: isize) -> Option<Cursor> {
        let ref spans = self.spans[..];
        assert!(cursor.span_index <= spans.len());
        if offs < 0 {
            let mut offs = (-offs) as usize;
            loop {
                if offs > cursor.char_index {
                    offs -= cursor.char_index;
                    if let Some(x) = cursor.span_index.checked_sub(1) {
                        cursor.span_index = x;
                        cursor.char_index = spans[x].0.len();
                    } else {
                        return None;
                    }
                } else {
                    cursor.char_index -= offs;
                    break;
                }
            }
        } else if offs > 0 {
            let mut offs = offs as usize;
            if cursor.span_index >= spans.len() {
                return None;
            }
            loop {
                let remaining = spans[cursor.span_index].0.len() - cursor.char_index;
                if offs >= remaining {
                    offs -= remaining;
                    cursor.span_index += 1;
                    cursor.char_index = 0;
                    if cursor.span_index >= spans.len() {
                        if offs > 0 {
                            return None;
                        } else {
                            break;
                        }
                    }
                } else {
                    cursor.char_index += offs;
                    break;
                }
            }
        }
        Some(cursor)
    }

    /// Compute the difference between given two `Cursor`s.
    pub fn sub(&self, cursor1: Cursor, cursor2: Cursor) -> isize {
        let inner = |mut cursor: Cursor, reference: Cursor| {
            debug_assert!(cursor <= reference);

            let mut distance = 0;
            let ref spans = self.spans[..];

            assert!(reference.span_index <= spans.len());

            while cursor.span_index < reference.span_index {
                distance += spans[cursor.span_index].0.len() - cursor.char_index;
                cursor.span_index += 1;
                cursor.char_index = 0;
            }

            distance + (reference.char_index - cursor.char_index)
        };

        if cursor1 <= cursor2 {
            inner(cursor1, cursor2) as isize
        } else {
            -(inner(cursor2, cursor1) as isize)
        }
    }
}

impl<S: Subspan, A> Text<S, A> {
    /// Iterate through `Run`s in the specified range.
    ///
    /// Note that the returned `Run`'s `Run::position()` is relative to the
    /// start of the range, not of entire the text.
    pub fn runs_in_range<'a>(
        &'a self,
        range: ops::Range<Cursor>,
    ) -> impl Iterator<Item = Run<&'a S::Output, &'a A>> + 'a {
        use itertools::unfold;
        unfold(
            (0, range.start),
            move |&mut (ref mut cur_position, ref mut cursor)| {
                if *cursor != range.end {
                    let (ref span, ref attribute) = self.spans[cursor.span_index];

                    let run_start = *cursor;
                    let run_end;

                    let span_start = run_start.char_index;
                    let span_end;

                    if cursor.span_index == range.end.span_index {
                        run_end = range.end;
                        span_end = run_end.char_index;
                    } else {
                        run_end = Cursor {
                            span_index: run_start.span_index + 1,
                            char_index: 0,
                        };
                        span_end = span.len();
                    }

                    let position = *cur_position..*cur_position + (span_end - span_start);

                    *cur_position += span_end - span_start;
                    *cursor = run_end;

                    Some(Run {
                        span: span.subspan(span_start..span_end),
                        attribute,
                        cursor: run_start..run_end,
                        position,
                    })
                } else {
                    None
                }
            },
        )
    }
}

impl<S: Span, A> From<(S, A)> for Text<S, A> {
    fn from(x: (S, A)) -> Self {
        let len = x.0.len();
        Self {
            spans: vec![x],
            len,
        }
    }
}

impl<S: Span, A> iter::FromIterator<(S, A)> for Text<S, A> {
    fn from_iter<T: iter::IntoIterator<Item = (S, A)>>(iter: T) -> Self {
        let mut len = 0;
        let spans: Vec<_> = iter.into_iter()
            .inspect(|x| {
                len += x.0.len();
            })
            .collect();
        Self { spans, len }
    }
}

impl<S, A> iter::IntoIterator for Text<S, A> {
    type Item = (S, A);
    type IntoIter = <Vec<(S, A)> as iter::IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.spans.into_iter()
    }
}

impl<S: Span, A: Default> Text<S, A> {
    /// Append a span of unattributed text contents using the lastly occuring
    /// attributes. If `self` is empty, the newly inserted contents are given
    /// the default attributes.
    pub fn push_text(&mut self, text: S) {
        let len = text.len();

        if let Some(x) = self.spans.last_mut() {
            x.0.append(&text);
            self.len += len;
            return;
        }

        self.spans.push((text, Default::default()));
        self.len += len;
    }
}

impl<S: Span, A: Default> From<S> for Text<S, A> {
    fn from(x: S) -> Self {
        (x, A::default()).into()
    }
}

/// A span of text with an identical set of attributes, associated with a range
/// in a `Text`.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Run<S, A> {
    span: S,
    attribute: A,
    position: ops::Range<usize>,
    cursor: ops::Range<Cursor>,
}

impl<S, A> Run<S, A> {
    pub fn span(&self) -> &S {
        &self.span
    }

    pub fn attribute(&self) -> &A {
        &self.attribute
    }

    pub fn position(&self) -> &ops::Range<usize> {
        &self.position
    }

    pub fn cursor(&self) -> &ops::Range<Cursor> {
        &self.cursor
    }

    pub fn len(&self) -> usize {
        self.position().len()
    }
}
