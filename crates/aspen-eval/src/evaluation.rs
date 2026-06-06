use shakmaty::{Chess, Color, Position};

use crate::tables::{endgame_score, game_phase, midgame_score};

const PHASE_LIMIT: i32 = 24;

pub fn evaluate(position: &Chess) -> i32 {
    let mut midgame = 0;
    let mut endgame = 0;
    let mut phase = 0;
    for (square, piece) in position.board().iter() {
        let perspective = white_relative(piece.color);
        midgame += perspective * midgame_score(piece, square);
        endgame += perspective * endgame_score(piece, square);
        phase += game_phase(piece.role);
    }
    let phase = phase.min(PHASE_LIMIT);
    let tapered = (midgame * phase + endgame * (PHASE_LIMIT - phase)) / PHASE_LIMIT;
    white_relative(position.turn()) * tapered
}

fn white_relative(color: Color) -> i32 {
    color.fold_wb(1, -1)
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

    #[test]
    fn startpos_is_balanced() {
        assert_eq!(evaluate(&Chess::default()), 0);
    }

    #[test]
    fn extra_queen_is_winning() {
        let with_queen = position("4k3/8/8/8/8/8/8/3QK3 w - - 0 1");
        assert!(evaluate(&with_queen) > 800);
    }

    #[test]
    fn side_to_move_flips_sign() {
        let white_to_move = position("4k3/8/8/8/8/8/8/3QK3 w - - 0 1");
        let black_to_move = position("4k3/8/8/8/8/8/8/3QK3 b - - 0 1");
        assert_eq!(evaluate(&white_to_move), -evaluate(&black_to_move));
    }
}
