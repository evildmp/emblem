pub(crate) mod typesetter;

use crate::args::ArgPath;
use crate::context::Context;
use crate::log::messages::Message;
use crate::parser;
use crate::path::SearchResult;
use crate::Action;
use crate::EmblemResult;
use crate::Log;
use derive_new::new;

use self::typesetter::Typesetter;

#[derive(new)]
pub struct Builder {
    input: ArgPath,

    #[allow(unused)]
    output_stem: ArgPath,

    #[allow(unused)]
    output_driver: Option<String>,
}

impl Action for Builder {
    type Response = Option<Vec<(ArgPath, String)>>;

    fn run<'ctx>(&self, ctx: &'ctx mut Context<'ctx>) -> EmblemResult<'ctx, Self::Response> {
        let fname: SearchResult = match self.input.as_ref().try_into() {
            Ok(f) => f,
            Err(e) => return EmblemResult::new(vec![Log::error(e.to_string())], None),
        };

        let root = match parser::parse_file(ctx, fname) {
            Ok(d) => d,
            Err(e) => return EmblemResult::new(vec![e.log()], None),
        };

        let mut ext_state = ctx
            .extension_state()
            .expect("internal error: failed to create Lua state");

        let typesetter = Typesetter::new(ctx, &mut ext_state);
        typesetter.typeset(root).unwrap();

        EmblemResult::new(vec![], Some(vec![]))
    }
}
