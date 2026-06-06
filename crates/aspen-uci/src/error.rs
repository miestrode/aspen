#[derive(Debug, thiserror::Error)]
pub enum UciParseError {
    #[error("empty command")]
    Empty,
    #[error("unknown command: {0}")]
    UnknownCommand(String),
    #[error("missing argument for `{0}`")]
    MissingArgument(&'static str),
    #[error("invalid integer for `{field}`")]
    InvalidInteger {
        field: &'static str,
        #[source]
        source: std::num::ParseIntError,
    },
    #[error("invalid fen")]
    InvalidFen(#[from] shakmaty::fen::ParseFenError),
    #[error("invalid uci move")]
    InvalidUciMove(#[from] shakmaty::uci::ParseUciMoveError),
    #[error("unknown option: {0}")]
    UnknownOption(String),
}
