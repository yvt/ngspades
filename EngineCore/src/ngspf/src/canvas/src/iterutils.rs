//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::ops::Range;

pub struct WithRange<T, F, I: Iterator> {
    getter: F,
    iter: I,
    state: WithRangeState<I::Item, T>,
}

enum WithRangeState<V, E> {
    Mid {
        last_item: V,
        last_endpoint: E,
        end: E,
    },
    End,
}

impl<T: Clone, F, I: Iterator> Iterator for WithRange<T, F, I>
where
    F: FnMut(&I::Item) -> T,
{
    type Item = (Range<T>, I::Item);

    fn size_hint(&self) -> (usize, Option<usize>) {
        if let WithRangeState::End = self.state {
            return (0, Some(0));
        }
        let (lower, upper) = self.iter.size_hint();
        (lower + 1, upper.and_then(|x| x.checked_add(1)))
    }

    fn next(&mut self) -> Option<Self::Item> {
        use std::mem::replace;

        match replace(&mut self.state, WithRangeState::End) {
            WithRangeState::Mid {
                last_item,
                last_endpoint,
                end,
            } => {
                if let Some(cur_item) = self.iter.next() {
                    let cur_endpoint = (self.getter)(&cur_item);
                    self.state = WithRangeState::Mid {
                        last_item: cur_item,
                        last_endpoint: cur_endpoint.clone(),
                        end,
                    };
                    Some((last_endpoint..cur_endpoint, last_item))
                } else {
                    Some((last_endpoint..end, last_item))
                }
            }
            WithRangeState::End => None,
        }
    }
}

impl<T: Clone, F, I: ExactSizeIterator> ExactSizeIterator for WithRange<T, F, I>
where
    F: FnMut(&I::Item) -> T,
{
    fn len(&self) -> usize {
        if let WithRangeState::End = self.state {
            return 0;
        }
        self.iter.len() + 1
    }
}

pub trait IterUtils: Iterator + Sized {
    /// Associate each element with a range, using values derived from adjacent
    /// elements as its endpoints.
    ///
    /// The result is a series of `(Range<T>, Self::Item)`. The lower bound of
    /// the range is derived from its associated element, and the upper bound
    /// comes from the next element.
    fn with_range<T: Clone, F>(mut self, end: T, mut getter: F) -> WithRange<T, F, Self>
    where
        F: FnMut(&Self::Item) -> T,
    {
        if let Some(first_item) = self.next() {
            let first_endpoint = getter(&first_item);
            WithRange {
                getter,
                iter: self,
                state: WithRangeState::Mid {
                    last_item: first_item,
                    last_endpoint: first_endpoint,
                    end,
                },
            }
        } else {
            WithRange {
                getter,
                iter: self,
                state: WithRangeState::End,
            }
        }
    }
}

impl<T> IterUtils for T
where
    T: Iterator,
{
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_range_0() {
        let nums: [&'static str; 0] = [];
        let ranges: Vec<_> = nums
            .iter()
            .cloned()
            .with_range(15, |x| x.parse::<u32>().unwrap())
            .collect();
        assert!(ranges.is_empty());
    }

    #[test]
    fn with_range_1() {
        let nums = ["1"];
        let ranges: Vec<_> = nums
            .iter()
            .cloned()
            .with_range(15, |x| x.parse::<u32>().unwrap())
            .collect();
        assert_eq!(ranges, vec![(1..15, "1")]);
    }

    #[test]
    fn with_range_4() {
        let nums = ["1", "5", "7", "10"];
        let ranges: Vec<_> = nums
            .iter()
            .cloned()
            .with_range(15, |x| x.parse::<u32>().unwrap())
            .collect();
        assert_eq!(
            ranges,
            vec![(1..5, "1"), (5..7, "5"), (7..10, "7"), (10..15, "10")]
        );
    }
}
