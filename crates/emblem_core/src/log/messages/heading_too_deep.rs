use crate::log::messages::Message;
use crate::log::{Log, Note, Src};
use crate::parser::Location;
use derive_new::new;

#[derive(Default, new)]
pub struct HeadingTooDeep<'i> {
    loc: Location<'i>,
    level: usize,
}

impl<'i> Message<'i> for HeadingTooDeep<'i> {
    fn log(self) -> Log<'i> {
        Log::error("heading too deep")
            .with_src(Src::new(&self.loc).with_annotation(Note::error(
                &self.loc,
                format!("found a level-{} heading here", self.level),
            )))
            .with_help("headings should be at most level 6")
    }
}
