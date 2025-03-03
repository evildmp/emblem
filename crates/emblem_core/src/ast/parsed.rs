use crate::ast::{text::Text, Dash, File, Glue, Par, ParPart};
use crate::parser::Location;

#[cfg(test)]
use crate::ast::AstDebug;

pub type ParsedFile<'i> = File<ParPart<Content<'i>>>;

#[allow(clippy::large_enum_variant)] // TODO(kcza): re-evaluate this (requires benchmarks)
#[derive(Debug)]
pub enum Content<'i> {
    Shebang {
        text: &'i str,
        loc: Location<'i>,
    },
    Command {
        qualifier: Option<Text<'i>>,
        name: Text<'i>,
        pluses: usize,
        attrs: Option<Attrs<'i>>,
        inline_args: Vec<Vec<Content<'i>>>,
        remainder_arg: Option<Vec<Content<'i>>>,
        trailer_args: Vec<Vec<Par<ParPart<Content<'i>>>>>,
        loc: Location<'i>,
        invocation_loc: Location<'i>,
    },
    Sugar(Sugar<'i>),
    Word {
        word: Text<'i>,
        loc: Location<'i>,
    },
    Whitespace {
        whitespace: &'i str,
        loc: Location<'i>,
    },
    Dash {
        dash: Dash,
        loc: Location<'i>,
    },
    Glue {
        glue: Glue,
        loc: Location<'i>,
    },
    SpiltGlue {
        raw: &'i str,
        loc: Location<'i>,
    },
    Verbatim {
        verbatim: &'i str,
        loc: Location<'i>,
    },
    Comment {
        comment: &'i str,
        loc: Location<'i>,
    },
    MultiLineComment {
        content: MultiLineComment<'i>,
        loc: Location<'i>,
    },
}

#[cfg(test)]
impl AstDebug for Content<'_> {
    fn test_fmt(&self, buf: &mut Vec<String>) {
        match self {
            Self::Shebang { text, .. } => text.surround(buf, "Shebang(", ")"),
            Self::Command {
                qualifier,
                name,
                pluses,
                attrs,
                inline_args,
                remainder_arg,
                trailer_args,
                ..
            } => {
                buf.push('.'.into());
                if let Some(qualifier) = qualifier {
                    qualifier.surround(buf, "(", ")");
                    buf.push('.'.into());
                }
                name.test_fmt(buf);
                if let Some(attrs) = attrs {
                    attrs.test_fmt(buf);
                }
                if *pluses > 0 {
                    "+".repeat(*pluses).surround(buf, "(", ")")
                }
                for arg in inline_args.iter() {
                    arg.surround(buf, "{", "}");
                }
                if let Some(arg) = remainder_arg {
                    buf.push(":".into());
                    arg.test_fmt(buf)
                }
                for arg in trailer_args.iter() {
                    buf.push("::".into());
                    arg.test_fmt(buf);
                }
            }
            Self::Sugar(s) => s.test_fmt(buf),
            Self::Word { word, .. } => word.surround(buf, "Word(", ")"),
            Self::Whitespace { whitespace, .. } => whitespace.surround(buf, "<", ">"),
            Self::Dash { dash, .. } => dash.test_fmt(buf),
            Self::Glue { glue, .. } => glue.test_fmt(buf),
            Self::SpiltGlue { raw, .. } => raw.surround(buf, "SpiltGlue(", ")"),
            Self::Verbatim { verbatim, .. } => verbatim.surround(buf, "!", "!"),
            Self::Comment { comment, .. } => {
                buf.push("//".into());
                comment.test_fmt(buf);
            }
            Self::MultiLineComment { content, .. } => content.surround(buf, "/*", "*/"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Attrs<'i> {
    attrs: Vec<Attr<'i>>,
    loc: Location<'i>,
}

impl<'i> Attrs<'i> {
    pub fn new(attrs: Vec<Attr<'i>>, loc: Location<'i>) -> Self {
        Self { attrs, loc }
    }

    pub fn args(&self) -> &[Attr<'i>] {
        &self.attrs
    }

    pub fn loc(&self) -> &Location<'i> {
        &self.loc
    }
}

#[cfg(test)]
impl AstDebug for Attrs<'_> {
    fn test_fmt(&self, buf: &mut Vec<String>) {
        self.args().test_fmt(buf);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attr<'i> {
    Named {
        eq_idx: usize,
        raw: &'i str,
        loc: Location<'i>,
    },
    Unnamed {
        raw: &'i str,
        loc: Location<'i>,
    },
}

impl<'i> Attr<'i> {
    pub fn named(raw: &'i str, loc: Location<'i>) -> Self {
        Self::Named {
            eq_idx: raw.find('=').unwrap(),
            raw,
            loc,
        }
    }

    pub fn unnamed(raw: &'i str, loc: Location<'i>) -> Self {
        Self::Unnamed { raw, loc }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Named { raw, eq_idx, .. } => raw[..*eq_idx].trim(),
            Self::Unnamed { raw, .. } => raw.trim(),
        }
    }

    #[allow(dead_code)]
    pub fn value(&self) -> Option<&str> {
        match self {
            Self::Named { raw, eq_idx, .. } => Some(raw[eq_idx + 1..].trim()),
            Self::Unnamed { .. } => None,
        }
    }

    pub fn loc(&self) -> &Location<'i> {
        match self {
            Self::Named { loc, .. } => loc,
            Self::Unnamed { loc, .. } => loc,
        }
    }

    #[allow(dead_code)]
    fn raw(&self) -> &str {
        match self {
            Self::Named { raw, .. } => raw,
            Self::Unnamed { raw, .. } => raw,
        }
    }
}

