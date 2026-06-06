use std::sync::atomic::AtomicBool;

use aspen_uci::{Engine, GoParams, UciOptions, run};
use shakmaty::{Chess, Move, Position};

#[derive(Debug, Default, UciOptions)]
struct FirstMoveOptions {
    #[uci(name = "Hash", min = 1, max = 1024, default = 16)]
    hash: i64,
}

#[derive(Debug, Default)]
struct FirstMoveEngine {
    options: FirstMoveOptions,
}

impl Engine for FirstMoveEngine {
    type Options = FirstMoveOptions;

    const NAME: &'static str = "aspen";
    const AUTHOR: &'static str = "aspen authors";

    fn options(&mut self) -> &mut Self::Options {
        &mut self.options
    }

    fn go(&mut self, position: &Chess, _params: &GoParams, _stop: &AtomicBool) -> Option<Move> {
        position.legal_moves().into_iter().next()
    }
}

fn main() -> std::io::Result<()> {
    run(FirstMoveEngine::default())
}
