//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use super::Boundary;
use std::ops::Range;

#[derive(Debug, Clone, Copy)]
pub struct Word<T> {
    /// The width of the word.
    pub width: f64,

    /// The spacing between this and the following word.
    pub spacing: f64,

    /// The minimum line height required to fit the word in a line.
    pub line_height: f64,

    /// A custom value returned by a word-wrapping function.
    pub value: T,
}

#[derive(Debug, Clone)]
pub struct Line<T> {
    pub start_value: T,
    pub x_coord_range: Range<f64>,
    pub y_coord: f64,
}

trait BoundaryExt: Boundary {
    fn line_range_by_height(
        &self,
        line_start: f64,
        line_height: f64,
        line: usize,
    ) -> Option<Range<f64>> {
        self.line_range(line_start..line_start + line_height, line)
    }

    fn line_width(&self, line_start: f64, line_height: f64, line: usize) -> Option<f64> {
        self.line_range_by_height(line_start, line_height, line)
            .map(|r| r.end - r.start)
    }
}

impl<T: Boundary + ?Sized> BoundaryExt for T {}

pub fn word_wrap_greedy<'a, I, T: 'a, B>(
    words: &'a mut I,
    boundary: &'a B,
    initial_y: f64,
) -> impl IntoIterator<Item = Line<T>> + 'a
where
    I: Iterator<Item = Word<T>>,
    B: Boundary,
{
    use itertools::unfold;
    use std::f64::NEG_INFINITY;

    struct State<I, T> {
        it: I,
        start: T,
        width: f64,
        hanging_width: f64,
        line: usize,
        line_y: f64,
        line_height: f64,
    }

    let first = words.next();

    let state = first.map(move |x| State {
        it: words,
        start: x.value,
        width: x.width,
        hanging_width: x.spacing,
        line: 0,
        line_y: initial_y,
        line_height: x.line_height,
    });

    unfold(state, move |state| {
        if let Some(mut s) = state.take() {
            let next;

            let boundary_width = boundary.line_width(s.line_y, s.line_height, s.line);

            if s.width < boundary_width.unwrap_or(NEG_INFINITY) {
                // Add more words to the current line
                loop {
                    let word = s.it.next();

                    if word.is_none() {
                        next = None;
                        break;
                    }

                    let word = word.unwrap();

                    let new_line_height = s.line_height.max(word.line_height);
                    let new_width = s.width + s.hanging_width + word.width;
                    let new_boundary_width = boundary.line_width(s.line_y, new_line_height, s.line);
                    if new_width > new_boundary_width.unwrap_or(NEG_INFINITY) {
                        // Overflow
                        if new_boundary_width.is_none() {
                            // Can't have more words
                            // TODO: Tell where we left off.
                            //       The layout engine would panic if we stop
                            //       iteration in the middle
                            next = None;
                        } else {
                            next = Some(word);
                        }
                        break;
                    }

                    s.width = new_width;
                    s.hanging_width = word.spacing;
                    s.line_height = new_line_height;
                }
            } else {
                if boundary_width.is_none() {
                    // Can't output even a single line
                    return None;
                } else {
                    next = s.it.next();
                }
            }

            let ret_line = Line {
                start_value: s.start,
                x_coord_range: boundary
                    .line_range_by_height(s.line_y, s.line_height, s.line)
                    .unwrap(),
                y_coord: s.line_y + s.line_height,
            };

            // Next line
            if let Some(word) = next {
                *state = Some(State {
                    it: s.it,
                    start: word.value,
                    width: word.width,
                    hanging_width: word.spacing,
                    line: s.line + 1,
                    line_y: s.line_y + s.line_height,
                    line_height: word.line_height,
                });
            }

            Some(ret_line)
        } else {
            None
        }
    })
}
