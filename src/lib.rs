#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod bit_set;
mod default_vec;
mod flag_vec;

pub use bit_set::BitSet;
pub use default_vec::{ConstDefault, DefaultVec};
pub use flag_vec::{DynamicFlagVec, FlagLength, FlagVec, StaticFlagVec};
