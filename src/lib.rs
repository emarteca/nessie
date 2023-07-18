#![feature(let_chains)]
#![feature(iter_intersperse)]
#![allow(deprecated)]

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

#[derive(Debug, Eq, PartialEq)]
pub enum TestGenMode {
    /// Current head of the current -- most up-to-date version (the default option)
    Head,
    /// Original `nessie` (from the ICSE 2022 paper), with some QOL fixes
    OGNessie,
    /// OGNessie with the addition of tracking primitive arg types
    TrackPrimitives,
    /// TrackPrimitives with the discovery and testgen phases merged
    MergeDiscGen,
    /// MergeDiscGen with the ability to chain methods
    ChainedMethods,
}

/// Autocast from strings to TestGenMode
impl std::str::FromStr for TestGenMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Head" => Ok(Self::Head),
            "OGNessie" => Ok(Self::OGNessie),
            "TrackPrimitives" => Ok(Self::TrackPrimitives),
            "MergeDiscGen" => Ok(Self::MergeDiscGen),
            "ChainedMethods" => Ok(Self::ChainedMethods),
            _ => Err(()),
        }
    }
}

impl TestGenMode {
    /// Short form label for the type of the testgen mode
    pub fn label(&self) -> String {
        match self {
            Self::Head => "Head",
            Self::OGNessie => "OGNessie",
            Self::TrackPrimitives => "TrackPrimitives",
            Self::MergeDiscGen => "MergeDiscGen",
            Self::ChainedMethods => "ChainedMethods",
        }
        .to_string()
    }

    /// Does this test generation mode include a separate API discovery phase?
    /// They all start with discovery!
    pub fn has_discovery(&self) -> bool {
        // match self {
        //     Self::OGNessie | Self::TrackPrimitives => true,
        //     _ => false,
        // }
        true
    }

    /// Do we support constructors?
    /// Only OGNessie does not
    pub fn supports_constructors(&self) -> bool {
        !matches!(self, Self::OGNessie)
    }

    /// Does this test gen mode generate chained method calls on the return values
    /// of previous function calls?
    pub fn chains_methods_on_retvals(&self) -> bool {
        matches!(self, Self::ChainedMethods | Self::Head)
    }

    /// Does this test gen mode discover new API signatures during the test generation?
    /// For now, this is just the opposite of `has_discovery`; but let's keep it a
    /// separate method in case this changes.
    pub fn discovers_during_testgen(&self) -> bool {
        !matches!(self, Self::OGNessie | Self::TrackPrimitives)
    }

    ///  Does this test gen mode track the types of primitive arguments?
    pub fn tracks_prim_types(&self) -> bool {
        !matches!(self, Self::OGNessie)
    }

    /// Check if this testgen mode generates calls for the function of specified name?
    pub fn will_gen_call_for(&self, fct_name: &str) -> bool {
        match self {
            // OGNessie doesn't generate calls to no-name functions (like modules that need to be immediately invoked)
            Self::OGNessie => !fct_name.is_empty(),
            _ => true,
        }
    }
}
