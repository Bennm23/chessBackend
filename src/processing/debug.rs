use std::{fmt::Display, time::Duration};

use pleco::{core::score::Score, BitMove, Player};

use super::consts::{MyVal, PAWN_EG, PAWN_MG};

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum EvalPasses {
    Pawn = 0,
    Knight = 1,
    Bishop = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
    Material = 6,
    Imbalance = 7,
    Mobility = 8,
    Threat = 9,
    PawnStructure = 10,
    Space = 11,
    Initiative = 12,
    Total = 13,
}

impl Default for EvalPasses {
    fn default() -> Self {
        Self::Total
    }
}

#[derive(Clone, Copy)]
pub struct Entry {
    white_score: Score,
    black_score: Score,
    ev_type:        EvalPasses,
}

impl Default for Entry {
    fn default() -> Self {
        Self {
             white_score: Score::ZERO,
             black_score: Score::ZERO,
             ev_type: Default::default() 
        }
    }
}

pub fn score_str(score: Score) -> String {
    format!("{:5.2} {:5.2}", score.mg() as f64 / PAWN_MG as f64, score.eg() as f64 / PAWN_EG as f64)
}

impl Entry {
    pub fn set_type(&mut self, player: Player, val: Score, eval: EvalPasses) {
        match player {
            Player::White => self.white_score = val,
            Player::Black => self.black_score = val,
        }
        self.ev_type = eval;
    }
}

impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.ev_type {
            EvalPasses::Material
            | EvalPasses::Imbalance
            // | EvalPasses::Initiative
            | EvalPasses::Total => write!(f, " ----  ---- |  ----  ----")?,
            _ => write!(f, "{} | {}", score_str(self.white_score), score_str(self.black_score))?,
        }
        write!(f, " | {}", score_str(self.white_score - self.black_score))

    }
}
pub trait Tracing<T> {
    fn trace(&mut self) -> Option<&mut T>;

    fn new() -> Self;
}

pub struct NoTrace<T> {
    _t: Option<T>,//Unused, always set to None
}

pub struct Trace<T> {
    t: T,
}

impl <T> Tracing<T> for NoTrace<T> {
    fn trace(&mut self) -> Option<&mut T> {
        None
    }

    fn new() -> Self {
        let t : Option<T> = None;
        NoTrace {
            _t: t
        }
    }
}

pub trait Debugger {
    fn new() -> Self;
}

impl <T: Debugger> Tracing<T> for Trace<T> {
    fn trace(&mut self) -> Option<&mut T> {
        Some(&mut self.t)
    }

    fn new() -> Self {
        Trace { t: T::new() }
    }
}


pub struct EvalDebugger {
    evals: [Entry; 14]
}
impl Debugger for EvalDebugger {
    fn new() -> Self {
        let e = Entry::default();
        Self {
            evals: [e, e, e, e, e, e, e, e, e, e, e, e, e, e],
        }
    }
}
impl EvalDebugger {

    pub fn set_eval(&mut self, eval: EvalPasses, player: Player, val: Score) {
        self.evals[eval as usize].set_type(player, val, eval);
    }
    pub fn set_two_eval(&mut self, eval: EvalPasses, white_score: Score, black_score: Score) {
        self.evals[eval as usize].set_type(Player::White, white_score, eval);
        self.evals[eval as usize].set_type(Player::Black, black_score, eval);
    }
    pub fn eval(&self, eval: EvalPasses) -> Entry {
        self.evals[eval as usize]
    }
}

impl Display for EvalDebugger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "     Term    |    White    |    Black    |    Total  ")?;
        writeln!(f, "             |   MG    EG  |   MG    EG  |   MG    EG")?;
        writeln!(f, " ------------+-------------+-------------+-----------")?;
        writeln!(f, "    Material | {}", self.eval(EvalPasses::Material))?;
        writeln!(f, "   Imbalance | {}", self.eval(EvalPasses::Imbalance))?;
        // writeln!(f, "  Initiative | {}", self.term(EvalPasses::Initiative))?;
        writeln!(f, "       Pawns | {}", self.eval(EvalPasses::Pawn))?;
        writeln!(f, "     Knights | {}", self.eval(EvalPasses::Knight))?;
        writeln!(f, "     Bishops | {}", self.eval(EvalPasses::Bishop))?;
        writeln!(f, "       Rooks | {}", self.eval(EvalPasses::Rook))?;
        writeln!(f, "      Queens | {}", self.eval(EvalPasses::Queen))?;
        writeln!(f, "    Mobility | {}", self.eval(EvalPasses::Mobility))?;
        writeln!(f, " King safety | {}", self.eval(EvalPasses::King))?;
        writeln!(f, "     Threats | {}", self.eval(EvalPasses::Threat))?;
        writeln!(f, " Pawn Struct | {}", self.eval(EvalPasses::PawnStructure))?;
        // writeln!(f, "       Space | {}", self.eval(EvalPasses::Space))?;
        writeln!(f, " ------------+-----------------+-----------------+----------------")?;
        writeln!(f, "       Total | {}", self.eval(EvalPasses::Total))
    }
}


pub struct SearchDebugger {

    search_records: Vec<DepthRecord>,
    search_duration: Duration,

}
impl Debugger for SearchDebugger {
    fn new() -> Self {
        Self {
            search_records: Vec::new(),
            search_duration: Duration::default(),
        }
    }
}
impl SearchDebugger {
    pub fn add_depth(
        &mut self,
        alpha: MyVal, beta: MyVal,
        nodes_explored: i64,
        best_move: BitMove, board_eval: MyVal,
        pv_moves: Vec<BitMove>,
    ) {
        self.search_records.push(DepthRecord {
            depth: self.search_records.len() + 1,
            alpha, beta,
            nodes_explored,
            best_move, board_eval,
            pv_moves,
        });
    }
    pub fn add_duration(&mut self, search_duration: Duration) {
        self.search_duration = search_duration;
    }
}
impl Display for SearchDebugger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, " {:5} | {:4} | {:4} |  {:6} |  {:5}  | {} ",
                "Depth", "Best", "Eval", "Alpha", "Beta", "Nodes Explored"
        )?;
        
        for node in &self.search_records {
            writeln!(f, "{node}")?;
        }
    
        writeln!(f, "-------+------+------+---------+---------+------------------")?;

        writeln!(f, "Search Took {} ms", self.search_duration.as_millis())
    }
}

pub struct DepthRecord {
    depth: usize,
    alpha: MyVal, beta: MyVal,
    nodes_explored: i64,
    best_move: BitMove, board_eval: MyVal,
    pv_moves: Vec<BitMove>,
}

impl Display for DepthRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "-------+------+------+---------+---------+------------------")?;
        writeln!(
            f, "{:>6} | {:4} | {:4} | {:6}  | {:6}  | {:<10} ",
            self.depth, self.best_move.stringify(), self.board_eval, self.alpha, self.beta, self.nodes_explored
        )?;

        write!(f, "PV Moves = [ ")?;

        for (i, entry) in self.pv_moves.iter().enumerate() {
            write!(f, "{entry} ")?;

            if i != self.pv_moves.len() - 1 {
                write!(f, ", ")?;
            }
        }

        writeln!(f, " ]")
    }
}