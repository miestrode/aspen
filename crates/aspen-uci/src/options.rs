use crate::UciParseError;

#[derive(Debug, Clone)]
pub enum UciOptionKind {
    Check {
        default: bool,
    },
    Spin {
        default: i64,
        min: i64,
        max: i64,
    },
    Combo {
        default: String,
        variants: Vec<String>,
    },
    Button,
    String {
        default: String,
    },
}

#[derive(Debug, Clone)]
pub struct UciOptionDeclaration {
    pub name: &'static str,
    pub kind: UciOptionKind,
}

pub trait UciOptions {
    fn declarations() -> Vec<UciOptionDeclaration>;
    fn set(&mut self, name: &str, value: Option<&str>) -> Result<(), UciParseError>;
}

impl std::fmt::Display for UciOptionDeclaration {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "option name {} type ", self.name)?;
        write_kind(&self.kind, formatter)
    }
}

fn write_kind(kind: &UciOptionKind, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match kind {
        UciOptionKind::Check { default } => write!(formatter, "check default {default}"),
        UciOptionKind::Spin { default, min, max } => {
            write!(formatter, "spin default {default} min {min} max {max}")
        }
        UciOptionKind::Combo { default, variants } => write_combo(default, variants, formatter),
        UciOptionKind::Button => write!(formatter, "button"),
        UciOptionKind::String { default } => write!(formatter, "string default {default}"),
    }
}

fn write_combo(
    default: &str,
    variants: &[String],
    formatter: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    write!(formatter, "combo default {default}")?;
    variants
        .iter()
        .try_for_each(|variant| write!(formatter, " var {variant}"))
}
