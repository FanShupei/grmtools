//! Build grammars at run-time.

use std::{
    any::type_name,
    borrow::Cow,
    collections::{HashMap, HashSet},
    convert::AsRef,
    env::{current_dir, var},
    error::Error,
    fmt::Debug,
    fs::{self, create_dir_all, read_to_string, File},
    hash::Hash,
    io::Write,
    path::{Path, PathBuf},
};

use lazy_static::lazy_static;
use num_traits::{PrimInt, Unsigned};
use regex::Regex;
use try_from::TryFrom;

use crate::lexer::{LRNonStreamingLexerDef, LexerDef};

const RUST_FILE_EXT: &str = "rs";

lazy_static! {
    static ref RE_TOKEN_ID: Regex = Regex::new(r"^[a-zA-Z_][a-zA-Z_0-9]*$").unwrap();
}

pub enum LexerKind {
    LRNonStreamingLexer,
}

/// Specify the visibility of the module generated by `LexerBuilder`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Visibility {
    /// Module-level visibility only.
    Private,
    /// `pub`
    Public,
    /// `pub(super)`
    PublicSuper,
    /// `pub(self)`
    PublicSelf,
    /// `pub(crate)`
    PublicCrate,
    /// `pub(in {arg})`
    PublicIn(String),
}

impl Visibility {
    fn cow_str(&self) -> Cow<'static, str> {
        match self {
            Visibility::Private => Cow::from(""),
            Visibility::Public => Cow::from("pub"),
            Visibility::PublicSuper => Cow::from("pub(super)"),
            Visibility::PublicSelf => Cow::from("pub(self)"),
            Visibility::PublicCrate => Cow::from("pub(crate)"),
            Visibility::PublicIn(data) => Cow::from(format!("pub(in {})", data)),
        }
    }
}

/// A `LexerBuilder` allows one to specify the criteria for building a statically generated
/// lexer.
pub struct LexerBuilder<'a, StorageT = u32> {
    lexer_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
    lexerkind: LexerKind,
    mod_name: Option<&'a str>,
    visibility: Visibility,
    rule_ids_map: Option<HashMap<String, StorageT>>,
    allow_missing_terms_in_lexer: bool,
    allow_missing_tokens_in_parser: bool,
}

