use crate::generated::chess::{*};

impl Position {
    pub fn from_grid(col : i32, row : i32) -> Position {
       let mut pos = Position::new();
       pos.row = row;
       pos.col = col;
       pos 
    }

    pub fn out_of_bounds(col : i32, row : i32) -> bool {
        col > 7 || col < 0 || row > 7 || row < 0
    }

    pub fn move_to(&self, col_inc : i32, row_inc : i32) -> Position{
        Position::from_grid(self.col + col_inc, self.row + row_inc)
    }
}