use std::str::FromStr;

use shakmaty::fen::Fen;
use shakmaty::uci::UciMove;

use crate::UciParseError;

#[derive(Debug, Clone)]
pub enum UciCommand {
    Uci,
    Debug(bool),
    IsReady,
    SetOption { name: String, value: Option<String> },
    UciNewGame,
    Position(PositionSpec),
    Go(GoParams),
    Stop,
    PonderHit,
    Quit,
}

#[derive(Debug, Clone)]
pub enum PositionSpec {
    StartPos { moves: Vec<UciMove> },
    Fen { fen: Fen, moves: Vec<UciMove> },
}

#[derive(Debug, Clone, Default)]
pub struct GoParams {
    pub search_moves: Vec<UciMove>,
    pub ponder: bool,
    pub white_time: Option<u64>,
    pub black_time: Option<u64>,
    pub white_increment: Option<u64>,
    pub black_increment: Option<u64>,
    pub moves_to_go: Option<u32>,
    pub depth: Option<u32>,
    pub nodes: Option<u64>,
    pub mate: Option<u32>,
    pub move_time: Option<u64>,
    pub infinite: bool,
}

impl FromStr for UciCommand {
    type Err = UciParseError;

    fn from_str(line: &str) -> Result<Self, Self::Err> {
        let mut tokens = line.split_whitespace();
        let keyword = tokens.next().ok_or(UciParseError::Empty)?;
        match keyword {
            "uci" => Ok(UciCommand::Uci),
            "isready" => Ok(UciCommand::IsReady),
            "ucinewgame" => Ok(UciCommand::UciNewGame),
            "stop" => Ok(UciCommand::Stop),
            "ponderhit" => Ok(UciCommand::PonderHit),
            "quit" => Ok(UciCommand::Quit),
            "debug" => parse_debug(tokens),
            "setoption" => parse_setoption(tokens),
            "position" => parse_position(tokens).map(UciCommand::Position),
            "go" => parse_go(tokens).map(UciCommand::Go),
            other => Err(UciParseError::UnknownCommand(other.to_owned())),
        }
    }
}

fn parse_debug<'tokens>(
    mut tokens: impl Iterator<Item = &'tokens str>,
) -> Result<UciCommand, UciParseError> {
    match tokens.next() {
        Some("on") => Ok(UciCommand::Debug(true)),
        Some("off") => Ok(UciCommand::Debug(false)),
        _ => Err(UciParseError::MissingArgument("debug")),
    }
}

fn parse_setoption<'tokens>(
    mut tokens: impl Iterator<Item = &'tokens str>,
) -> Result<UciCommand, UciParseError> {
    if tokens.next() != Some("name") {
        return Err(UciParseError::MissingArgument("name"));
    }
    let mut name = String::new();
    let mut value: Option<String> = None;
    for token in tokens.by_ref() {
        if token == "value" {
            value = Some(join_rest(&mut tokens));
            break;
        }
        push_token(&mut name, token);
    }
    if name.is_empty() {
        return Err(UciParseError::MissingArgument("name"));
    }
    Ok(UciCommand::SetOption { name, value })
}

fn parse_position<'tokens>(
    mut tokens: impl Iterator<Item = &'tokens str>,
) -> Result<PositionSpec, UciParseError> {
    match tokens.next().ok_or(UciParseError::MissingArgument("position"))? {
        "startpos" => Ok(PositionSpec::StartPos {
            moves: parse_trailing_moves(tokens)?,
        }),
        "fen" => parse_fen_position(tokens),
        other => Err(UciParseError::UnknownCommand(other.to_owned())),
    }
}

fn parse_fen_position<'tokens>(
    mut tokens: impl Iterator<Item = &'tokens str>,
) -> Result<PositionSpec, UciParseError> {
    let mut fen_text = String::new();
    let mut at_moves = false;
    for token in tokens.by_ref() {
        if token == "moves" {
            at_moves = true;
            break;
        }
        push_token(&mut fen_text, token);
    }
    let fen = Fen::from_ascii(fen_text.as_bytes())?;
    let moves = if at_moves {
        collect_moves(tokens)?
    } else {
        Vec::new()
    };
    Ok(PositionSpec::Fen { fen, moves })
}

fn parse_trailing_moves<'tokens>(
    mut tokens: impl Iterator<Item = &'tokens str>,
) -> Result<Vec<UciMove>, UciParseError> {
    match tokens.next() {
        Some("moves") => collect_moves(tokens),
        _ => Ok(Vec::new()),
    }
}

fn collect_moves<'tokens>(
    tokens: impl Iterator<Item = &'tokens str>,
) -> Result<Vec<UciMove>, UciParseError> {
    tokens
        .map(|token| token.parse::<UciMove>().map_err(UciParseError::from))
        .collect()
}

fn parse_go<'tokens>(
    tokens: impl Iterator<Item = &'tokens str>,
) -> Result<GoParams, UciParseError> {
    let mut tokens = tokens.peekable();
    let mut params = GoParams::default();
    while let Some(token) = tokens.next() {
        match token {
            "ponder" => params.ponder = true,
            "infinite" => params.infinite = true,
            "wtime" => params.white_time = Some(next_int(&mut tokens, "wtime")?),
            "btime" => params.black_time = Some(next_int(&mut tokens, "btime")?),
            "winc" => params.white_increment = Some(next_int(&mut tokens, "winc")?),
            "binc" => params.black_increment = Some(next_int(&mut tokens, "binc")?),
            "movestogo" => params.moves_to_go = Some(next_int(&mut tokens, "movestogo")?),
            "depth" => params.depth = Some(next_int(&mut tokens, "depth")?),
            "nodes" => params.nodes = Some(next_int(&mut tokens, "nodes")?),
            "mate" => params.mate = Some(next_int(&mut tokens, "mate")?),
            "movetime" => params.move_time = Some(next_int(&mut tokens, "movetime")?),
            "searchmoves" => params.search_moves = collect_search_moves(&mut tokens),
            _ => {}
        }
    }
    Ok(params)
}

