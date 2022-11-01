#![feature(let_chains)]

//! Data and feedback directed automated test generator for JavaScript libraries.

pub mod code_gen;
pub mod consts;
pub mod decisions;
pub mod discovery;
pub mod errors;
pub mod functions;
pub mod mined_seed_reps;
pub mod module_reps;
pub mod testgen;
pub mod tests;

#[macro_use]
#[allow(deprecated)]
extern crate rand_derive;
