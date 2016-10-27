use unjson::ty::Object;
use easter::prog::Script;
use result::Result;
use node::ExtractNode;

pub trait IntoScript {
    fn into_script(self) -> Result<Script>;
}

impl IntoScript for Object {
    fn into_script(mut self) -> Result<Script> {
        let body = try!(self.extract_stmt_list("body"));
        Ok(Script { location: try!(self.extract_loc()), body: body })
    }
}
