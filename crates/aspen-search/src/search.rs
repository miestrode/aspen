use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use aspen_eval::evaluate;
use shakmaty::{Chess, Move, Position};

const INFINITY: i32 = 30_000;
const MATE: i32 = 29_000;
const MAX_DEPTH: u32 = 64;
const CHECKPOINT_INTERVAL: u64 = 2048;

#[derive(Debug, Clone, Default)]
pub struct SearchLimits {
    pub depth: Option<u32>,
    pub nodes: Option<u64>,
    pub time: Option<Duration>,
    pub infinite: bool,
}

#[derive(Debug, Clone)]
pub struct SearchInfo {
    pub depth: u32,
    pub score: i32,
    pub nodes: u64,
    pub time: Duration,
    pub pv: Vec<Move>,
}

pub fn search(
    position: &Chess,
    limits: &SearchLimits,
    stop: &AtomicBool,
    report: &mut dyn FnMut(SearchInfo),
) -> Option<Move> {
    let start = Instant::now();
    let mut searcher = Searcher::new(start, limits, stop);
    let mut best_move = None;
    let max_depth = limits.depth.unwrap_or(MAX_DEPTH).min(MAX_DEPTH);
    for depth in 1..=max_depth {
        let mut pv = Vec::new();
        let score = searcher.negamax(position, depth, 0, -INFINITY, INFINITY, &mut pv);
        if searcher.aborted {
            break;
        }
        best_move = pv.first().copied();
        report(SearchInfo {
            depth,
            score,
            nodes: searcher.nodes,
            time: start.elapsed(),
            pv,
        });
        if searcher.should_stop_deepening() {
            break;
        }
    }
    best_move.or_else(|| position.legal_moves().first().copied())
}

#[derive(Debug)]
struct Searcher<'a> {
    stop: &'a AtomicBool,
    start: Instant,
    deadline: Option<Instant>,
    budget: Option<Duration>,
    node_limit: Option<u64>,
    nodes: u64,
    aborted: bool,
}

impl<'a> Searcher<'a> {
    fn new(start: Instant, limits: &SearchLimits, stop: &'a AtomicBool) -> Self {
        let budget = (!limits.infinite).then_some(limits.time).flatten();
        Searcher {
            stop,
            start,
            deadline: budget.map(|budget| start + budget),
            budget,
            node_limit: limits.nodes,
            nodes: 0,
            aborted: false,
        }
    }

    fn negamax(
        &mut self,
        position: &Chess,
        depth: u32,
        ply: u32,
        mut alpha: i32,
        beta: i32,
        pv: &mut Vec<Move>,
    ) -> i32 {
        if self.should_abort() {
            return 0;
        }
        if depth == 0 {
            pv.clear();
            return evaluate(position);
        }
        let moves = position.legal_moves();
        if moves.is_empty() {
            pv.clear();
            return terminal_score(position, ply);
        }
        let mut best = -INFINITY;
        let mut child_pv = Vec::new();
        for legal_move in &moves {
            let mut child = position.clone();
            child.play_unchecked(*legal_move);
            self.nodes += 1;
            let score = -self.negamax(&child, depth - 1, ply + 1, -beta, -alpha, &mut child_pv);
            if self.aborted {
                return best;
            }
            if score > best {
                best = score;
                extend_principal_variation(pv, *legal_move, &child_pv);
            }
            alpha = alpha.max(score);
            if alpha >= beta {
                break;
            }
        }
        best
    }

    fn should_abort(&mut self) -> bool {
        if self.aborted {
            return true;
        }
        if self.node_limit.is_some_and(|limit| self.nodes >= limit) {
            self.aborted = true;
            return true;
        }
        if self.nodes.is_multiple_of(CHECKPOINT_INTERVAL) && self.reached_external_limit() {
            self.aborted = true;
        }
        self.aborted
    }

    fn reached_external_limit(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
            || self.deadline.is_some_and(|deadline| Instant::now() >= deadline)
    }

    fn should_stop_deepening(&self) -> bool {
        self.budget
            .is_some_and(|budget| self.start.elapsed() * 2 >= budget)
    }
}

fn terminal_score(position: &Chess, ply: u32) -> i32 {
    if position.is_check() {
        -MATE + ply as i32
    } else {
        0
    }
}

fn extend_principal_variation(pv: &mut Vec<Move>, head: Move, tail: &[Move]) {
    pv.clear();
    pv.push(head);
    pv.extend_from_slice(tail);
}

#[cfg(test)]
mod tests {
    use shakmaty::CastlingMode;
    use shakmaty::fen::Fen;

    use super::*;

    fn position(fen: &str) -> Chess {
        fen.parse::<Fen>()
            .unwrap()
            .into_position(CastlingMode::Standard)
            .unwrap()
    }

    fn best_move(position: &Chess, depth: u32) -> Move {
        let limits = SearchLimits {
            depth: Some(depth),
            ..SearchLimits::default()
        };
        let stop = AtomicBool::new(false);
        search(position, &limits, &stop, &mut |_| {}).unwrap()
    }

    fn long_algebraic(legal_move: Move) -> String {
        legal_move.to_uci(CastlingMode::Standard).to_string()
    }

    #[test]
    fn finds_mate_in_one() {
        let position = position("6k1/5ppp/8/8/8/8/8/R6K w - - 0 1");
        assert_eq!(long_algebraic(best_move(&position, 3)), "a1a8");
    }

    #[test]
    fn grabs_a_hanging_queen() {
        let position = position("4k3/8/8/8/3q4/8/8/3RK3 w - - 0 1");
        assert_eq!(long_algebraic(best_move(&position, 2)), "d1d4");
    }

    #[test]
    fn returns_a_legal_move_from_startpos() {
        let chosen = best_move(&Chess::default(), 4);
        assert!(Chess::default().legal_moves().contains(&chosen));
    }
}
