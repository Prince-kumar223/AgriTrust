#![no_std]

//! AgriTrust escrow contract — scaffolding only.
//!
//! This crate currently defines the contract type that will be deployed to
//! the Soroban network. Feature logic (listing -> offer -> escrow ->
//! delivery -> release) will be added in a later phase.

use soroban_sdk::{contract, contractimpl};

/// The AgriTrust escrow contract.
#[contract]
pub struct AgriTrustContract;

#[contractimpl]
impl AgriTrustContract {}
