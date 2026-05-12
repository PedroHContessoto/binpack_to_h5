//! Stockfish-style entry filtering.
//!
//! Matches the official `nnue-pytorch` `SparseBatchProvider` default
//! (`filtered=True`):
//!
//! - **Skip captures** (~12-20% of positions): the eval at a capture is
//!   the result of quiescence search down to a quiet position; training the
//!   static eval to predict QS output teaches it search's job.
//! - **Skip in-check** (~5-10%): the side-to-move is forced to respond,
//!   so the static eval is meaningless without a check-extension search.
//!
//! Combined: ~25-35% of raw entries dropped. Including these positions in
//! training degrades model quality significantly.
//!
//! Reference: <https://github.com/official-stockfish/nnue-pytorch>
//! (`data_loader/cpp/training_data_loader.cpp` — `make_skip_predicate`)

use sfbinpack::chess::piece::Piece;
use sfbinpack::chess::position::Position;
use sfbinpack::chess::r#move::{Move, MoveType};

/// Returns true if the move is a capture (normal capture, en passant).
/// Castling is intentionally NOT a capture even though sfbinpack encodes it
/// as `king-to-rook-square`.
#[inline(always)]
pub fn is_capture(pos: &Position, mv: &Move) -> bool {
    if mv.mtype() == MoveType::Castle {
        return false;
    }
    if mv.mtype() == MoveType::EnPassant {
        return true;
    }
    pos.piece_at(mv.to()) != Piece::NONE
}

/// True when a `TrainingDataEntry` has obviously invalid Move/Position fields
/// (square index out of range). Some sfbinpack 0.6.x versions occasionally
/// deserialize garbage from certain binpacks; we skip those entries silently
/// rather than crashing the whole pipeline.
#[inline(always)]
pub fn is_malformed_entry(entry: &sfbinpack::TrainingDataEntry) -> bool {
    let to_idx = entry.mv.to().index();
    let from_idx = entry.mv.from().index();
    to_idx >= 64 || from_idx >= 64
}

/// True if the entry should be excluded from training (capture or in-check).
/// Caller is responsible for handling malformed entries before this is called.
#[inline(always)]
pub fn should_skip_entry(entry: &sfbinpack::TrainingDataEntry) -> bool {
    if is_capture(&entry.pos, &entry.mv) {
        return true;
    }
    if entry.pos.is_checked(entry.pos.side_to_move()) {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use sfbinpack::chess::coords::Square;

    fn mk_move(from: u32, to: u32, mt: MoveType) -> Move {
        Move::new(Square::new(from), Square::new(to), mt, Piece::none())
    }

    #[test]
    fn castle_not_a_capture() {
        // Standard starting position has no clear path for castle, but the
        // is_capture check shouldn't depend on legality; only on move type.
        let pos = Position::from_fen(
            "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1",
        )
        .unwrap();
        let castle = mk_move(4, 7, MoveType::Castle); // E1 -> H1 (KQ-side encoding)
        assert!(!is_capture(&pos, &castle));
    }

    #[test]
    fn en_passant_is_capture() {
        let pos = Position::from_fen(
            "rnbqkbnr/pp1ppppp/8/2pP4/8/8/PPP1PPPP/RNBQKBNR w KQkq c6 0 3",
        )
        .unwrap();
        // d5 -> c6 en passant (35 -> 42)
        let ep = mk_move(35, 42, MoveType::EnPassant);
        assert!(is_capture(&pos, &ep));
    }

    #[test]
    fn quiet_pawn_push_not_a_capture() {
        let pos = Position::from_fen(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        )
        .unwrap();
        let push = mk_move(12, 28, MoveType::Normal); // e2 -> e4
        assert!(!is_capture(&pos, &push));
    }

    #[test]
    fn normal_capture_detected() {
        // White knight on f3 captures pawn on g5 (Bxg5 not possible; use Nxg5
        // after a contrived setup).
        let pos = Position::from_fen(
            "rnbqkb1r/ppp1pppp/5n2/3P2p1/8/5N2/PPP1PPPP/RNBQKB1R w KQkq - 0 4",
        )
        .unwrap();
        // f3 (sq 21) -> g5 (sq 38), capture pawn on g5
        let capture = mk_move(21, 38, MoveType::Normal);
        assert!(is_capture(&pos, &capture));
    }

    #[test]
    fn malformed_squares_caught() {
        // Construct an entry with an obviously invalid `from` square.
        // Square::new(64) is the convention for "no square" / invalid in some
        // sfbinpack paths; certain corrupt entries deserialize to even higher.
        let pos = Position::from_fen(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        )
        .unwrap();
        let bad = sfbinpack::TrainingDataEntry {
            pos,
            mv: mk_move(120, 12, MoveType::Normal),
            score: 0,
            ply: 0,
            result: 0,
        };
        assert!(is_malformed_entry(&bad));
    }
}
