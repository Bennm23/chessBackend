use std::time::{Duration, Instant};

use pleco::{
    BitMove, Board, PieceType, Player, ScoringMove,
    core::{
        GenTypes,
        mono_traits::{BlackType, PlayerTrait, WhiteType},
        score::{DRAW, INFINITE, MATE, NEG_INFINITE},
    },
    tools::{
        PreFetchable,
        tt::{Entry, NodeBound, TranspositionTable},
    },
};

use crate::{
    consts::MVV_LVA,
    debug::NoTrace,
    evaluation::{eval_board_ai, trace_eval},
};

use super::{
    consts::{EvalVal, MyVal, QUEEN_VALUE},
    debug::{SearchDebugger, Trace, Tracing},
    tables::{material::Material, pawn_table::PawnTable},
};

const MATE_V: MyVal = MATE as MyVal;
const DRAW_V: MyVal = DRAW as MyVal;
const NEG_INF_V: MyVal = NEG_INFINITE as MyVal;
const INF_V: MyVal = INFINITE as MyVal;

const NULL_BIT_MOVE: BitMove = BitMove::null();

// const TT_ENTRIES: usize = 170_000; // Enough to use 16 MB of memory
const TT_ENTRIES: usize = 500_000;
pub const MAX_PLY: usize = 31;
const NUM_SQUARES: usize = 64;

// Mild LMR parameters.
const LMR_MIN_DEPTH: i8 = 3;
const LMR_MIN_MOVE_INDEX: i32 = 6;

// Null-move parameters.
const NULL_MOVE_MIN_DEPTH: i8 = 3;
const NULL_MOVE_REDUCTION_BASE: i8 = 2;

// Futility parameters (very mild, only on quiet nodes, never in check).
const FUTILITY_MAX_DEPTH: i8 = 2; // only at depth 1..2
const FUTILITY_BASE_MARGIN: MyVal = 100; // ~1 pawn per depth unit

// Searcher with TT, history, killers.
pub struct MySearcher<T: Tracing<SearchDebugger>> {
    pawn_table: PawnTable,
    material: Material,
    start_time: Instant,
    time_limit_ms: Option<u128>,

    tracer: T,
    nodes_explored: i64,

    // PV moves (for root / debug; not a full PV table)
    pv_moves: [ScoringMove; MAX_PLY],

    // Killer moves: two per ply
    killer_moves: [[BitMove; 2]; MAX_PLY],
    // History heuristic: [side][from][to]
    history: [[[i32; NUM_SQUARES]; NUM_SQUARES]; 2],

    // Transposition table
    tt: TranspositionTable,

    // Last root best move (for aspiration + PV ordering)
    last_root_move: BitMove,
}

pub const NULL_SCORE: ScoringMove = ScoringMove::null();

// Public API (unchanged)
pub fn search_to_depth_and_time(board: &mut Board, ply: u8, time: Option<u128>) -> BitMove {
    let mut searcher = MySearcher::new(NoTrace::new(), time);
    searcher.find_best_move(board, ply)
}

pub fn start_search(board: &mut Board) -> BitMove {
    let mut searcher = MySearcher::new(Trace::new(), Some(1000));
    searcher.find_best_move(board, MAX_PLY as u8)
}

pub fn eval_search(board: &mut Board) -> f64 {
    let mut searcher = MySearcher::new(NoTrace::new(), Some(1000));
    searcher.search_eval(board, MAX_PLY as u8)
}

impl<T: Tracing<SearchDebugger>> MySearcher<T> {
    pub fn new(tracer: T, time_limit: Option<u128>) -> Self {
        Self {
            pawn_table: PawnTable::new(),
            material: Material::new(),
            start_time: Instant::now(),
            time_limit_ms: time_limit,

            tracer,
            nodes_explored: 0,
            pv_moves: [NULL_SCORE; MAX_PLY],

            killer_moves: [[NULL_BIT_MOVE; 2]; MAX_PLY],
            history: [[[0; NUM_SQUARES]; NUM_SQUARES]; 2],

            tt: TranspositionTable::new_num_entries(TT_ENTRIES),
            last_root_move: NULL_BIT_MOVE,
        }
    }

