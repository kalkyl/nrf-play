#![no_std]
#![cfg_attr(test, no_main)]

use nrf_play as _; // memory layout + panic handler

#[defmt_test::tests]
mod tests {}
