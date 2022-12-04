use clap::{
    builder::{OsStr, StringValueParser, TypedValueParser},
    error,
    ArgAction::{Append, Count, Help, Version},
    CommandFactory, Parser, ValueEnum,
    ValueHint::{AnyPath, FilePath},
};
use derive_new::new;
use num_enum::FromPrimitive;
use std::ffi::OsString;
use std::{fs, io, path};

const LONG_ABOUT: &str = "Takes input of a markdown-like document, processes it and typesets it before passing the result to a driver for outputting in some format. Extensions can be used to include arbitrary functionality; device drivers are also extensions.";

/// Parsed command-line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about=LONG_ABOUT, disable_help_flag=true, disable_version_flag=true)]
#[warn(missing_docs)]
pub struct Args {
    /// Pass variable into extension-space
    #[arg(short = 'a', action = Append, value_parser = ExtArg::parser(),  value_name="arg=value")]
    pub extension_args: Vec<ExtArg>,

    /// Colourise log messages
    #[arg(long, value_enum, default_value_t, value_name = "when")]
    pub colour: ColouriseOutput,

    /// Make warnings fatal
    #[arg(short = 'E', default_value_t = false)]
    pub fatal_warnings: bool,

    /// Override detected input format
    #[arg(short, value_name = "format")]
    pub input_driver: Option<String>,

    /// File to typeset
    #[arg(value_name = "in-file", value_hint=FilePath)]
    pub input_file: Option<String>,

    /// Print help information, use `--help` for more detail
    #[arg(short, long, action=Help)]
    help: Option<bool>,

    /// Print info and exit
    #[arg(long = "list", value_enum, value_name = "what")]
    pub list_info: Option<RequestedInfo>,

    /// Limit lua memory usage
    #[arg(long, value_parser = MemoryLimit::parser(), default_value = "unlimited", value_name = "amount")]
    pub max_mem: MemoryLimit,

    /// Override detected output format
    #[arg(short, value_name = "format")]
    pub output_driver: Option<String>,

    /// Output file path
    #[arg(value_name = "out-file", value_hint=AnyPath)]
    pub output_file: Option<String>,

    /// Set root stylesheet
    #[arg(short, value_name = "style")]
    pub style: Option<String>,

    /// Restrict system access
    #[arg(long, value_enum, default_value_t, value_name = "level")]
    pub sandbox: SandboxLevel,

    /// Style search-path, colon-separated
    #[arg(long, env = "EM_STYLE_PATH", value_parser = SearchPath::parser(), default_value = "", value_name = "path")]
    pub style_path: SearchPath,

    /// Set output verbosity
    #[arg(short, action=Count, default_value_t=0, value_name = "level")]
    verbosity_ctr: u8,

    /// Parsed output verbosity
    #[clap(skip)]
    pub verbosity: Verbosity,

    /// Print version info
    #[arg(long, action=Version)]
    version: Option<bool>,

    /// Load an extension
    #[arg(short = 'x', action=Append, value_name = "ext")]
    pub extensions: Vec<String>,

    /// Extension search-path, colon-separated
    #[arg(long, env = "EM_EXT_PATH", value_parser = SearchPath::parser(), default_value = "", value_name = "path")]
    pub extension_path: SearchPath,
}

impl Args {
    /// Parse command-line arguments
    pub fn new() -> Self {
        Args::parse().sanitised()
    }

    /// Validate and infer argument values
    fn sanitised(mut self) -> Self {
        if self.verbosity_ctr >= 3 {
            let mut cmd = Args::command();
            let err = cmd.error(error::ErrorKind::TooManyValues, "too verbose");
            err.exit();
        }
        if let Ok(v) = Verbosity::try_from(self.verbosity_ctr) {
            self.verbosity = v;
        }
        self
    }
}

