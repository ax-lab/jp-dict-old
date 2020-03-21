#[macro_use]
extern crate lazy_static;

extern crate db;
pub use db::*;

mod data;
pub use data::get_db;