    pub fn trace(tracer: T) -> Self {
        Self {
            pawn_table: PawnTable::new(),
            material: Material::new(),
            start_time: Instant::now(),
            time_limit_ms: None,

            tracer,
            nodes_explored: 0,
            pv_moves: [NULL_SCORE; MAX_PLY],

            killer_moves: [[NULL_BIT_MOVE; 2]; MAX_PLY],
            history: [[[0; NUM_SQUARES]; NUM_SQUARES]; 2],

            tt: TranspositionTable::new_num_entries(TT_ENTRIES),
            last_root_move: NULL_BIT_MOVE,
        }
    }

    pub fn eval(&mut self, board: &Board) -> MyVal {
        let pawns = &mut self.pawn_table;
        let material = &mut self.material;
        let res = eval_board_ai(board, pawns, material);

        if res > MyVal::MAX as EvalVal || res < MyVal::MIN as EvalVal {
            println!("ERROR: eval overflow for i16");
        }
        res as MyVal
    }

    #[inline(always)]
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    #[inline(always)]
    pub fn time_up(&self) -> bool {
        if let Some(limit) = self.time_limit_ms {
            self.start_time.elapsed().as_millis() > limit
        } else {
            false
        }
    }

    /// White-centric eval in pawns (using search).
    pub fn search_eval(&mut self, board: &mut Board, max_ply: u8) -> f64 {
        let best_move = self.perform_search(board, max_ply);
        let score_cp = best_move.score as f64;
        if board.turn() == Player::Black {
            -score_cp / 100.0
        } else {
            score_cp / 100.0
        }
    }

    /// Top-level search: iterative deepening + aspiration windows.
    pub fn perform_search(&mut self, board: &mut Board, max_ply: u8) -> ScoringMove {
        self.start_time = Instant::now();
        self.nodes_explored = 0;
        self.pv_moves = [NULL_SCORE; MAX_PLY];
        self.killer_moves = [[NULL_BIT_MOVE; 2]; MAX_PLY];
        self.history = [[[0; NUM_SQUARES]; NUM_SQUARES]; 2];
        self.tt.new_search();
        self.last_root_move = NULL_BIT_MOVE;

        let mut alpha: MyVal;
        let mut beta: MyVal;

        let mut best_move: ScoringMove = ScoringMove::blank(0);
        let mut score: MyVal = 0;
        let mut reached_depth: u8 = 1;

        'iterative: for depth in 1..=max_ply {
            if self.time_up() {
                break 'iterative;
            }

            let use_aspiration = depth >= 3;
            let mut window: MyVal = 30; // ~0.30 pawns

            if use_aspiration {
                alpha = (score - window).max(NEG_INF_V);
                beta = (score + window).min(INF_V);
            } else {
                alpha = NEG_INF_V;
                beta = INF_V;
            }

            let mut fail_count = 0;
            let root_pv_move = self.last_root_move;

            'aspiration: loop {
                let res = self.alpha_beta(
                    board,
                    alpha,
                    beta,
                    depth as i8,
                    0,
                    false,          // allow_null = false at root
                    root_pv_move,
                );

                if self.time_up() {
                    if self.tracer.trace().is_some() {
                        println!("Out of time, exiting at depth = {depth}");
                    }
                    break 'iterative;
                }

                score = res.score;
                reached_depth = depth;

                if !res.bit_move.is_null() {
                    best_move = res;
                    self.last_root_move = res.bit_move;
                }

                if best_move.score >= MATE_V - max_ply as MyVal {
                    println!("Mate found at depth = {depth}");
                    break 'iterative;
                }

                if !use_aspiration {
                    break 'aspiration;
                }

                // Fail-low
                if score <= alpha {
                    fail_count += 1;
                    if fail_count >= 3 {
                        alpha = NEG_INF_V;
                        beta = INF_V;
                    } else {
                        window = window.saturating_mul(2).saturating_add(10);
                        alpha = (score - window).max(NEG_INF_V);
                    }
                    continue 'aspiration;
                }

                // Fail-high
                if score >= beta {
                    fail_count += 1;
                    if fail_count >= 3 {
                        alpha = NEG_INF_V;
                        beta = INF_V;
                    } else {
                        window = window.saturating_mul(2).saturating_add(10);
                        beta = (score + window).min(INF_V);
                    }
                    continue 'aspiration;
                }

                // Within window
                break 'aspiration;
            }