impl<'a, StorageT> LexerBuilder<'a, StorageT>
where
    StorageT: Copy + Debug + Eq + Hash + PrimInt + TryFrom<usize> + Unsigned,
{
    /// Create a new `LexerBuilder`.
    ///
    /// `StorageT` must be an unsigned integer type (e.g. `u8`, `u16`) which is big enough to index
    /// all the tokens, rules, and productions in the lexer and less than or equal in size
    /// to `usize` (e.g. on a 64-bit machine `u128` would be too big). If you are lexing large
    /// files, the additional storage requirements of larger integer types can be noticeable, and
    /// in such cases it can be worth specifying a smaller type. `StorageT` defaults to `u32` if
    /// unspecified.
    ///
    /// # Examples
    ///
    /// ```text
    /// LexerBuilder::<u8>::new()
    ///     .process_file_in_src("grm.l", None)
    ///     .unwrap();
    /// ```
    pub fn new() -> Self {
        LexerBuilder {
            lexer_path: None,
            output_path: None,
            lexerkind: LexerKind::LRNonStreamingLexer,
            mod_name: None,
            visibility: Visibility::Private,
            rule_ids_map: None,
            allow_missing_terms_in_lexer: false,
            allow_missing_tokens_in_parser: true,
        }
    }

    /// Set the input lexer path to a file relative to this project's `src` directory. This will
    /// also set the output path (i.e. you do not need to call [LexerBuilder::output_path]).
    ///
    /// For example if `a/b.l` is passed as `inp` then [LexerBuilder::process] will:
    ///   * use `src/a/b.l` as the input file.
    ///   * write output to a file which can then be imported by calling `lrlex_mod!("a/b.l")`.
    ///   * create a module in that output file named `b_l`.
    ///
    /// You can override the output path and/or module name by calling [LexerBuilder::output_path]
    /// and/or [LexerBuilder::mod_name], respectively, after calling this function.
    ///
    /// This is a convenience function that makes it easier to compile lexer files stored in a
    /// project's `src/` directory: please see [LexerBuilder::process] for additional constraints
    /// and information about the generated files.
    pub fn lexer_in_src_dir<P>(mut self, srcp: P) -> Result<Self, Box<dyn Error>>
    where
        P: AsRef<Path>,
    {
        if !srcp.as_ref().is_relative() {
            return Err(format!(
                "Lexer path '{}' must be a relative path.",
                srcp.as_ref().to_str().unwrap_or("<invalid UTF-8>")
            )
            .into());
        }

        let mut lexp = current_dir()?;
        lexp.push("src");
        lexp.push(srcp.as_ref());
        self.lexer_path = Some(lexp);

        let mut outp = PathBuf::new();
        outp.push(var("OUT_DIR").unwrap());
        outp.push(srcp.as_ref().parent().unwrap().to_str().unwrap());
        create_dir_all(&outp)?;
        let mut leaf = srcp
            .as_ref()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        leaf.push_str(&format!(".{}", RUST_FILE_EXT));
        outp.push(leaf);
        Ok(self.output_path(outp))
    }

    /// Set the input lexer path to `inp`. If specified, you must also call
    /// [LexerBuilder::output_path]. In general it is easier to use
    /// [LexerBuilder::lexer_in_src_dir].
    pub fn lexer_path<P>(mut self, inp: P) -> Self
    where
        P: AsRef<Path>,
    {
        self.lexer_path = Some(inp.as_ref().to_owned());
        self
    }

    /// Set the output lexer path to `outp`.
    pub fn output_path<P>(mut self, outp: P) -> Self
    where
        P: AsRef<Path>,
    {
        self.output_path = Some(outp.as_ref().to_owned());
        self
    }

    /// Set the type of lexer to be generated to `lexerkind`.
    pub fn lexerkind(mut self, lexerkind: LexerKind) -> Self {
        self.lexerkind = lexerkind;
        self
    }

    /// Set the generated module name to `mod_name`. If no module name is specified,
    /// [`process_file`](#method.process_file) will attempt to create a sensible default based on
    /// the input filename.
    pub fn mod_name(mut self, mod_name: &'a str) -> Self {
        self.mod_name = Some(mod_name);
        self
    }

    /// Set the visibility of the generated module to `vis`. Defaults to `Visibility::Private`.
    pub fn visibility(mut self, vis: Visibility) -> Self {
        self.visibility = vis;
        self
    }

    /// Set this lexer builder's map of rule IDs to `rule_ids_map`. By default, lexing rules have
    /// arbitrary, but distinct, IDs. Setting the map of rule IDs (from rule names to `StorageT`)
    /// allows users to synchronise a lexer and parser and to check that all rules are used by both
    /// parts).
    pub fn rule_ids_map<T: std::borrow::Borrow<HashMap<String, StorageT>> + Clone>(
        mut self,
        rule_ids_map: T,
    ) -> Self {
        self.rule_ids_map = Some(rule_ids_map.borrow().to_owned());
        self
    }

    /// Given the filename `a/b.l` as input, statically compile the file `src/a/b.l` into a Rust
    /// module which can then be imported using `lrlex_mod!("a/b.l")`. This is a convenience
    /// function around [`process_file`](struct.LexerBuilder.html#method.process_file) which makes
    /// it easier to compile `.l` files stored in a project's `src/` directory: please see
    /// [`process_file`](#method.process_file) for additional constraints and information about the
    /// generated files.
    #[deprecated(
        since = "0.10.3",
        note = "Please use lexer_in_src_dir() and process() instead"
    )]
    #[allow(deprecated)]
    pub fn process_file_in_src(
        self,
        srcp: &str,
    ) -> Result<(Option<HashSet<String>>, Option<HashSet<String>>), Box<dyn Error>> {
        let mut inp = current_dir()?;
        inp.push("src");
        inp.push(srcp);
        let mut outp = PathBuf::new();
        outp.push(var("OUT_DIR").unwrap());
        outp.push(Path::new(srcp).parent().unwrap().to_str().unwrap());
        create_dir_all(&outp)?;
        let mut leaf = Path::new(srcp)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        leaf.push_str(&format!(".{}", RUST_FILE_EXT));
        outp.push(leaf);
        self.process_file(inp, outp)
    }

    /// Statically compile the `.l` file `inp` into Rust, placing the output into the file `outp`.
    /// The latter defines a module as follows:
    ///
    /// ```text
    ///    mod modname {
    ///      pub fn lexerdef() -> LexerDef<StorageT> { ... }
    ///
    ///      ...
    ///    }
    /// ```
    ///
    /// where:
    ///  * `modname` is either:
    ///    * the module name specified [`mod_name`](#method.mod_name)
    ///    * or, if no module name was explicitly specified, then for the file `/a/b/c.l` the
    ///      module name is `c_l` (i.e. the file's leaf name, minus its extension, with a prefix of
    ///      `_l`).
    #[deprecated(
        since = "0.10.3",
        note = "Please use lexer_path() and process() instead"
    )]
    pub fn process_file<P, Q>(
        mut self,
        inp: P,
        outp: Q,
    ) -> Result<(Option<HashSet<String>>, Option<HashSet<String>>), Box<dyn Error>>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        self.lexer_path = Some(inp.as_ref().to_owned());
        self.output_path = Some(outp.as_ref().to_owned());
        self.process()
    }

    /// Statically compile the `.l` file specified by [LexerBuilder::lexer_path()] into Rust,
    /// placing the output into the file specified by [LexerBuilder::output_path()].
    ///
    /// The generated module follows the form:
    ///
    /// ```text
    ///    mod modname {
    ///      pub fn lexerdef() -> LexerDef<StorageT> { ... }
    ///
    ///      ...
    ///    }
    /// ```
    ///
    /// where:
    ///  * `modname` is either:
    ///    * the module name specified by [LexerBuilder::mod_name()]
    ///    * or, if no module name was explicitly specified, then for the file `/a/b/c.l` the
    ///      module name is `c_l` (i.e. the file's leaf name, minus its extension, with a prefix of
    ///      `_l`).
    pub fn process(
        self,
    ) -> Result<(Option<HashSet<String>>, Option<HashSet<String>>), Box<dyn Error>> {
        let lexerp = self
            .lexer_path
            .as_ref()
            .expect("lexer_path must be specified before processing.");
        let outp = self
            .output_path
            .as_ref()
            .expect("output_path must be specified before processing.");

        let mut lexerdef: Box<dyn LexerDef<StorageT>> = match self.lexerkind {
            LexerKind::LRNonStreamingLexer => {
                Box::new(LRNonStreamingLexerDef::from_str(&read_to_string(&lexerp)?)?)
            }
        };
        let (missing_from_lexer, missing_from_parser) = match self.rule_ids_map {
            Some(ref rim) => {
                // Convert from HashMap<String, _> to HashMap<&str, _>
                let owned_map = rim
                    .iter()
                    .map(|(x, y)| (&**x, *y))
                    .collect::<HashMap<_, _>>();
                let (x, y) = lexerdef.set_rule_ids(&owned_map);
                (
                    x.map(|a| a.iter().map(|&b| b.to_string()).collect::<HashSet<_>>()),
                    y.map(|a| a.iter().map(|&b| b.to_string()).collect::<HashSet<_>>()),
                )
            }
            None => (None, None),
        };

        if !self.allow_missing_terms_in_lexer {
            if let Some(ref mfl) = missing_from_lexer {
                eprintln!("Error: the following tokens are used in the grammar but are not defined in the lexer:");
                for n in mfl {
                    eprintln!("    {}", n);
                }
                fs::remove_file(&outp).ok();
                panic!();
            }
        }
        if !self.allow_missing_tokens_in_parser {
            if let Some(ref mfp) = missing_from_parser {
                eprintln!("Error: the following tokens are defined in the lexer but not used in the grammar:");
                for n in mfp {
                    eprintln!("    {}", n);
                }
                fs::remove_file(&outp).ok();
                panic!();
            }
        }

        let mod_name = match self.mod_name {
            Some(s) => s.to_owned(),
            None => {
                // The user hasn't specified a module name, so we create one automatically: what we
                // do is strip off all the filename extensions (note that it's likely that inp ends
                // with `l.rs`, so we potentially have to strip off more than one extension) and
                // then add `_l` to the end.
                let mut stem = lexerp.to_str().unwrap();
                loop {
                    let new_stem = Path::new(stem).file_stem().unwrap().to_str().unwrap();
                    if stem == new_stem {
                        break;
                    }
                    stem = new_stem;
                }
                format!("{}_l", stem)
            }
        };

        let mut outs = String::new();
        //
        // Header

        let (lexerdef_name, lexerdef_type) = match self.lexerkind {
            LexerKind::LRNonStreamingLexer => (
                "LRNonStreamingLexerDef",
                format!("LRNonStreamingLexerDef<{}>", type_name::<StorageT>()),
            ),
        };

        outs.push_str(&format!(
            "{mod_vis} mod {mod_name} {{
use lrlex::{{LexerDef, LRNonStreamingLexerDef, Rule}};

#[allow(dead_code)]
pub fn lexerdef() -> {lexerdef_type} {{
    let rules = vec![",
            mod_vis = self.visibility.cow_str(),
            mod_name = mod_name,
            lexerdef_type = lexerdef_type
        ));

        // Individual rules
        for r in lexerdef.iter_rules() {
            let tok_id = match r.tok_id {
                Some(ref t) => format!("Some({:?})", t),
                None => "None".to_owned(),
            };
            let n = match r.name {
                Some(ref n) => format!("Some({:?}.to_string())", n),
                None => "None".to_owned(),
            };
            outs.push_str(&format!(
                "
        Rule::new({}, {}, \"{}\".to_string()).unwrap(),",
                tok_id,
                n,
                r.re_str.replace("\\", "\\\\").replace("\"", "\\\"")
            ));
        }

        // Footer
        outs.push_str(&format!(
            "
    ];
    {lexerdef_name}::from_rules(rules)
}}

",
            lexerdef_name = lexerdef_name
        ));

        // Token IDs
        if let Some(ref rim) = self.rule_ids_map {
            for (n, id) in rim {
                if RE_TOKEN_ID.is_match(n) {
                    outs.push_str(&format!(
                        "#[allow(dead_code)]\npub const T_{}: {} = {:?};\n",
                        n.to_ascii_uppercase(),
                        type_name::<StorageT>(),
                        *id
                    ));
                }
            }
        }

        // Footer
        outs.push('}');

        // If the file we're about to write out already exists with the same contents, then we
        // don't overwrite it (since that will force a recompile of the file, and relinking of the
        // binary etc).
        if let Ok(curs) = read_to_string(&outp) {
            if curs == outs {
                return Ok((missing_from_lexer, missing_from_parser));
            }
        }
        let mut f = File::create(outp)?;
        f.write_all(outs.as_bytes())?;
        Ok((missing_from_lexer, missing_from_parser))
    }

    /// If passed false, tokens used in the grammar but not defined in the lexer will cause a
    /// panic at lexer generation time. Defaults to false.
    pub fn allow_missing_terms_in_lexer(mut self, allow: bool) -> Self {
        self.allow_missing_terms_in_lexer = allow;
        self
    }

    /// If passed false, tokens defined in the lexer but not used in the grammar will cause a
    /// panic at lexer generation time. Defaults to true (since lexers sometimes define tokens such
    /// as reserved words, which are intentionally not in the grammar).
    pub fn allow_missing_tokens_in_parser(mut self, allow: bool) -> Self {
        self.allow_missing_tokens_in_parser = allow;
        self
    }
}
