#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotationSyntax {
    RustAttribute,
    CSharpAttribute,
}

impl std::fmt::Display for AnnotationSyntax {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RustAttribute => f.write_str("rust_attribute"),
            Self::CSharpAttribute => f.write_str("csharp_attribute"),
        }
    }
}

impl std::str::FromStr for AnnotationSyntax {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "rust_attribute" => Ok(Self::RustAttribute),
            "csharp_attribute" => Ok(Self::CSharpAttribute),
            other => Err(format!("unknown annotation syntax: `{other}`")),
        }
    }
}
