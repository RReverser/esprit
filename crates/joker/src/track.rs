use std::fmt::{self, Debug, Formatter};
use serde::ser::{Serialize, Serializer};

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Posn {
    pub offset: u32,
    pub line: u32,
    pub column: u32
}

impl Posn {
    pub fn origin() -> Posn {
        Posn {
            offset: 0,
            line: 0,
            column: 0
        }
    }
}

impl Debug for Posn {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.write_fmt(format_args!("{}:{}", self.line + 1, self.column + 1))
    }
}

impl Serialize for Posn {
    fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<(), S::Error> {
        let mut state = serializer.serialize_struct("Posn", 2)?;
        serializer.serialize_struct_elt(&mut state, "line", self.line)?;
        serializer.serialize_struct_elt(&mut state, "column", self.column)?;
        serializer.serialize_struct_end(state)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Span {
    pub start: Posn,
    pub end: Posn
}

impl Debug for Span {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.write_fmt(format_args!("{:?}..{:?}", self.start, self.end))
    }
}

impl Serialize for Span {
    fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<(), S::Error> {
        let mut state = serializer.serialize_struct("Span", 2)?;
        serializer.serialize_struct_elt(&mut state, "start", self.start)?;
        serializer.serialize_struct_elt(&mut state, "end", self.end)?;
        serializer.serialize_struct_end(state)
    }
}

pub trait TrackingRef {
    fn tracking_ref(&self) -> &Option<Span>;
}

pub trait TrackingMut: TrackingRef {
    fn tracking_mut(&mut self) -> &mut Option<Span>;
}

impl<'a, T: TrackingRef> TrackingRef for &'a T {
    fn tracking_ref(&self) -> &Option<Span> { (*self).tracking_ref() }
}

impl TrackingRef for Option<Span> {
    fn tracking_ref(&self) -> &Option<Span> { self }
}

impl TrackingMut for Option<Span> {
    fn tracking_mut(&mut self) -> &mut Option<Span> { self }
}

pub trait Untrack {
    fn untrack(&mut self);
}

impl<T> Untrack for Box<T>
  where T: Untrack
{
    fn untrack(&mut self) {
        (**self).untrack();
    }
}

impl<T> Untrack for Option<T>
  where T: Untrack
{
    fn untrack(&mut self) {
        match *self {
            Some(ref mut x) => { x.untrack(); }
            None => { }
        }
    }
}

impl Untrack for Option<Span> {
    fn untrack(&mut self) {
        *self = None;
    }
}

impl<T> Untrack for Vec<T>
  where T: Untrack
{
    fn untrack(&mut self) {
        for x in self {
            x.untrack();
        }
    }
}

pub fn span<T, U>(left: &T, right: &U) -> Option<Span>
  where T: TrackingRef,
        U: TrackingRef
{
    match (*left.tracking_ref(), *right.tracking_ref()) {
        (Some(l), Some(r)) => Some(Span { start: l.start, end: r.end }),
        _ => None
    }
}
