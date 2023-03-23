#![feature(let_chains)]
#![feature(iter_intersperse)]

//! Data and feedback directed automated test generator for JavaScript libraries.

pub mod code_gen;
pub mod consts;
pub mod decisions;
pub mod errors;
pub mod functions;
pub mod legacy;
pub mod mined_seed_reps;
pub mod module_reps;
pub mod testgen;
pub mod tests;

#[macro_use]
extern crate rand_derive;
