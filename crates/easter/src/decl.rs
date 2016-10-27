use joker::track::*;

use id::Id;
use fun::Fun;
use patt::{Patt, CompoundPatt, OptionalPatt, DefaultPatt};
use expr::Expr;

#[derive(Debug, PartialEq)]
pub enum Decl {
    Fun(Fun)
}

impl TrackingRef for Decl {
    fn tracking_ref(&self) -> &Option<Span> {
        let Decl::Fun(ref fun) = *self;
        fun.tracking_ref()
    }
}

impl TrackingMut for Decl {
    fn tracking_mut(&mut self) -> &mut Option<Span> {
        let Decl::Fun(ref mut fun) = *self;
        fun.tracking_mut()
    }
}

impl Untrack for Decl {
    fn untrack(&mut self) {
        let Decl::Fun(ref mut fun) = *self;
        fun.untrack();
    }
}

pub type Dtor = OptionalPatt<Patt<Id>>;

pub trait DtorExt {
    fn from_simple_init(Id, Expr) -> Dtor;
    fn from_compound_init(CompoundPatt<Id>, Expr) -> Dtor;
    fn from_init(Patt<Id>, Expr) -> Dtor;
    fn from_init_opt(Patt<Id>, Option<Expr>) -> Result<Dtor, CompoundPatt<Id>>;
}

impl DtorExt for Dtor {
    fn from_compound_init(lhs: CompoundPatt<Id>, rhs: Expr) -> Dtor {
        Dtor::from_init(Patt::Compound(lhs), rhs)
    }

    fn from_simple_init(lhs: Id, rhs: Expr) -> Dtor {
        Dtor::from_init(Patt::Simple(lhs), rhs)
    }

    fn from_init(lhs: Patt<Id>, rhs: Expr) -> Dtor {
        OptionalPatt::Default(DefaultPatt {
            location: span(&lhs, &rhs),
            patt: lhs,
            default: rhs
        })
    }

    fn from_init_opt(lhs: Patt<Id>, rhs: Option<Expr>) -> Result<Dtor, CompoundPatt<Id>> {
        match (lhs, rhs) {
            (Patt::Compound(patt), None) => Err(patt),
            (lhs, Some(rhs)) => Ok(Dtor::from_init(lhs, rhs)),
            (lhs, None) => Ok(OptionalPatt::Simple(lhs))
        }
    }
}