            if let Some(dbg) = self.tracer.trace() {
                self.pv_moves[0] = best_move;
                let pv_line: Vec<BitMove> = self
                    .pv_moves
                    .iter()
                    .filter(|p| !p.bit_move.is_null())
                    .map(|s| s.bit_move)
                    .collect();

                dbg.add_depth(
                    alpha,
                    beta,
                    self.nodes_explored,
                    best_move.bit_move,
                    best_move.score,
                    pv_line,
                );
                self.nodes_explored = 0;
            }
        }

        if let Some(dbg) = self.tracer.trace() {
            dbg.add_duration(self.start_time.elapsed());
            println!("{dbg}");

            let eval = trace_eval(board);
            println!("TT Percent = {}", self.tt.hash_percent());
            println!("Raw Eval Before Move = {eval}");
            println!("AB Eval = {}", best_move.score);
            if best_move.score >= MATE_V - max_ply as MyVal {
                println!("Mate Found At Depth = {reached_depth}");
            }
        }
        // println!("TT Percent = {}", self.tt.hash_percent());
        // println!("REACHED DEPTH {reached_depth}");

        best_move
    }

    pub fn find_best_move(&mut self, board: &mut Board, max_ply: u8) -> BitMove {
        let res = self.perform_search(board, max_ply);
        res.bit_move
    }

    /// Core alpha-beta + PVS + move ordering (TT + history + killers) with:
    /// - mild LMR
    /// - null-move pruning
    /// - mild forward futility pruning
    fn alpha_beta(
        &mut self,
        board: &mut Board,
        mut alpha: MyVal,
        mut beta: MyVal,
        mut depth: i8,
        ply: u8,
        allow_null: bool,
        root_pv_move: BitMove,
    ) -> ScoringMove {
        if self.time_up() {
            // Fail-soft: current alpha as best-known.
            return ScoringMove::new_score(NULL_BIT_MOVE, alpha);
        }

        self.nodes_explored += 1;

        if (ply as usize) >= MAX_PLY - 1 {
            depth = 0;
        }

        // 50-move rule and 3-fold repetition: terminal draw.
        if board.fifty_move_rule() || board.threefold_repetition() {
            return ScoringMove::blank(DRAW_V);
        }

        let in_check = board.in_check();

        // Mate bound: clamp alpha/beta to legal mate score range.
        let mate_bound = MATE_V - ply as MyVal;
        if alpha < -mate_bound {
            alpha = -mate_bound;
        }
        if beta > mate_bound {
            beta = mate_bound;
        }
        if alpha >= beta {
            return ScoringMove::blank(mate_in(ply));
        }

        if depth <= 0 {
            let q = self.quiescence_search(board, alpha, beta, ply, 0);
            return ScoringMove::blank(q);
        }

        let mut static_eval = self.eval(board);

        // Generate moves early so we can detect checkmate/stalemate; we may still cut off via null move.
        let mut moves = board.generate_moves();

        if moves.is_empty() {
            if in_check {
                return ScoringMove::blank(mated_in(ply));
            } else {
                return ScoringMove::blank(DRAW_V);
            }
        }

        let alpha_orig = alpha;
        let zobrist = board.zobrist();

        // ---- TT PROBE BLOCK ----
        let mut tt_move = NULL_BIT_MOVE;
        {
            let (hit, entry): (bool, &mut Entry) = self.tt.probe(zobrist);

            if hit {
                tt_move = entry.best_move;
                let tt_val = entry.score;
                let tt_depth = entry.depth as i8;
                let tt_bound = entry.node_type();

                if let Some(cut_score) =
                    tt_maybe_cutoff(tt_val, tt_depth, depth, alpha, beta, tt_bound)
                {
                    return ScoringMove::new_score(entry.best_move, cut_score);
                }

                if tt_can_improve_static(tt_val, static_eval, tt_bound) {
                    static_eval = tt_val;
                }
            }
        }
        // ---- end TT block ----

        // -------- Mild forward futility pruning --------
        //
        // Only when:
        // - not in check
        // - low depth (<= FUTILITY_MAX_DEPTH)
        // - eval is clearly below alpha even after adding a small margin
        // - not a known near-mate score
        if !in_check
            && depth <= FUTILITY_MAX_DEPTH
            && static_eval.abs() < MATE_V - 256
        {
            let margin = futility_margin(depth);
            if static_eval + margin <= alpha {
                // Go to qsearch instead of full tree; preserves tactics.
                let q = self.quiescence_search(board, alpha, beta, ply, 0);
                return ScoringMove::blank(q);
            }
        }
        // ------------------------------------------------

        // -------- Null-move pruning --------
        //
        // Conditions:
        // - allowed (not after another null)
        // - not in check
        // - depth reasonably high
        // - static_eval already good enough to fail high
        // - side to move has some non-pawn material
        if allow_null
            && depth >= NULL_MOVE_MIN_DEPTH
            && !in_check
            && static_eval >= beta
            && static_eval.abs() < MATE_V - 256
        {
            let npm = board.non_pawn_material(board.turn());
            if npm > 0 {
                let r = NULL_MOVE_REDUCTION_BASE + depth / 4; // small reduction scaling with depth
                unsafe {
                    board.apply_null_move();
                }
                let null_res = self.alpha_beta(
                    board,
                    -beta,
                    -beta + 1,
                    depth - 1 - r,
                    ply + 1,
                    false,      // do not allow nested null
                    NULL_BIT_MOVE,
                );
                unsafe {
                    board.undo_null_move();
                }

                let null_score = -null_res.score;
                if null_score >= beta {
                    return ScoringMove::blank(null_score);
                }
            }
        }
        // ------------------------------------------------

        let side_idx = match board.turn() {
            Player::White => 0,
            Player::Black => 1,
        };

        let killers = if (ply as usize) < MAX_PLY {
            self.killer_moves[ply as usize]
        } else {
            [NULL_BIT_MOVE; 2]
        };

        let root_hint = if ply == 0 { root_pv_move } else { NULL_BIT_MOVE };

        moves.sort_by_key(|mv| {
            score_move(
                board,
                *mv,
                ply,
                side_idx,
                root_hint,
                tt_move,
                &killers,
                &self.history,
            )
        });

        let mut best_move = NULL_BIT_MOVE;
        let mut best_score = NEG_INF_V;
        let mut legal_moves = 0;
        let mut first_move = true;

        for (idx, mv) in moves.into_iter().enumerate() {
            if self.time_up() {
                break;
            }

            let move_index = idx as i32 + 1;

            let is_capture_or_promo = board.is_capture_or_promotion(mv);
            let gives_check = board.gives_check(mv);

            self.tt.prefetch(board.key_after(mv));

            board.apply_move(mv);

            // Mild LMR on quiet, non-check, non-first moves.
            let mut new_depth = depth - 1;
            let mut reduced = false;
            if !in_check
                && !is_capture_or_promo
                && !gives_check
                && !first_move
                && depth >= LMR_MIN_DEPTH
                && move_index >= LMR_MIN_MOVE_INDEX
            {
                // reduce by 1 ply
                new_depth -= 1;
                reduced = true;
            }

            let mut score: MyVal;

            if first_move {
                // Full-window for first (PV) move
                score = -self
                    .alpha_beta(
                        board,
                        -beta,
                        -alpha,
                        depth - 1,
                        ply + 1,
                        true,           // allow null below PV
                        NULL_BIT_MOVE,
                    )
                    .score;
                first_move = false;
            } else {
                // PVS + possible reduced-depth null-window search
                score = -self
                    .alpha_beta(
                        board,
                        -alpha - 1,
                        -alpha,
                        new_depth,
                        ply + 1,
                        true,           // allow null below
                        NULL_BIT_MOVE,
                    )
                    .score;

                // If reduced and better than alpha, re-search with full depth (still narrow window).
                if reduced && score > alpha {
                    score = -self
                        .alpha_beta(
                            board,
                            -alpha - 1,
                            -alpha,
                            depth - 1,
                            ply + 1,
                            true,
                            NULL_BIT_MOVE,
                        )
                        .score;
                }

                // If still potentially raising alpha into PV window, full re-search.
                if score > alpha && score < beta {
                    score = -self
                        .alpha_beta(
                            board,
                            -beta,
                            -alpha,
                            depth - 1,
                            ply + 1,
                            true,
                            NULL_BIT_MOVE,
                        )
                        .score;
                }
            }

            board.undo_move();
            legal_moves += 1;

            if score > best_score {
                best_score = score;
                best_move = mv;
            }

            if score > alpha {
                alpha = score;

                if alpha >= beta {
                    // Beta cutoff: update killers/history for quiet moves
                    if !is_capture_or_promo && !gives_check {
                        self.store_killer(ply, mv);
                        self.update_history(side_idx, mv, depth);
                    }

                    if !self.time_up() {
                        let age = self.tt.time_age();
                        let (_, entry): (bool, &mut Entry) = self.tt.probe(zobrist);
                        entry.place(
                            zobrist,
                            best_move,
                            best_score,
                            static_eval,
                            depth as i16,
                            NodeBound::LowerBound,
                            age,
                        );
                    }

                    return ScoringMove::new_score(best_move, best_score);
                }
            }
        }

        if legal_moves == 0 {
            if in_check {
                return ScoringMove::blank(mated_in(ply));
            } else {
                return ScoringMove::blank(DRAW_V);
            }
        }

        // Classify node result for TT for non-cutoff nodes.
        let tt_flag = if best_score <= alpha_orig {
            NodeBound::UpperBound // fail-low
        } else {
            NodeBound::Exact // inside window, not fail-high (we handled those earlier)
        };

        if matches!(tt_flag, NodeBound::Exact)
            && !best_move.is_null()
            && (ply as usize) < MAX_PLY
        {
            self.pv_moves[ply as usize] = ScoringMove::new_score(best_move, best_score);
        }

        if !self.time_up() {
            let age = self.tt.time_age();
            let (_, entry): (bool, &mut Entry) = self.tt.probe(zobrist);
            entry.place(
                zobrist,
                best_move,
                best_score,
                static_eval,
                depth as i16,
                tt_flag,
                age,
            );
        }

        ScoringMove::new_score(best_move, best_score)
    }

    /// Quiescence search: stand-pat, captures, evasions if in check.
    fn quiescence_search(
        &mut self,
        board: &mut Board,
        mut alpha: MyVal,
        beta: MyVal,
        ply: u8,
        depth: i8,
    ) -> MyVal {
        if self.time_up() {
            return alpha;
        }

        if board.fifty_move_rule() || board.threefold_repetition() {
            return DRAW_V;
        }

        let in_check = board.in_check();
        let static_eval = self.eval(board);

        // Depth cap: allow fall-through when in check to search evasions;
        // only stand-pat when not in check.
        if depth <= -5 && !in_check {
            return static_eval;
        }

        if !in_check {
            // Stand-pat
            if static_eval >= beta {
                return static_eval;
            }

            if static_eval > alpha {
                alpha = static_eval;
            }

            // Simple delta pruning: if even max capture gain can't raise static_eval to alpha, prune.
            let mut max_gain = QUEEN_VALUE as i32;
            if let Some(last) = board.last_move() {
                if last.is_promo() {
                    max_gain += QUEEN_VALUE as i32;
                }
            }

            if (static_eval as i32) + max_gain < alpha as i32 {
                return static_eval;
            }
        }

        // Generate moves: evasions if in check, otherwise captures.
        let mut moves = if in_check {
            board.generate_moves_of_type(GenTypes::Evasions)
        } else {
            board.generate_moves_of_type(GenTypes::Captures)
        };

        if moves.is_empty() {
            if in_check {
                return mated_in(ply);
            } else {
                return static_eval;
            }
        }

        // Order captures by MVV-LVA (using your table).
        if !in_check {
            moves.sort_by_key(|mv| {
                if mv.is_capture() {
                    get_capture_score(board, mv)
                } else if mv.is_promo() {
                    (mv.promo_piece() as MyVal).saturating_mul(-3)
                } else {
                    0
                }
            });
        }

        let mut best = if in_check { NEG_INF_V } else { static_eval };
        let next_depth = depth - 1;

        for mv in moves {
            if self.time_up() {
                return best.max(alpha);
            }

            self.tt.prefetch(board.key_after(mv));

            board.apply_move(mv);
            let score =
                -self.quiescence_search(board, -beta, -alpha, ply + 1, next_depth);
            board.undo_move();

            if score >= beta {
                return score;
            }

            if score > best {
                best = score;
                if best > alpha {
                    alpha = best;
                }
            }
        }

        best
    }

    fn store_killer(&mut self, ply: u8, mv: BitMove) {
        let p = ply as usize;
        if p >= MAX_PLY || mv.is_null() {
            return;
        }

        if self.killer_moves[p][0] != mv {
            self.killer_moves[p][1] = self.killer_moves[p][0];
            self.killer_moves[p][0] = mv;
        }
    }

    fn update_history(&mut self, side_idx: usize, mv: BitMove, depth: i8) {
        let from = mv.get_src_u8() as usize;
        let to = mv.get_dest_u8() as usize;
        if from >= NUM_SQUARES || to >= NUM_SQUARES {
            return;
        }

        let bonus = (depth as i32).max(1).pow(2);
        let entry = &mut self.history[side_idx][from][to];
        *entry = (*entry + bonus).clamp(-10_000, 10_000);
    }
}


