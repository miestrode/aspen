use std::sync::atomic::AtomicBool;

use shakmaty::{Chess, Move};

use crate::{GoParams, UciOptions};

pub trait Engine: Send {
    type Options: UciOptions;

    const NAME: &'static str;
    const AUTHOR: &'static str;

    fn options(&mut self) -> &mut Self::Options;

    fn new_game(&mut self) {}

    fn go(&mut self, position: &Chess, params: &GoParams, stop: &AtomicBool) -> Option<Move>;
}