impl Default for Args {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, I> From<I> for Args
where
    T: Into<OsString> + Clone,
    I: IntoIterator<Item = T>,
{
    fn from(itr: I) -> Self {
        Args::parse_from(itr).sanitised()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemoryLimit {
    Limited(usize),
    Unlimited,
}

impl MemoryLimit {
    fn parser() -> impl TypedValueParser {
        StringValueParser::new().try_map(Self::try_from)
    }
}

impl Default for MemoryLimit {
    fn default() -> Self {
        Self::Unlimited
    }
}

impl TryFrom<OsStr> for MemoryLimit {
    type Error = error::Error;

    fn try_from(raw: OsStr) -> Result<Self, Self::Error> {
        if let Some(s) = raw.to_str() {
            return Self::try_from(s);
        }

        let mut cmd = Args::command();
        Err(cmd.error(
            error::ErrorKind::InvalidValue,
            format!("could not convert '{:?}' to an OS string", raw),
        ))
    }
}

impl TryFrom<String> for MemoryLimit {
    type Error = error::Error;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        Self::try_from(&raw[..])
    }
}

impl TryFrom<&str> for MemoryLimit {
    type Error = error::Error;

    fn try_from(raw: &str) -> Result<Self, Self::Error> {
        if raw.is_empty() {
            let mut cmd = Args::command();
            return Err(cmd.error(error::ErrorKind::InvalidValue, "need amount"));
        }

        if raw == "unlimited" {
            return Ok(Self::Unlimited);
        }

        let (raw_amt, unit): (String, String) = raw.chars().partition(|c| c.is_numeric());

        let amt: usize = match raw_amt.parse() {
            Ok(a) => a,
            Err(e) => {
                let mut cmd = Args::command();
                return Err(cmd.error(error::ErrorKind::InvalidValue, e));
            }
        };

        let multiplier: usize = {
            match &unit[..] {
                "K" => 1 << 10,
                "M" => 1 << 20,
                "G" => 1 << 30,
                "" => 1,
                _ => {
                    let mut cmd = Args::command();
                    return Err(cmd.error(
                        error::ErrorKind::InvalidValue,
                        format!("unrecognised unit: {}", unit),
                    ));
                }
            }
        };

        Ok(Self::Limited(amt * multiplier))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchPath {
    path: Vec<path::PathBuf>,
}

impl SearchPath {
    fn parser() -> impl TypedValueParser {
        StringValueParser::new().map(Self::from)
    }

    fn normalised(&self) -> Self {
        Self {
            path: self.path.iter().flat_map(|d| d.canonicalize()).collect(),
        }
    }

    pub fn open<S, T>(&self, src: S, target: T) -> Result<SearchResult, io::Error>
    where
        S: Into<path::PathBuf>,
        T: AsRef<path::Path>,
    {
        let target = target.as_ref();

        if target.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Absolute paths are forbidden: got {:?}", target,),
            ));
        }

        let src = src.into().canonicalize()?;

        let localpath = path::PathBuf::from(&src).join(target);
        if localpath.starts_with(&src) {
            if let Ok(f) = fs::File::open(&localpath) {
                if let Ok(metadata) = f.metadata() {
                    if metadata.is_file() {
                        return Ok(SearchResult::new(localpath, f));
                    }
                }
            }
        }

        for dir in self.normalised().path {
            let needle = {
                let p = path::PathBuf::from(&dir).join(target);
                match p.canonicalize() {
                    Ok(p) => p,
                    _ => continue,
                }
            };

            if !needle.starts_with(&dir) {
                continue;
            }

            if let Ok(f) = fs::File::open(&needle) {
                if let Ok(metadata) = f.metadata() {
                    if metadata.is_file() {
                        return Ok(SearchResult::new(needle, f));
                    }
                }
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Could not find file {:?} along path \"{}\"",
                target.as_os_str(),
                self.to_string()
            ),
        ))
    }
}

impl ToString for SearchPath {
    fn to_string(&self) -> String {
        self.path
            .iter()
            .map(|dir| dir.to_str().unwrap_or("?"))
            .collect::<Vec<&str>>()
            .join(":")
    }
}

impl From<String> for SearchPath {
    fn from(raw: String) -> Self {
        Self::from(&raw[..])
    }
}

impl From<&str> for SearchPath {
    fn from(raw: &str) -> Self {
        Self {
            path: raw
                .split(':')
                .filter(|s| !s.is_empty())
                .map(|s| s.into())
                .collect(),
        }
    }
}

impl<S> From<Vec<S>> for SearchPath
where
    S: Into<path::PathBuf>,
{
    fn from(raw: Vec<S>) -> Self {
        let mut path = vec![];
        for p in raw {
            path.push(p.into());
        }
        Self { path }
    }
}

#[derive(Debug, new)]
pub struct SearchResult {
    path: path::PathBuf,
    file: fs::File,
}

impl SearchResult {
    pub fn path(&self) -> &path::Path {
        &self.path
    }

    pub fn file(&self) -> &fs::File {
        &self.file
    }
}

#[derive(ValueEnum, Clone, Debug)]
pub enum RequestedInfo {
    InputFormats,
    InputExtensions,
    OutputFormats,
    OutputExtensions,
}