fn collect_search_moves<'tokens>(
    tokens: &mut std::iter::Peekable<impl Iterator<Item = &'tokens str>>,
) -> Vec<UciMove> {
    let mut moves = Vec::new();
    while let Some(parsed) = tokens.peek().and_then(|token| token.parse::<UciMove>().ok()) {
        moves.push(parsed);
        tokens.next();
    }
    moves
}

fn next_int<'tokens, Integer>(
    tokens: &mut impl Iterator<Item = &'tokens str>,
    field: &'static str,
) -> Result<Integer, UciParseError>
where
    Integer: FromStr<Err = std::num::ParseIntError>,
{
    let raw = tokens.next().ok_or(UciParseError::MissingArgument(field))?;
    raw.parse()
        .map_err(|source| UciParseError::InvalidInteger { field, source })
}

fn join_rest<'tokens>(tokens: &mut impl Iterator<Item = &'tokens str>) -> String {
    let mut joined = String::new();
    tokens.for_each(|token| push_token(&mut joined, token));
    joined
}

fn push_token(target: &mut String, token: &str) {
    if !target.is_empty() {
        target.push(' ');
    }
    target.push_str(token);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uci_move(text: &str) -> UciMove {
        text.parse().unwrap()
    }

    #[test]
    fn parses_keyword_commands() {
        assert!(matches!("uci".parse(), Ok(UciCommand::Uci)));
        assert!(matches!("isready".parse(), Ok(UciCommand::IsReady)));
        assert!(matches!("ucinewgame".parse(), Ok(UciCommand::UciNewGame)));
        assert!(matches!("stop".parse(), Ok(UciCommand::Stop)));
        assert!(matches!("ponderhit".parse(), Ok(UciCommand::PonderHit)));
        assert!(matches!("quit".parse(), Ok(UciCommand::Quit)));
        assert!(matches!("debug on".parse(), Ok(UciCommand::Debug(true))));
        assert!(matches!("debug off".parse(), Ok(UciCommand::Debug(false))));
    }

    #[test]
    fn parses_startpos_with_moves() {
        let UciCommand::Position(PositionSpec::StartPos { moves }) =
            "position startpos moves e2e4 e7e5".parse().unwrap()
        else {
            panic!("expected startpos");
        };
        assert_eq!(moves, [uci_move("e2e4"), uci_move("e7e5")]);
    }

    #[test]
    fn parses_fen_with_moves() {
        let command =
            "position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 moves e2e4"
                .parse()
                .unwrap();
        let UciCommand::Position(PositionSpec::Fen { moves, .. }) = command else {
            panic!("expected fen");
        };
        assert_eq!(moves, [uci_move("e2e4")]);
    }

    #[test]
    fn parses_go_parameters() {
        let UciCommand::Go(params) = "go wtime 1000 btime 2000 movestogo 30 depth 7 infinite"
            .parse()
            .unwrap()
        else {
            panic!("expected go");
        };
        assert_eq!(params.white_time, Some(1000));
        assert_eq!(params.black_time, Some(2000));
        assert_eq!(params.moves_to_go, Some(30));
        assert_eq!(params.depth, Some(7));
        assert!(params.infinite);
    }

    #[test]
    fn parses_go_searchmoves() {
        let UciCommand::Go(params) = "go searchmoves e2e4 d2d4 depth 3".parse().unwrap() else {
            panic!("expected go");
        };
        assert_eq!(params.search_moves, [uci_move("e2e4"), uci_move("d2d4")]);
        assert_eq!(params.depth, Some(3));
    }

    #[test]
    fn parses_setoption_with_spaced_name() {
        let UciCommand::SetOption { name, value } =
            "setoption name Clear Hash".parse().unwrap()
        else {
            panic!("expected setoption");
        };
        assert_eq!(name, "Clear Hash");
        assert_eq!(value, None);

        let UciCommand::SetOption { name, value } =
            "setoption name Hash value 32".parse().unwrap()
        else {
            panic!("expected setoption");
        };
        assert_eq!(name, "Hash");
        assert_eq!(value.as_deref(), Some("32"));
    }

    #[test]
    fn reports_parse_errors() {
        assert!(matches!("".parse::<UciCommand>(), Err(UciParseError::Empty)));
        assert!(matches!(
            "frobnicate".parse::<UciCommand>(),
            Err(UciParseError::UnknownCommand(_))
        ));
        assert!(matches!(
            "position startpos moves z9z9".parse::<UciCommand>(),
            Err(UciParseError::InvalidUciMove(_))
        ));
        assert!(matches!(
            "position fen not a fen".parse::<UciCommand>(),
            Err(UciParseError::InvalidFen(_))
        ));
        assert!(matches!(
            "go depth notanumber".parse::<UciCommand>(),
            Err(UciParseError::InvalidInteger { .. })
        ));
    }
}
