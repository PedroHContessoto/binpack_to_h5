//! HalfP feature encoding and centipawn-to-storage normalization.
//!
//! HalfP encoding (768 features = 2 colors × 6 piece types × 64 squares):
//! ```text
//! feature_idx = color * 384 + piece_type * 64 + square
//!   color:       0=white, 1=black  (the piece color, board is white-POV)
//!   piece_type:  0=Pawn, 1=Knight, 2=Bishop, 3=Rook, 4=Queen, 5=King
//!   square:      0..63 (a1=0, h8=63)
//! ```

use sfbinpack::chess::coords::Square;
use sfbinpack::chess::piece::Piece;
use sfbinpack::chess::position::Position;

use crate::config::{EVAL_SCALE, MATE_BASE, MATE_DECAY, MATE_THRESHOLD_CP, TARGET_CLIP};

/// Fills `buf` with HalfP feature indices (white-POV) for every piece on the
/// board. Caller owns `buf`; we clear it first to allow reuse.
#[inline(always)]
pub fn encode_halfp(pos: &Position, buf: &mut Vec<i16>) {
    buf.clear();
    for sq in 0..64u32 {
        let p = pos.piece_at(Square::new(sq));
        if p == Piece::NONE {
            continue;
        }
        let kind = p.piece_type().ordinal() as i16;
        let color = p.color().ordinal() as i16;
        let idx = color * 384 + kind * 64 + sq as i16;
        buf.push(idx);
    }
}

/// Centipawn -> normalized storage value (white-POV).
///
/// Normal scores are scaled by `EVAL_SCALE` and clipped to `±TARGET_CLIP`.
/// Mate scores (|cp| > MATE_THRESHOLD_CP) get a separate encoding above the
/// clip range so they remain distinguishable from saturated normal evals.
#[inline(always)]
pub fn normalize_eval(score_cp: i32) -> f32 {
    let abs_cp = score_cp.unsigned_abs() as i32;
    let sign: f32 = if score_cp >= 0 { 1.0 } else { -1.0 };
    if abs_cp > MATE_THRESHOLD_CP {
        let ply_estimate = (((32000 - abs_cp) as f32 / 100.0).max(1.0).min(30.0)) as f32;
        sign * (MATE_BASE - ply_estimate * MATE_DECAY)
    } else {
        sign * (abs_cp as f32 / EVAL_SCALE).min(TARGET_CLIP)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startpos_has_32_pieces() {
        let pos = Position::from_fen(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        )
        .unwrap();
        let mut buf = Vec::new();
        encode_halfp(&pos, &mut buf);
        assert_eq!(buf.len(), 32);
    }

    #[test]
    fn feature_indices_in_range() {
        let pos = Position::from_fen(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        )
        .unwrap();
        let mut buf = Vec::new();
        encode_halfp(&pos, &mut buf);
        for &idx in &buf {
            assert!((0..768).contains(&(idx as i32)), "idx {idx} out of [0, 768)");
        }
    }

    #[test]
    fn empty_board_yields_no_features() {
        let pos = Position::from_fen("8/8/8/8/8/8/8/8 w - - 0 1").unwrap();
        let mut buf = vec![1, 2, 3]; // garbage to verify clear()
        encode_halfp(&pos, &mut buf);
        assert!(buf.is_empty());
    }

    #[test]
    fn normalize_eval_zero() {
        assert_eq!(normalize_eval(0), 0.0);
    }

    #[test]
    fn normalize_eval_clipped() {
        // 10000 cp / 400 = 25.0, clipped to 6.5
        assert_eq!(normalize_eval(10_000), 6.5);
        assert_eq!(normalize_eval(-10_000), -6.5);
    }

    #[test]
    fn normalize_eval_normal_range() {
        // 200 cp / 400 = 0.5
        assert!((normalize_eval(200) - 0.5).abs() < 1e-6);
        // -100 cp / 400 = -0.25
        assert!((normalize_eval(-100) - (-0.25)).abs() < 1e-6);
    }

    #[test]
    fn normalize_eval_mate_above_clip() {
        // Mate scores must exceed TARGET_CLIP to remain distinguishable.
        let mate_in_5 = normalize_eval(31_500);
        assert!(mate_in_5.abs() > TARGET_CLIP, "mate enc {mate_in_5} <= clip");
    }
}
