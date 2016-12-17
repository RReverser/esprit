use std::fmt;
use std::fmt::{Display, Formatter};
use joker::track::{Span, TrackingRef};
use expr::Expr;
use patt::{Patt, AssignTarget, CompoundPatt, PropPatt};
use obj::{Prop, PropVal};
use vecutil::VecUtil;

#[derive(Debug, PartialEq, Clone)]
pub enum Error {
    InvalidAssignTarget(Option<Span>),
    InvalidPropPatt(Option<Span>)
}

impl Display for Error {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        match self {
            &Error::InvalidAssignTarget(_) => {
                fmt.write_str("invalid assignment pattern")
            }
            &Error::InvalidPropPatt(_) => {
                fmt.write_str("invalid object property in assignment pattern")
            }
        }
    }
}

pub trait IntoAssignTarget {
    fn into_assign_target(self) -> Result<AssignTarget, Error>;
}

impl IntoAssignTarget for Expr {
    fn into_assign_target(self) -> Result<AssignTarget, Error> {
        Ok(match self {
            Expr::Id(id)                     => AssignTarget::Id(id),
            Expr::Dot(location, obj, key)    => AssignTarget::Dot(location, obj, key),
            Expr::Brack(location, obj, prop) => AssignTarget::Brack(location, obj, prop),
            _ => { return Err(Error::InvalidAssignTarget(*self.tracking_ref())); }
        })
    }
}

pub trait IntoAssignPatt : IntoAssignTarget {
    fn into_assign_patt(self) -> Result<Patt<AssignTarget>, Error>;
}

impl IntoAssignPatt for Expr {
    fn into_assign_patt(self) -> Result<Patt<AssignTarget>, Error> {
        Ok(Patt::Compound(match self {
            Expr::Obj(location, props) => {
                CompoundPatt::Obj(location, props.map(|prop| prop.into_assign_prop())?)
            }
            Expr::Arr(location, exprs) => {
                CompoundPatt::Arr(location, exprs.map(|expr| match expr {
                    Some(expr) => expr.into_assign_patt().map(Some),
                    None => Ok(None)
                })?)
            }
            _ => { return self.into_assign_target().map(Patt::Simple); }
        }))
    }
}

pub trait IntoAssignProp {
    fn into_assign_prop(self) -> Result<PropPatt<AssignTarget>, Error>;
}

impl IntoAssignProp for Prop {
    fn into_assign_prop(self) -> Result<PropPatt<AssignTarget>, Error> {
        let location = *self.tracking_ref();
        let key = self.key;
        let patt = match self.val {
            PropVal::Init(expr) => expr.into_assign_patt()?,
            _ => { return Err(Error::InvalidPropPatt(*self.val.tracking_ref())); }
        };
        Ok(PropPatt { location: location, key: key, patt: patt })
    }
}