#[inline(always)]
fn mate_in(ply: u8) -> MyVal {
    MATE_V - ply as MyVal
}

#[inline(always)]
fn mated_in(ply: u8) -> MyVal {
    -MATE_V + ply as MyVal
}

#[inline(always)]
fn futility_margin(depth: i8) -> MyVal {
    // Very conservative: ~1 pawn per depth unit, plus a small base
    FUTILITY_BASE_MARGIN * depth.max(1) as MyVal
}

/// TT: can we reuse this as an immediate cutoff or exact value?
#[inline(always)]
fn tt_maybe_cutoff(
    tt_value: MyVal,
    depth_in_table: i8,
    needed_depth: i8,
    alpha: MyVal,
    beta: MyVal,
    bound: NodeBound,
) -> Option<MyVal> {
    if depth_in_table < needed_depth {
        return None;
    }

    match bound {
        NodeBound::Exact => Some(tt_value),
        NodeBound::LowerBound => {
            if tt_value >= beta {
                Some(tt_value)
            } else {
                None
            }
        }
        NodeBound::UpperBound => {
            if tt_value <= alpha {
                Some(tt_value)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// TT: is this value a better static approximation?
#[inline(always)]
fn tt_can_improve_static(tt_value: MyVal, current_val: MyVal, bound: NodeBound) -> bool {
    match bound {
        NodeBound::LowerBound => tt_value > current_val,
        NodeBound::UpperBound => tt_value < current_val,
        NodeBound::Exact => true,
        _ => false,
    }
}

/// Move ordering score: lower is better (we sort by key ascending).
fn score_move(
    board: &Board,
    mv: BitMove,
    ply: u8,
    side_idx: usize,
    root_pv_move: BitMove,
    tt_move: BitMove,
    killers: &[BitMove; 2],
    history: &[[[i32; NUM_SQUARES]; NUM_SQUARES]; 2],
) -> i32 {
    // Strong negative = higher priority (we sort ascending).
    let mut score: i32 = 0;

    // Root PV move
    if ply == 0 && !root_pv_move.is_null() && mv == root_pv_move {
        return -1_000_000_000;
    }

    // TT move
    if !tt_move.is_null() && mv == tt_move {
        return -900_000_000;
    }

    let from = mv.get_src_u8() as usize;
    let to = mv.get_dest_u8() as usize;

    let is_capture = board.is_capture(mv);
    let is_promo = mv.is_promo();

    if is_capture {
        // Captures (including capture-promotions)
        let cap_score = get_capture_score(board, &mv) as i32; // 0..negative
        score -= 500_000 + cap_score * 100;

        if is_promo {
            // Capture-promotion is even more forcing.
            score -= 50_000;
        }
    } else if is_promo {
        // Quiet promotion (no capture)
        let promo_piece = mv.promo_piece();
        let promo_bonus = match promo_piece {
            PieceType::Q => 400_000,
            PieceType::R => 350_000,
            PieceType::B => 325_000,
            PieceType::N => 320_000,
            _ => 300_000,
        };
        score -= promo_bonus;
    } else {
        // Quiet non-promo moves: killers + history

        if mv == killers[0] {
            score -= 400_000;
        } else if mv == killers[1] {
            score -= 399_000;
        }

        if from < NUM_SQUARES && to < NUM_SQUARES {
            let hist = history[side_idx][from][to];
            score -= hist;
        }
    }

    score
}

fn get_capture_score(board: &Board, mv: &BitMove) -> MyVal {
    let attacker = board.piece_at_sq(mv.get_src());
    let dest = if mv.is_en_passant() {
        if board.turn() == Player::White {
            WhiteType::down(mv.get_dest())
        } else {
            BlackType::down(mv.get_dest())
        }
    } else {
        mv.get_dest()
    };
    let captured = board.piece_at_sq(dest);
    if (!attacker.type_of().is_real()) || (!captured.type_of().is_real()) {
        println!(
            "Error in get_capture_score: attacker = {:?}, captured = {:?}",
            attacker, captured
        );
        println!("Attack as usize = {}", attacker.type_of() as usize - 1);
        println!("Captured as usize = {}", captured.type_of() as usize - 1);
        panic!("THE ERROR");
    }

    MVV_LVA[attacker.type_of() as usize - 1][captured.type_of() as usize - 1]
}

#[cfg(test)]
mod tests {
    use pleco::{BitMove, Board};

    use crate::{debug::Trace, debug::Tracing, final_search::MySearcher};   
    fn ev_depth(fen: &str, depth: u8) -> BitMove {
        let mut board = Board::from_fen(fen).unwrap();
        let mut searcher = MySearcher::trace(Trace::new());
        searcher.find_best_move(&mut board, depth)
    }

    #[test]
    fn test_black_avoids_mate() {
        //This position, black has been blundering mate
        //1k1rr3/pp3p1Q/5q2/P7/4n1B1/1P1p3P/3P1PP1/1R3K1R w - - 2 25
        let fen = "1k1rr3/pp3p1Q/5q2/P7/4n1B1/1P1p3P/3P1PP1/1R3K1R w - - 2 25";
        let mv = ev_depth(fen, 5);
        println!("MV = {mv}");
        // assert!(mv.get_src_u8() == 55 && mv.get_dest_u8() == 28);
    }
}