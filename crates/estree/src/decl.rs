use easter::decl::{Dtor, DtorExt};
use unjson::ty::Object;

use result::Result;
use error::Error;
use node::ExtractNode;
use easter::patt::OptionalPatt;
use tag::{Tag, TagOf};
use patt::IntoPatt;

pub trait IntoDecl {
    fn into_dtor(self) -> Result<Dtor>;
}

impl IntoDecl for Object {
    fn into_dtor(mut self) -> Result<Dtor> {
        match try!(self.tag()) {
            Tag::VariableDeclarator => {
                let lhs = try!(self.extract_patt("id"));
                let init = try!(self.extract_expr_opt("init"));
                Dtor::from_init_opt(lhs, init).map_err(Error::UninitializedPattern)
            }
            Tag::AssignmentPattern => {
                let lhs = try!(self.extract_patt("left"));
                let rhs = try!(self.extract_expr("right"));
                Ok(Dtor::from_init(lhs, rhs))
            }
            _ => self.into_patt().map(OptionalPatt::Simple)
        }
    }
}
