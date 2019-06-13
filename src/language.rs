use failure::Fail;
use std::str::FromStr;
use derive_more::Display;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Display)]
pub enum Language {
    #[display(fmt = "C")]
    C,
    #[display(fmt = "C#")]
    CSharp,
    #[display(fmt = "C++")]
    CPlusPlus,
    #[display(fmt = "Cobol")]
    Cobol,
    #[display(fmt = "Go")]
    Go,
    #[display(fmt = "Haskell")]
    Haskell,
    #[display(fmt = "Java")]
    Java,
    #[display(fmt = "Node.js")]
    NodeJs,
    #[display(fmt = "SpiderMonkey")]
    SpiderMonkey,
    #[display(fmt = "Kotlin")]
    Kotlin,
    #[display(fmt = "Common Lisp")]
    CommonLisp,
    #[display(fmt = "Objective-C")]
    ObjectiveC,
    #[display(fmt = "OCaml")]
    OCaml,
    #[display(fmt = "Pascal")]
    Pascal,
    #[display(fmt = "PHP")]
    Php,
    #[display(fmt = "Prolog")]
    Prolog,
    #[display(fmt = "Python 2")]
    Python2,
    #[display(fmt = "Python 3")]
    Python3,
    #[display(fmt = "Ruby")]
    Ruby,
    #[display(fmt = "Rust")]
    Rust,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Fail)]
pub enum LanguageParseError {
    #[fail(display = "Unknown language: {:?}", _0)]
    UnknownLanguage(String),
}

impl FromStr for Language {
    type Err = LanguageParseError;

    fn from_str(text: &str) -> Result<Language, Self::Err> {
        match text.to_lowercase().as_str() {
            "c" => Ok(Language::C),
            "c#" => Ok(Language::CSharp),
            "cpp" | "cxx" | "C++" => Ok(Language::CPlusPlus),
            "cobol" => Ok(Language::Cobol),
            "go" => Ok(Language::Go),
            "haskell" => Ok(Language::Haskell),
            "java" => Ok(Language::Java),
            "nodejs" | "node.js" | "js" | "node" => Ok(Language::NodeJs),
            "spidermonkey" | "spider monkey" => Ok(Language::SpiderMonkey),
            "kotlin" => Ok(Language::Kotlin),
            "commonlisp" | "lisp" | "common lisp" => Ok(Language::CommonLisp),
            "objectivec" | "objective-c" => Ok(Language::ObjectiveC),
            "ocaml" => Ok(Language::OCaml),
            "pascal" => Ok(Language::Pascal),
            "php" => Ok(Language::Php),
            "prolog" => Ok(Language::Prolog),
            "python2" | "python 2" => Ok(Language::Python2),
            "python3" | "python 3" => Ok(Language::Python3),
            "ruby" => Ok(Language::Ruby),
            "rust" => Ok(Language::Rust),
            _ => Err(LanguageParseError::UnknownLanguage(text.to_owned())),
        }
    }
}

