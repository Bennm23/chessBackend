pub mod searching;
pub mod final_search;
pub mod search_test;
pub mod consts;
pub mod debug;

pub mod tables;
pub mod evaluation;

pub mod prelude {
    // easier exporting
    pub use super::evaluation;
    pub use super::tables;
    pub use super::debug;
    pub use super::consts;
    pub use super::searching;
}

#[allow(unused)]
macro_rules! print_at_ply {
    ($indent:expr, $fmt:expr, $($args:tt)*) => {
        {
            // Create a string of spaces
            let spaces = "  ".repeat($indent as usize);
            // Format the message with the provided arguments
            let message = format!($fmt, $($args)*);
            // Print the indented message
            println!("{}{}", spaces, message);
        }
    };
    // Case 2: No additional arguments
    ($indent:expr, $fmt:expr) => {
        {
            let spaces = " ".repeat($indent as usize);
            println!("{}{}", spaces, $fmt);
        }
    };
}
