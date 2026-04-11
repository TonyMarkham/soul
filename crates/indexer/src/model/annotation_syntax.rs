#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotationSyntax(pub String);

impl std::fmt::Display for AnnotationSyntax {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for AnnotationSyntax {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}
