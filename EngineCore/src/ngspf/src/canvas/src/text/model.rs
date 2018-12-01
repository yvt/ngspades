//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Types for representing text data.
use attrtext;

/// A reference to a text/foreign element consisting a paragraph.
#[derive(Debug, Clone, Copy)]
pub enum ElementRef<'a> {
    /// A text fragment.
    Text(&'a str),

    /// A foreign object.
    ///
    /// A foreign object is treated like a string of `ForeignObject::len()`
    /// bytes length as far as the byte position calculation is concerned.
    /// In most parts of the text layouting algorithm, it's treated as a
    /// private-use character.
    Foreign(&'a ForeignObject),
}

impl<'a> ElementRef<'a> {
    pub fn text(&self) -> Option<&'a str> {
        if let &ElementRef::Text(x) = self {
            Some(x)
        } else {
            None
        }
    }

    pub fn foreign(&self) -> Option<&'a ForeignObject> {
        if let &ElementRef::Foreign(x) = self {
            Some(x)
        } else {
            None
        }
    }
}

/// A foreign object that can be embedded into a paragraph.
pub trait ForeignObject: crate::Debug {
    /// The dimensions of the foreign object.
    fn extents(&self) -> [f64; 2];

    /// Get the byte length used during byte position calculation.
    /// Defaults to `1`.
    fn len(&self) -> usize {
        1
    }
}

/// Types that can be converted to an `ElementRef`.
pub trait AsElementRef {
    fn as_element_ref(&self) -> ElementRef;
}

impl AsElementRef for str {
    fn as_element_ref(&self) -> ElementRef {
        ElementRef::Text(self)
    }
}

impl AsElementRef for String {
    fn as_element_ref(&self) -> ElementRef {
        ElementRef::Text(self)
    }
}

impl<'a> AsElementRef for ElementRef<'a> {
    fn as_element_ref(&self) -> ElementRef {
        *self
    }
}

impl<'a, T: ?Sized + AsElementRef> AsElementRef for &'a T {
    fn as_element_ref(&self) -> ElementRef {
        (**self).as_element_ref()
    }
}

impl<'a> attrtext::Span for ElementRef<'a> {
    fn len(&self) -> usize {
        match self {
            &ElementRef::Text(x) => x.len(),
            &ElementRef::Foreign(x) => x.len(),
        }
    }
}
