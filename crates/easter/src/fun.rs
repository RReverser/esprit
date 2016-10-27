use joker::track::*;

use id::Id;
use patt::PattList;
use stmt::StmtListItem;
use decl::Dtor;

pub type Params = PattList<Dtor>;

#[derive(Debug, PartialEq)]
pub struct Fun {
    pub location: Option<Span>,
    pub id: Option<Id>,
    pub params: Params,
    pub body: Vec<StmtListItem>
}

impl TrackingRef for Fun {
    fn tracking_ref(&self) -> &Option<Span> { &self.location }
}

impl TrackingMut for Fun {
    fn tracking_mut(&mut self) -> &mut Option<Span> { &mut self.location }
}

impl Untrack for Fun {
    fn untrack(&mut self) {
        self.location = None;
        self.id.untrack();
        self.params.untrack();
        self.body.untrack();
    }
}