#[cfg(test)]
impl AstDebug for Attr<'_> {
    fn test_fmt(&self, buf: &mut Vec<String>) {
        match self {
            Self::Unnamed { .. } => {
                self.raw().surround(buf, "(", ")");
            }
            Self::Named { eq_idx, .. } => {
                let raw = self.raw();
                (&raw[..*eq_idx]).surround(buf, "(", ")");
                buf.push("=".into());
                (&raw[*eq_idx + 1..]).surround(buf, "(", ")");
            }
        }
    }
}

#[derive(Debug)]
pub enum Sugar<'i> {
    Italic {
        delimiter: &'i str,
        arg: Vec<Content<'i>>,
        loc: Location<'i>,
    },
    Bold {
        delimiter: &'i str,
        arg: Vec<Content<'i>>,
        loc: Location<'i>,
    },
    Monospace {
        arg: Vec<Content<'i>>,
        loc: Location<'i>,
    },
    Smallcaps {
        arg: Vec<Content<'i>>,
        loc: Location<'i>,
    },
    AlternateFace {
        arg: Vec<Content<'i>>,
        loc: Location<'i>,
    },
    Heading {
        level: usize,
        pluses: usize,
        standoff: &'i str,
        arg: Vec<Content<'i>>,
        loc: Location<'i>,
        invocation_loc: Location<'i>,
    },
    Mark {
        mark: &'i str,
        loc: Location<'i>,
    },
    Reference {
        reference: &'i str,
        loc: Location<'i>,
    },
}

impl<'i> Sugar<'i> {
    pub fn call_name(&self) -> &'static str {
        match self {
            Self::Italic { .. } => "it",
            Self::Bold { .. } => "bf",
            Self::Monospace { .. } => "tt",
            Self::Smallcaps { .. } => "sc",
            Self::AlternateFace { .. } => "af",
            Self::Heading { level, .. } => match level {
                1 => "h1",
                2 => "h2",
                3 => "h3",
                4 => "h4",
                5 => "h5",
                6 => "h6",
                _ => panic!("internal error: unknown heading level {level}"),
            },
            Self::Mark { .. } => "mark",
            Self::Reference { .. } => "ref",
        }
    }
}

#[cfg(test)]
impl<'i> AstDebug for Sugar<'i> {
    fn test_fmt(&self, buf: &mut Vec<String>) {
        buf.push(format!("${}", self.call_name()));
        match self {
            Self::Italic { arg, delimiter, .. } => {
                delimiter.surround(buf, "(", ")");
                arg.surround(buf, "{", "}");
            }
            Self::Bold { arg, delimiter, .. } => {
                delimiter.surround(buf, "(", ")");
                arg.surround(buf, "{", "}");
            }
            Self::Monospace { arg, .. } => {
                arg.surround(buf, "{", "}");
            }
            Self::Smallcaps { arg, .. } => {
                arg.surround(buf, "{", "}");
            }
            Self::AlternateFace { arg, .. } => {
                arg.surround(buf, "{", "}");
            }
            Self::Heading { arg, pluses, .. } => {
                if *pluses > 0 {
                    "+".repeat(*pluses).surround(buf, "(", ")");
                }
                arg.surround(buf, "{", "}");
            }
            Self::Mark { mark, .. } => {
                mark.surround(buf, "[", "]");
            }
            Self::Reference { reference, .. } => {
                reference.surround(buf, "[", "]");
            }
        }
    }
}

