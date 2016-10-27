use joker::track::*;

use id::Id;
use expr::Expr;
use obj::{PropKey, DotKey};

#[derive(Debug, PartialEq)]
pub enum CompoundPatt<T> {
    Arr(PattList<Option<OptionalPatt<Patt<T>>>>),
    Obj(PattList<OptionalPatt<PropPatt<T>>>)
}

impl<T> TrackingRef for CompoundPatt<T> {
    fn tracking_ref(&self) -> &Option<Span> {
        match *self {
            CompoundPatt::Arr(ref patts) => patts.tracking_ref(),
            CompoundPatt::Obj(ref props) => props.tracking_ref()
        }
    }
}

impl<T> TrackingMut for CompoundPatt<T> {
    fn tracking_mut(&mut self) -> &mut Option<Span> {
        match *self {
            CompoundPatt::Arr(ref mut patts) => patts.tracking_mut(),
            CompoundPatt::Obj(ref mut props) => props.tracking_mut()
        }
    }
}

impl<T: Untrack> Untrack for CompoundPatt<T> {
    fn untrack(&mut self) {
        *self.tracking_mut() = None;
    }
}

#[derive(Debug, PartialEq)]
pub struct DefaultPatt<T> {
    pub location: Option<Span>,
    pub patt: T,
    pub default: Expr
}

impl<T> TrackingRef for DefaultPatt<T> {
    fn tracking_ref(&self) -> &Option<Span> { &self.location }
}

impl<T> TrackingMut for DefaultPatt<T> {
    fn tracking_mut(&mut self) -> &mut Option<Span> { &mut self.location }
}

impl<T: Untrack> Untrack for DefaultPatt<T> {
    fn untrack(&mut self) {
        self.location = None;
        self.patt.untrack();
        self.default.untrack();
    }
}

#[derive(Debug, PartialEq)]
pub enum OptionalPatt<T> {
    Simple(T),
    Default(DefaultPatt<T>)
}

impl<T: TrackingRef> TrackingRef for OptionalPatt<T> {
    fn tracking_ref(&self) -> &Option<Span> {
        match *self {
            OptionalPatt::Simple(ref patt) => patt.tracking_ref(),
            OptionalPatt::Default(ref def_patt) => def_patt.tracking_ref()
        }
    }
}

impl<T: TrackingMut> TrackingMut for OptionalPatt<T> {
    fn tracking_mut(&mut self) -> &mut Option<Span> {
        match *self {
            OptionalPatt::Simple(ref mut patt) => patt.tracking_mut(),
            OptionalPatt::Default(ref mut def_patt) => def_patt.tracking_mut()
        }
    }
}

impl<T: Untrack> Untrack for OptionalPatt<T> {
    fn untrack(&mut self) {
        match *self {
            OptionalPatt::Simple(ref mut patt) => patt.untrack(),
            OptionalPatt::Default(ref mut def_patt) => def_patt.untrack()
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct PattList<E> {
    pub location: Option<Span>,
    pub list: Vec<E>,
    // pub rest: Option<Patt<R>>
}

impl<E> TrackingRef for PattList<E> {
    fn tracking_ref(&self) -> &Option<Span> { &self.location }
}

impl<E> TrackingMut for PattList<E> {
    fn tracking_mut(&mut self) -> &mut Option<Span> { &mut self.location }
}

impl<E: Untrack> Untrack for PattList<E> {
    fn untrack(&mut self) {
        self.location = None;
        self.list.untrack();
        // self.rest.untrack();
    }
}

#[derive(Debug, PartialEq)]
pub struct PropPatt<T> {
    pub location: Option<Span>,
    pub key: PropKey,
    pub patt: Patt<T>
}

impl<T> TrackingRef for PropPatt<T> {
    fn tracking_ref(&self) -> &Option<Span> { &self.location }
}

impl<T> TrackingMut for PropPatt<T> {
    fn tracking_mut(&mut self) -> &mut Option<Span> { &mut self.location }
}

impl<T: Untrack> Untrack for PropPatt<T> {
    fn untrack(&mut self) {
        self.location = None;
        self.key.untrack();
        self.patt.untrack();
    }
}

#[derive(Debug, PartialEq)]
pub enum Patt<T> {
    Simple(T),
    Compound(CompoundPatt<T>)
}

impl<T> Patt<T> {
    pub fn is_simple(&self) -> bool {
        match *self {
            Patt::Simple(_)   => true,
            Patt::Compound(_) => false
        }
    }
}

impl<T: TrackingRef> TrackingRef for Patt<T> {
    fn tracking_ref(&self) -> &Option<Span> {
        match *self {
            Patt::Simple(ref simple) => simple.tracking_ref(),
            Patt::Compound(ref patt) => patt.tracking_ref()
        }
    }
}

impl<T: TrackingMut> TrackingMut for Patt<T> {
    fn tracking_mut(&mut self) -> &mut Option<Span> {
        match *self {
            Patt::Simple(ref mut simple) => simple.tracking_mut(),
            Patt::Compound(ref mut patt) => patt.tracking_mut()
        }
    }
}

impl<T: Untrack> Untrack for Patt<T> {
    fn untrack(&mut self) {
        match *self {
            Patt::Simple(ref mut simple) => { simple.untrack(); }
            Patt::Compound(ref mut patt) => { patt.untrack(); }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum AssignTarget {
    Id(Id),
    Dot(Option<Span>, Box<Expr>, DotKey),
    Brack(Option<Span>, Box<Expr>, Box<Expr>)
}

impl TrackingRef for AssignTarget {
    fn tracking_ref(&self) -> &Option<Span> {
        match *self {
            AssignTarget::Id(ref id) => id.tracking_ref(),
            AssignTarget::Dot(ref location, _, _)
          | AssignTarget::Brack(ref location, _, _) => location
        }
    }
}

impl TrackingMut for AssignTarget {
    fn tracking_mut(&mut self) -> &mut Option<Span> {
        match *self {
            AssignTarget::Id(ref mut id) => id.tracking_mut(),
            AssignTarget::Dot(ref mut location, _, _)
          | AssignTarget::Brack(ref mut location, _, _) => location
        }
    }
}

impl Untrack for AssignTarget {
    fn untrack(&mut self) {
        match *self {
            AssignTarget::Id(ref mut id) => {
                id.untrack();
            }
            AssignTarget::Dot(ref mut location, ref mut obj, ref mut prop) => {
                *location = None;
                obj.untrack();
                prop.untrack();
            }
            AssignTarget::Brack(ref mut location, ref mut obj, ref mut prop) => {
                *location = None;
                obj.untrack();
                prop.untrack();
            }
        }
    }
}
