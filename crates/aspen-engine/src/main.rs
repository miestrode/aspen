use std::sync::atomic::AtomicBool;
use std::time::Duration;

use aspen_search::{SearchInfo, SearchLimits, search};
use aspen_uci::{Engine, GoParams, Info, Score, UciOptions, run};
use shakmaty::{CastlingMode, Chess, Color, Move, Position};

const MOVE_OVERHEAD_MILLIS: u64 = 30;
const DEFAULT_MOVES_TO_GO: u64 = 20;

#[derive(Debug, Default, UciOptions)]
struct AspenOptions {
    #[uci(name = "Hash", min = 1, max = 1024, default = 16)]
    hash: i64,
}

#[derive(Debug, Default)]
struct AspenEngine {
    options: AspenOptions,
}

impl Engine for AspenEngine {
    type Options = AspenOptions;

    const NAME: &'static str = "aspen";
    const AUTHOR: &'static str = "aspen authors";

    fn options(&mut self) -> &mut Self::Options {
        &mut self.options
    }

    fn go(
        &mut self,
        position: &Chess,
        params: &GoParams,
        stop: &AtomicBool,
        report: &mut dyn FnMut(Info),
    ) -> Option<Move> {
        let limits = search_limits(params, position.turn());
        let mut on_info = |info| report(uci_info(info));
        search(position, &limits, stop, &mut on_info)
    }
}

fn search_limits(params: &GoParams, side_to_move: Color) -> SearchLimits {
    SearchLimits {
        depth: params.depth,
        nodes: params.nodes,
        time: time_budget(params, side_to_move),
        infinite: params.infinite,
    }
}

fn time_budget(params: &GoParams, side_to_move: Color) -> Option<Duration> {
    if params.infinite {
        return None;
    }
    if let Some(move_time) = params.move_time {
        return Some(budget_from_millis(move_time));
    }
    let remaining = side_to_move.fold_wb(params.white_time, params.black_time)?;
    let increment = side_to_move
        .fold_wb(params.white_increment, params.black_increment)
        .unwrap_or(0);
    let divisor = params.moves_to_go.map_or(DEFAULT_MOVES_TO_GO, u64::from).max(1);
    let raw = (remaining / divisor + increment / 2).min(remaining);
    Some(budget_from_millis(raw))
}

fn budget_from_millis(millis: u64) -> Duration {
    Duration::from_millis(millis.saturating_sub(MOVE_OVERHEAD_MILLIS).max(1))
}

fn uci_info(info: SearchInfo) -> Info {
    Info {
        depth: info.depth,
        seldepth: None,
        score: Score::Centipawns(info.score),
        nodes: info.nodes,
        nps: nodes_per_second(info.nodes, info.time),
        time_ms: info.time.as_millis() as u64,
        pv: info
            .pv
            .into_iter()
            .map(|legal_move| legal_move.to_uci(CastlingMode::Standard))
            .collect(),
    }
}

fn nodes_per_second(nodes: u64, time: Duration) -> Option<u64> {
    let seconds = time.as_secs_f64();
    (seconds > 0.0).then(|| (nodes as f64 / seconds) as u64)
}

fn main() -> std::io::Result<()> {
    run(AspenEngine::default())
}
