pub mod searching;
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

