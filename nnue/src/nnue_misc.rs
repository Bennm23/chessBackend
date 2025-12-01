use pleco::{Board, Player};

use crate::{constants::LAYER_STACKS, nnue_utils::{format_cp_aligned_dot, to_cp}};

pub struct EvalTrace {
    pub selected_bucket: usize,
    pub side_to_move: Player,
    pub psqt: [i32; LAYER_STACKS],
    pub positional: [i32; LAYER_STACKS],
}
impl EvalTrace {
    pub fn new() -> Self {
        Self {
            selected_bucket: 0,
            side_to_move: Player::White,
            psqt: [0; LAYER_STACKS],
            positional: [0; LAYER_STACKS],
        }
    }
    pub fn print(&self, board: &Board) {
        println!("EvalTrace");
        println!("NNUE Network Contributions ({} to move)", self.side_to_move);

        let spacing = 13;

        let linspace: &str = &"-".repeat(spacing);

        println!("+{}+{}+{}+{}+", linspace, linspace, linspace, linspace);
        println!(
            "|{:^spacing$}|{:^spacing$}|{:^spacing$}|{:^spacing$}|",
            "Bucket", "Material", "Positional", "Total"
        );
        println!(
            "|{:^spacing$}|{:^spacing$}|{:^spacing$}|{:^spacing$}|",
            "", "(PSQT)", "(Layers)", ""
        );
        println!("+{}+{}+{}+{}+", linspace, linspace, linspace, linspace);

        for bucket in 0..LAYER_STACKS {
            let total = self.psqt[bucket] + self.positional[bucket];
            println!(
                "|{:^spacing$}|{:^spacing$}|{:^spacing$}|{:^spacing$}|{}",
                bucket,
                format_cp_aligned_dot(self.psqt[bucket], board),
                format_cp_aligned_dot(self.positional[bucket], board),
                format_cp_aligned_dot(total, board),
                if bucket == self.selected_bucket {
                    " <-- selected"
                } else {
                    ""
                },
            );
        }
        println!("+{}+{}+{}+{}+", linspace, linspace, linspace, linspace);

        let mut nnue_eval = self.psqt[self.selected_bucket] + self.positional[self.selected_bucket];
        if board.turn() == Player::Black {
            nnue_eval = -nnue_eval;
        }
        println!("NNUE Evaluation            {} (white side)", 0.01 * to_cp(nnue_eval, &board));
    }
}
