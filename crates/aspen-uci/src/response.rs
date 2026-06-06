use shakmaty::uci::UciMove;

use crate::UciOptionDeclaration;

#[derive(Debug, Clone)]
pub enum Response {
    IdName(&'static str),
    IdAuthor(&'static str),
    UciOk,
    ReadyOk,
    BestMove {
        best: UciMove,
        ponder: Option<UciMove>,
    },
    Option(UciOptionDeclaration),
}

impl std::fmt::Display for Response {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Response::IdName(name) => write!(formatter, "id name {name}"),
            Response::IdAuthor(author) => write!(formatter, "id author {author}"),
            Response::UciOk => write!(formatter, "uciok"),
            Response::ReadyOk => write!(formatter, "readyok"),
            Response::BestMove { best, ponder } => write_best_move(*best, *ponder, formatter),
            Response::Option(declaration) => write!(formatter, "{declaration}"),
        }
    }
}

fn write_best_move(
    best: UciMove,
    ponder: Option<UciMove>,
    formatter: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    write!(formatter, "bestmove {best}")?;
    match ponder {
        Some(ponder) => write!(formatter, " ponder {ponder}"),
        None => Ok(()),
    }
}