#[derive(ValueEnum, Clone, Debug, Default, FromPrimitive, Eq, PartialEq)]
#[repr(u8)]
pub enum Verbosity {
    /// Output errors and warnings
    #[default]
    Terse,

    /// Output more information about what's going on
    Verbose,

    /// Show debugging info (very verbose)
    Debug,
}

#[derive(ValueEnum, Clone, Debug, Default, PartialEq, Eq)]
pub enum SandboxLevel {
    /// Extensions have no restrictions placed upon them.
    Unrestricted,

    /// Prohibit creation of new subprocesses and file system access outside of the current
    /// working directory.
    #[default]
    Standard,

    /// Same restrictions as Standard, but all file system access if prohibited.
    Strict,
}

#[derive(ValueEnum, Clone, Debug, Default, PartialEq, Eq)]
pub enum ColouriseOutput {
    Never,
    #[default]
    Auto,
    Always,
}

/// Command-line arg declaration
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtArg {
    raw: String,
    eq_idx: usize,
}

impl ExtArg {
    pub fn parser() -> impl TypedValueParser {
        StringValueParser::new().try_map(Self::try_from)
    }

    pub fn name(&self) -> &str {
        &self.raw[..self.eq_idx]
    }

    pub fn value(&self) -> &str {
        &self.raw[self.eq_idx + 1..]
    }
}

