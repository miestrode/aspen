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
    Info(Info),
    Option(UciOptionDeclaration),
}

#[derive(Debug, Clone, Copy)]
pub enum Score {
    Centipawns(i32),
    Mate(i32),
}

#[derive(Debug, Clone)]
pub struct Info {
    pub depth: u32,
    pub seldepth: Option<u32>,
    pub score: Score,
    pub nodes: u64,
    pub nps: Option<u64>,
    pub time_ms: u64,
    pub pv: Vec<UciMove>,
}

impl std::fmt::Display for Response {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Response::IdName(name) => write!(formatter, "id name {name}"),
            Response::IdAuthor(author) => write!(formatter, "id author {author}"),
            Response::UciOk => write!(formatter, "uciok"),
            Response::ReadyOk => write!(formatter, "readyok"),
            Response::BestMove { best, ponder } => write_best_move(*best, *ponder, formatter),
            Response::Info(info) => write_info(info, formatter),
            Response::Option(declaration) => write!(formatter, "{declaration}"),
        }
    }
}

impl std::fmt::Display for Score {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Score::Centipawns(value) => write!(formatter, "cp {value}"),
            Score::Mate(value) => write!(formatter, "mate {value}"),
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

fn write_info(info: &Info, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(formatter, "info depth {}", info.depth)?;
    if let Some(seldepth) = info.seldepth {
        write!(formatter, " seldepth {seldepth}")?;
    }
    write!(formatter, " score {}", info.score)?;
    write!(formatter, " nodes {}", info.nodes)?;
    if let Some(nps) = info.nps {
        write!(formatter, " nps {nps}")?;
    }
    write!(formatter, " time {}", info.time_ms)?;
    write_pv(&info.pv, formatter)
}

fn write_pv(pv: &[UciMove], formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    if pv.is_empty() {
        return Ok(());
    }
    write!(formatter, " pv")?;
    pv.iter().try_for_each(|uci_move| write!(formatter, " {uci_move}"))
}