#[derive(Debug)]
pub struct MultiLineComment<'i>(pub Vec<MultiLineCommentPart<'i>>);

#[cfg(test)]
impl AstDebug for MultiLineComment<'_> {
    fn test_fmt(&self, buf: &mut Vec<String>) {
        self.0.test_fmt(buf);
    }
}

#[derive(Debug)]
pub enum MultiLineCommentPart<'i> {
    Newline,
    Comment(&'i str),
    Nested(MultiLineComment<'i>),
}

#[cfg(test)]
impl AstDebug for MultiLineCommentPart<'_> {
    fn test_fmt(&self, buf: &mut Vec<String>) {
        match self {
            Self::Newline => buf.push(r"\n".into()),
            Self::Comment(w) => w.test_fmt(buf),
            Self::Nested(c) => {
                buf.push("Nested".into());
                c.test_fmt(buf);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::parser::Point;

    mod attrs {
        use super::*;
        use crate::FileName;

        #[test]
        fn args() {
            let p1 = Point::new(FileName::new("fname.em"), "helloworld");
            let p2 = p1.clone().shift("hello");
            let p3 = p2.clone().shift("world");
            let tests = vec![
                vec![],
                vec![
                    Attr::unnamed("hello", Location::new(&p1, &p2)),
                    Attr::unnamed("world", Location::new(&p2, &p3)),
                ],
            ];

            for test in tests {
                assert_eq!(
                    Attrs::new(test.clone(), Location::new(&p1, &p2)).args(),
                    &test
                );
            }
        }
    }

    mod attr {
        use super::*;
        use crate::FileName;

        #[test]
        fn unnamed() {
            let raw = " \tfoo\t ";
            let p1 = Point::new(FileName::new("fname.em"), raw);
            let attr = Attr::unnamed(raw, Location::new(&p1, &p1.clone().shift(raw)));

            assert_eq!(attr.name(), "foo");
            assert_eq!(attr.value(), None);
            assert_eq!(attr.raw(), raw);
        }

        #[test]
        fn named() {
            let raw = " \tfoo\t =\t bar \t";
            let p1 = Point::new(FileName::new("fname.em"), raw);
            let attr = Attr::named(raw, Location::new(&p1, &p1.clone().shift(raw)));

            assert_eq!(attr.name(), "foo");
            assert_eq!(attr.value(), Some("bar"));
            assert_eq!(attr.raw(), raw);
        }
    }

    mod sugar {
        use super::*;
        use crate::FileName;

        #[test]
        fn call_name() {
            let text = "hello, world!";
            let p1 = Point::new(FileName::new("main.em"), text);
            let p2 = p1.clone().shift(text);
            let loc = Location::new(&p1, &p2);

            assert_eq!(
                "it",
                Sugar::Italic {
                    delimiter: "_",
                    arg: vec![],
                    loc: loc.clone()
                }
                .call_name()
            );
            assert_eq!(
                "bf",
                Sugar::Bold {
                    delimiter: "**",
                    arg: vec![],
                    loc: loc.clone()
                }
                .call_name()
            );
            assert_eq!(
                "tt",
                Sugar::Monospace {
                    arg: vec![],
                    loc: loc.clone()
                }
                .call_name()
            );
            assert_eq!(
                "sc",
                Sugar::Smallcaps {
                    arg: vec![],
                    loc: loc.clone()
                }
                .call_name()
            );
            assert_eq!(
                "af",
                Sugar::AlternateFace {
                    arg: vec![],
                    loc: loc.clone()
                }
                .call_name()
            );

            for level in 1..=6 {
                for pluses in 0..=2 {
                    assert_eq!(
                        format!("h{level}"),
                        Sugar::Heading {
                            level,
                            pluses,
                            standoff: " ",
                            arg: vec![],
                            loc: loc.clone(),
                            invocation_loc: loc.clone(),
                        }
                        .call_name()
                    );
                }
            }
        }
    }
}