impl TryFrom<String> for ExtArg {
    type Error = error::Error;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        match raw.chars().position(|c| c == '=') {
            Some(0) => {
                let mut cmd = Args::command();
                Err(cmd.error(error::ErrorKind::InvalidValue, "need argument name"))
            }
            Some(loc) => Ok(Self { raw, eq_idx: loc }),
            None => {
                let mut cmd = Args::command();
                Err(cmd.error(error::ErrorKind::InvalidValue, "need a value"))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    mod args {
        use super::*;

        #[test]
        fn debug_assert() {
            Args::command().debug_assert()
        }

        #[test]
        fn colourise_output() {
            assert_eq!(Args::from(&["em"]).colour, ColouriseOutput::Auto);
            assert_eq!(
                Args::from(&["em", "--colour", "never"]).colour,
                ColouriseOutput::Never
            );
            assert_eq!(
                Args::from(&["em", "--colour", "auto"]).colour,
                ColouriseOutput::Auto
            );
            assert_eq!(
                Args::from(&["em", "--colour", "always"]).colour,
                ColouriseOutput::Always
            );
        }

        #[test]
        fn fatal_warnings() {
            assert!(!Args::from(&["em"]).fatal_warnings);
            assert!(Args::from(&["em", "-E"]).fatal_warnings);
        }

        #[test]
        fn input_driver() {
            assert_eq!(Args::from(&["em"]).input_driver, None);
            assert_eq!(
                Args::from(&["em", "-i", "chickens"]).input_driver,
                Some("chickens".to_owned())
            );
        }

        #[test]
        fn output_driver() {
            assert_eq!(Args::from(&["em"]).output_driver, None);
            assert_eq!(
                Args::from(&["em", "-o", "pies"]).output_driver,
                Some("pies".to_owned())
            );
        }

        #[test]
        fn input_file() {
            assert_eq!(Args::from(&["em"]).input_file, None);
            assert_eq!(
                Args::from(&["em", "chickens"]).input_file,
                Some("chickens".to_owned())
            );
        }

        #[test]
        fn output_file() {
            assert_eq!(Args::from(&["em"]).output_file, None);
            assert_eq!(
                Args::from(&["em", "_", "pies"]).output_file,
                Some("pies".to_owned())
            );
        }

        #[test]
        fn max_mem() {
            assert_eq!(Args::from(&["em"]).max_mem, MemoryLimit::Unlimited);
            assert_eq!(
                Args::from(&["em", "--max-mem", "25"]).max_mem,
                MemoryLimit::Limited(25)
            );
            assert_eq!(
                Args::from(&["em", "--max-mem", "25K"]).max_mem,
                MemoryLimit::Limited(25 * 1024)
            );
            assert_eq!(
                Args::from(&["em", "--max-mem", "25M"]).max_mem,
                MemoryLimit::Limited(25 * 1024 * 1024)
            );
            assert_eq!(
                Args::from(&["em", "--max-mem", "25G"]).max_mem,
                MemoryLimit::Limited(25 * 1024 * 1024 * 1024)
            );
        }

        #[test]
        fn style() {
            assert_eq!(Args::from(&["em"]).style, None);
            assert_eq!(
                Args::from(&["em", "-s", "funk"]).style,
                Some("funk".to_owned())
            );
        }

        #[test]
        fn sandbox() {
            assert_eq!(Args::from(&["em"]).sandbox, SandboxLevel::Standard);
            assert_eq!(
                Args::from(&["em", "--sandbox", "unrestricted"]).sandbox,
                SandboxLevel::Unrestricted
            );
            assert_eq!(
                Args::from(&["em", "--sandbox", "standard"]).sandbox,
                SandboxLevel::Standard
            );
            assert_eq!(
                Args::from(&["em", "--sandbox", "strict"]).sandbox,
                SandboxLevel::Strict
            );
        }

        #[test]
        fn style_path() {
            assert_eq!(Args::from(&["em"]).style_path, SearchPath::default());
            assert_eq!(
                Args::from(&["em", "--style-path", "club:house"]).style_path,
                SearchPath::from(vec!["club".to_owned(), "house".to_owned()])
            );
        }

        #[test]
        fn verbosity() {
            assert_eq!(
                {
                    let empty: [&str; 0] = [];
                    Args::from(empty).verbosity
                },
                Verbosity::Terse
            );
            assert_eq!(Args::from(["em"]).verbosity, Verbosity::Terse);
            assert_eq!(Args::from(["em", "-v"]).verbosity, Verbosity::Verbose);
            assert_eq!(Args::from(["em", "-vv"]).verbosity, Verbosity::Debug);
        }

        #[test]
        fn extensions() {
            let empty: [&str; 0] = [];
            assert_eq!(Args::from(["em"]).extensions, empty);
            assert_eq!(
                Args::from(["em", "-x", "foo", "-x", "bar", "-x", "baz"]).extensions,
                ["foo".to_owned(), "bar".to_owned(), "baz".to_owned()]
            );
        }

        #[test]
        fn extension_args() {
            assert_eq!(Args::from(&["em"]).extension_args, vec![]);

            {
                let valid_ext_args =
                    Args::from(&["em", "-ak=v", "-ak2=v2", "-ak3="]).extension_args;
                assert_eq!(valid_ext_args.len(), 3);
                assert_eq!(valid_ext_args[0].name(), "k");
                assert_eq!(valid_ext_args[0].value(), "v");
                assert_eq!(valid_ext_args[1].name(), "k2");
                assert_eq!(valid_ext_args[1].value(), "v2");
                assert_eq!(valid_ext_args[2].name(), "k3");
                assert_eq!(valid_ext_args[2].value(), "");
            }
        }

        #[test]
        fn extension_path() {
            assert_eq!(Args::from(&["em"]).extension_path, SearchPath::default());
            assert_eq!(
                Args::from(&["em", "--extension-path", "club:house"]).extension_path,
                SearchPath::from(vec!["club".to_owned(), "house".to_owned()])
            );
        }
    }

    mod search_path {
        use super::*;
        use std::io::Read;
        #[test]
        fn search_path_from() {
            assert_eq!(
                SearchPath::from("foo:bar::baz"),
                SearchPath {
                    path: vec!["foo", "bar", "baz"].iter().map(|d| d.into()).collect()
                }
            );

            assert_eq!(
                SearchPath::from("foo:bar::baz".to_owned()),
                SearchPath {
                    path: vec!["foo", "bar", "baz"].iter().map(|d| d.into()).collect()
                }
            );

            assert_eq!(
                SearchPath::from(
                    vec!["foo", "bar", "baz"]
                        .iter()
                        .map(|d| path::PathBuf::from(d))
                        .collect::<Vec<_>>()
                ),
                SearchPath {
                    path: vec!["foo", "bar", "baz"].iter().map(|d| d.into()).collect()
                }
            );
        }

        #[test]
        fn to_string() {
            let path = SearchPath::from("asdf:fdsa: ::q");
            assert_eq!(path.to_string(), "asdf:fdsa: :q");
        }

        fn make_file(tmppath: &path::Path, filepath: &str, content: &str) -> Result<(), io::Error> {
            let path = path::PathBuf::from(tmppath).join(filepath);

            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::write(path, content)
        }

        #[test]
        fn open() -> Result<(), io::Error> {
            let tmpdir = tempfile::tempdir()?;
            let tmppath = tmpdir.path().canonicalize()?;

            make_file(&tmppath, "a.txt", "a")?;
            make_file(&tmppath, "B/b.txt", "b")?;
            make_file(&tmppath, "C1/C2/c.txt", "c")?;
            make_file(&tmppath, "D/d.txt", "c")?;
            make_file(&tmppath, "x.txt", "x")?;

            let raw_path: Vec<path::PathBuf> = vec!["B", "C1", "D"]
                .iter()
                .map(|s| path::PathBuf::from(&tmppath).join(s))
                .collect();
            let path = SearchPath::from(raw_path).normalised();

            {
                let a = path.open(&tmppath, "a.txt");
                assert!(a.is_ok(), "{:?}", a);
                let mut content = String::new();
                let found = a.unwrap();
                assert_eq!(found.path(), tmppath.join("a.txt"));
                found.file().read_to_string(&mut content)?;
                assert_eq!(content, "a");
            }

            {
                let b = path.open(&tmppath, "b.txt");
                assert!(b.is_ok(), "{:?}", b);
                let found = b.unwrap();
                assert_eq!(found.path(), tmppath.join("B/b.txt"));
                let mut content = String::new();
                found.file().read_to_string(&mut content)?;
                assert_eq!(content, "b");
            }

            {
                let c = path.open(&tmppath, "C2/c.txt");
                assert!(c.is_ok());
                let found = c.unwrap();
                assert_eq!(found.path(), tmppath.join("C1/C2/c.txt"));
                let mut content = String::new();
                found.file().read_to_string(&mut content)?;
                assert_eq!(content, "c");
            }

            {
                let c = path.open(&tmppath, "D/d.txt");
                assert!(c.is_ok());
                let found = c.unwrap();
                assert_eq!(found.path(), tmppath.join("D/d.txt"));
                let mut content = String::new();
                found.file().read_to_string(&mut content)?;
                assert_eq!(content, "c");
            }

            {
                let abs_path = tmppath.join("a.txt");
                let abs_result =
                    path.open(&tmppath, &path::PathBuf::from(&abs_path).canonicalize()?);
                assert!(abs_result.is_err());
                let err = abs_result.unwrap_err();
                assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
                assert_eq!(
                    err.to_string(),
                    format!("Absolute paths are forbidden: got {:?}", abs_path,)
                );
            }

            {
                let dir_result = path.open(&tmppath, "D");
                assert!(dir_result.is_err());
                let err = dir_result.unwrap_err();
                assert_eq!(err.kind(), io::ErrorKind::NotFound);
                assert_eq!(
                    err.to_string(),
                    format!(
                        "Could not find file \"D\" along path \"{}\"",
                        path.to_string()
                    )
                );
            }

            {
                let dir_result = path.open(&tmppath, "C2");
                assert!(dir_result.is_err());
                let err = dir_result.unwrap_err();
                assert_eq!(err.kind(), io::ErrorKind::NotFound);
                assert_eq!(
                    err.to_string(),
                    format!(
                        "Could not find file \"C2\" along path \"{}\"",
                        path.to_string()
                    )
                );
            }

            {
                let inaccessible = path.open(&tmppath, "c.txt");
                assert!(inaccessible.is_err());
                let err = inaccessible.unwrap_err();
                assert_eq!(err.kind(), io::ErrorKind::NotFound);
                assert_eq!(
                    err.to_string(),
                    format!(
                        "Could not find file \"c.txt\" along path \"{}\"",
                        path.to_string()
                    )
                );
            }

            {
                let inaccessible = path.open(&tmppath, "../a.txt");
                assert!(inaccessible.is_err());
                let abs_file = inaccessible.unwrap_err();
                assert_eq!(abs_file.kind(), io::ErrorKind::NotFound);
                assert_eq!(
                    abs_file.to_string(),
                    format!(
                        "Could not find file \"../a.txt\" along path \"{}\"",
                        path.to_string()
                    )
                );
            }

            {
                let non_existent = path.open(&tmppath, "non-existent.txt");
                assert!(non_existent.is_err());
                let non_existent = non_existent.unwrap_err();
                assert_eq!(non_existent.kind(), io::ErrorKind::NotFound);
                assert_eq!(
                    non_existent.to_string(),
                    format!(
                        "Could not find file \"non-existent.txt\" along path \"{}\"",
                        path.to_string()
                    )
                );
            }

            Ok(())
        }
    }
    mod search_result {
        use super::*;
        use io::Write;

        #[test]
        fn fields() -> io::Result<()> {
            let tmpdir = tempfile::tempdir()?;
            let path = tmpdir.path().join("fields.txt");
            let mut file = fs::File::create(&path)?;
            file.write(b"asdf")?;

            let s = SearchResult::new(path.clone(), file.try_clone()?);

            assert_eq!(s.path(), &path);
            assert_eq!(
                s.file().metadata().unwrap().len(),
                file.metadata().unwrap().len()
            );

            Ok(())
        }
    }
}
