#![no_std]

//! # AgriTrust Escrow Contract
//!
//! A [Soroban] smart contract that escrows payments for agricultural trades
//! between a **farmer** (seller) and a **buyer**.
//!
//! ## Lifecycle
//!
//! ```text
//! Pending ──(deposit)──► Funded ──(deliver)──► Delivered ──(receipt)──► Completed
//!    │                     │  │                       │
//!    └──(cancel)──► Cancelled   └────(dispute)───────► Disputed
//! ```
//!
//! The buyer locks the agreed price in the contract (`Funded`), the farmer
//! ships the crop and marks it `Delivered`, and only when the buyer confirms
//! receipt (`Completed`) is the payment automatically released to the farmer.
//! Either party can freeze the funds by raising a `Disputed` state. A trade
//! can only be cancelled while `Pending`, before any funds have moved.
//!
//! ## Soroban concepts used here (quick primer)
//!
//! - **Contracts are stateless.** All state lives in the environment's key /
//!   value storage and is keyed by a [`DataKey`]. Trades are stored in
//!   *persistent* storage (live until explicitly extended/removed), keyed by
//!   `trade_id`.
//! - **`require_auth()`** is how a contract verifies *who* authorized a call:
//!   calling `addr.require_auth()` panics the transaction unless the holder of
//!   `addr`'s key signed the current invocation. This is the canonical
//!   Soroban "caller check" (there is no `msg.sender` equivalent — see
//!   [CAP-44]).
//! - **Events** (`#[contractevent]`) are cheap, indexable logs emitted by the
//!   contract. Every state transition publishes one, so off-chain indexers /
//!   analytics can reconstruct trade history without reading storage.
//! - **Cross-contract calls** — the contract moves tokens by calling the
//!   token contract's `transfer` through [`soroban_sdk::token::TokenClient`].
//!
//! [Soroban]: https://soroban.stellar.org
//! [CAP-44]: https://github.com/stellar/stellar-protocol/blob/master/core/cap-0044.md

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token::TokenClient,
    Address, Env, String, Symbol,
};

/// Errors returned by this contract. Serialized on-chain so callers receive
/// typed failures instead of raw panics.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// `get_trade`/state-changing fn was called with an unknown `trade_id`.
    TradeNotFound = 1,
    /// A trade with this `trade_id` already exists.
    TradeAlreadyExists = 2,
    /// The caller is not one of the trade's parties (or the passed-in address
    /// is not the registered party for this action).
    UnauthorizedCaller = 3,
    /// The trade is not in the state required for this action
    /// (e.g. double-funding, or confirming out of order).
    InvalidState = 4,
    /// `price` must be strictly positive at trade creation.
    InvalidPrice = 5,
}

/// Lifecycle state of a trade. Stored on-chain inside [`Trade`].
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TradeState {
    /// Created, no funds have moved. Cancellable.
    Pending,
    /// The buyer's payment is locked in the contract.
    Funded,
    /// The farmer marked the crop as delivered.
    Delivered,
    /// The buyer confirmed receipt; payment was released to the farmer.
    Completed,
    /// A party flagged a dispute; funds stay locked.
    Disputed,
    /// Cancelled before funding; nothing was refunded because nothing moved.
    Cancelled,
}

/// A trade and its escrow record — the single unit of state in the contract.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trade {
    /// Unique on-chain identifier used as the persistent-storage key.
    pub trade_id: Symbol,
    /// Seller. Creates the trade and confirms delivery.
    pub farmer: Address,
    /// Buyer. Funds the escrow and confirms receipt.
    pub buyer: Address,
    /// Token contract whose units are escrowed (XLM or an asset on Stellar).
    /// Kept per-trade so different trades can use different assets.
    pub asset: Address,
    /// Human-readable description of the produce (e.g. "2t wheat, Grade A").
    pub crop_details: String,
    /// Agreed price in the smallest unit of `asset`. The exact amount that is
    /// locked on deposit and released on receipt.
    pub price: i128,
    /// Current lifecycle state (see [`TradeState`]).
    pub state: TradeState,
}

/// Persistent-storage keys. Using an enum (rather than raw symbols) keeps the
/// key space namespaced and extensible.
#[contracttype]
pub enum DataKey {
    /// The full `Trade` record, keyed by `trade_id`.
    Trade(Symbol),
}

// ---------------------------------------------------------------------------
// Events — published on every state transition for off-chain indexing.
// The `#[topic]` fields become event topics (indexable); the rest form the
// event data payload. Each struct's fixed topic is its snake_case name.
// ---------------------------------------------------------------------------

/// Published when a trade is created (`Pending`).
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TradeCreated {
    #[topic]
    pub trade_id: Symbol,
    pub farmer: Address,
    pub buyer: Address,
    pub crop_details: String,
    pub price: i128,
}

/// Published when the buyer locks payment in escrow (`Funded`).
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TradeFunded {
    #[topic]
    pub trade_id: Symbol,
    pub buyer: Address,
    pub amount: i128,
}

/// Published when the farmer marks the crop as delivered (`Delivered`).
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TradeDelivered {
    #[topic]
    pub trade_id: Symbol,
    pub farmer: Address,
}

/// Published when the buyer confirms receipt and payment is released
/// (`Completed`).
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TradeCompleted {
    #[topic]
    pub trade_id: Symbol,
    pub farmer: Address,
    pub buyer: Address,
    pub amount: i128,
}

/// Published when either party flags a dispute (`Disputed`).
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TradeDisputed {
    #[topic]
    pub trade_id: Symbol,
    pub caller: Address,
    pub reason: String,
}

/// Published when a `Pending` trade is cancelled (`Cancelled`).
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TradeCancelled {
    #[topic]
    pub trade_id: Symbol,
    pub caller: Address,
}

// ---------------------------------------------------------------------------
// Storage helpers
// ---------------------------------------------------------------------------

/// Returns `true` if a trade with `trade_id` exists in persistent storage.
fn has_trade(env: &Env, trade_id: &Symbol) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Trade(trade_id.clone()))
}

/// Persists `trade` under `DataKey::Trade(trade.trade_id)`.
fn store_trade(env: &Env, trade: &Trade) {
    env.storage()
        .persistent()
        .set(&DataKey::Trade(trade.trade_id.clone()), trade);
}

/// Loads the trade for `trade_id`, or returns [`Error::TradeNotFound`].
fn load_trade(env: &Env, trade_id: &Symbol) -> Result<Trade, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Trade(trade_id.clone()))
        .ok_or(Error::TradeNotFound)
}

/// The AgriTrust escrow contract.
#[contract]
pub struct AgriTrustContract;

#[contractimpl]
impl AgriTrustContract {
    /// Creates a new trade in the [`TradeState::Pending`] state.
    ///
    /// Only the farmer (the party selling the produce) may create a trade —
    /// their [`Address::require_auth`] is enforced. No funds move here.
    ///
    /// # Arguments
    /// * `trade_id` — unique on-chain id, also used as the storage key.
    ///   Reusing an existing id is rejected with [`Error::TradeAlreadyExists`].
    /// * `farmer` — seller address. Must be the caller.
    /// * `buyer` — buyer address that will fund the escrow later.
    /// * `asset` — token contract whose units are escrowed (XLM or any
    ///   Stellar asset). Stored per-trade so trades can use different assets.
    /// * `crop_details` — human-readable description of the produce.
    /// * `price` — agreed price in the smallest unit of `asset` (must be > 0).
    ///
    /// # Errors
    /// * [`Error::TradeAlreadyExists`] — `trade_id` is already in use.
    /// * [`Error::InvalidPrice`] — `price` is not strictly positive.
    pub fn create_trade(
        env: Env,
        trade_id: Symbol,
        farmer: Address,
        buyer: Address,
        asset: Address,
        crop_details: String,
        price: i128,
    ) -> Result<Trade, Error> {
        // Only the farmer may create a trade on their own behalf.
        farmer.require_auth();

        // `trade_id` is the persistent-storage key: a collision would silently
        // overwrite an existing trade, so reject it explicitly.
        if has_trade(&env, &trade_id) {
            return Err(Error::TradeAlreadyExists);
        }
        if price <= 0 {
            return Err(Error::InvalidPrice);
        }

        let trade = Trade {
            trade_id: trade_id.clone(),
            farmer: farmer.clone(),
            buyer: buyer.clone(),
            asset,
            crop_details,
            price,
            state: TradeState::Pending,
        };
        store_trade(&env, &trade);

        // Emit the creation event so off-chain indexers can track the trade.
        TradeCreated {
            trade_id,
            farmer,
            buyer,
            crop_details: trade.crop_details.clone(),
            price: trade.price,
        }
        .publish(&env);

        Ok(trade)
    }

    /// Locks the buyer's payment in the escrow contract ([`TradeState::Funded`]).
    ///
    /// The caller must be the buyer registered at trade creation — their
    /// [`Address::require_auth`] is enforced. The escrowed amount is *derived
    /// from the trade itself* (exactly `price`), so the escrow can never be
    /// funded with more or less than the agreed price.
    ///
    /// # Arguments
    /// * `trade_id` — identifier of the trade to fund.
    /// * `buyer` — address funding the escrow; must equal `Trade::buyer`.
    ///
    /// # Errors
    /// * [`Error::TradeNotFound`] — no trade with this id exists.
    /// * [`Error::UnauthorizedCaller`] — `buyer` is not the registered buyer.
    /// * [`Error::InvalidState`] — the trade is not [`TradeState::Pending`]
    ///   (e.g. it is already funded — double funding is rejected).
    pub fn deposit_payment(env: Env, trade_id: Symbol, buyer: Address) -> Result<Trade, Error> {
        let mut trade = load_trade(&env, &trade_id)?;

        // Only the buyer registered at creation may fund the escrow.
        if buyer != trade.buyer {
            return Err(Error::UnauthorizedCaller);
        }
        // Prove that the buyer's key authorized this exact call.
        buyer.require_auth();

        // A trade is funded exactly once, from the Pending state.
        if trade.state != TradeState::Pending {
            return Err(Error::InvalidState);
        }

        // Move exactly `price` from the buyer into the contract. The token
        // contract performs the transfer; the buyer's auth covers it as a
        // sub-invocation of this call.
        let token = TokenClient::new(&env, &trade.asset);
        token.transfer(&buyer, env.current_contract_address(), &trade.price);

        trade.state = TradeState::Funded;
        store_trade(&env, &trade);

        TradeFunded {
            trade_id,
            buyer,
            amount: trade.price,
        }
        .publish(&env);

        Ok(trade)
    }

    /// Farmer marks the crop as delivered ([`TradeState::Delivered`]).
    ///
    /// # Arguments
    /// * `trade_id` — identifier of the trade.
    /// * `farmer` — address confirming delivery; must equal `Trade::farmer`.
    ///
    /// # Errors
    /// * [`Error::TradeNotFound`] — no trade with this id exists.
    /// * [`Error::UnauthorizedCaller`] — `farmer` is not the registered farmer.
    /// * [`Error::InvalidState`] — the trade is not [`TradeState::Funded`]
    ///   (delivery cannot be confirmed before the escrow is funded).
    pub fn confirm_delivery(env: Env, trade_id: Symbol, farmer: Address) -> Result<Trade, Error> {
        let mut trade = load_trade(&env, &trade_id)?;

        // Only the registered farmer may mark the crop as delivered.
        if farmer != trade.farmer {
            return Err(Error::UnauthorizedCaller);
        }
        farmer.require_auth();

        // Delivery only makes sense after the escrow is funded.
        if trade.state != TradeState::Funded {
            return Err(Error::InvalidState);
        }

        trade.state = TradeState::Delivered;
        store_trade(&env, &trade);

        TradeDelivered { trade_id, farmer }.publish(&env);

        Ok(trade)
    }

    /// Buyer confirms receipt; the payment is automatically released to the
    /// farmer ([`TradeState::Completed`]).
    ///
    /// This is the only function that moves funds *out* of the escrow: it
    /// transfers `price` from the contract to the farmer.
    ///
    /// # Arguments
    /// * `trade_id` — identifier of the trade.
    /// * `buyer` — address confirming receipt; must equal `Trade::buyer`.
    ///
    /// # Errors
    /// * [`Error::TradeNotFound`] — no trade with this id exists.
    /// * [`Error::UnauthorizedCaller`] — `buyer` is not the registered buyer.
    /// * [`Error::InvalidState`] — the trade is not [`TradeState::Delivered`]
    ///   (receipt cannot be confirmed before delivery).
    pub fn confirm_receipt(env: Env, trade_id: Symbol, buyer: Address) -> Result<Trade, Error> {
        let mut trade = load_trade(&env, &trade_id)?;

        // Only the registered buyer may confirm receipt of the goods.
        if buyer != trade.buyer {
            return Err(Error::UnauthorizedCaller);
        }
        buyer.require_auth();

        // Receipt can only be confirmed after the farmer marked delivery.
        if trade.state != TradeState::Delivered {
            return Err(Error::InvalidState);
        }

        // Release: pay the farmer the escrowed price out of the contract.
        // The contract itself is the `from` address, so no extra auth entry
        // is needed — the contract authorizes its own outgoing transfer.
        let token = TokenClient::new(&env, &trade.asset);
        token.transfer(&env.current_contract_address(), &trade.farmer, &trade.price);

        trade.state = TradeState::Completed;
        store_trade(&env, &trade);

        TradeCompleted {
            trade_id,
            farmer: trade.farmer.clone(),
            buyer,
            amount: trade.price,
        }
        .publish(&env);

        Ok(trade)
    }

    /// Either party freezes the escrow by raising a dispute
    /// ([`TradeState::Disputed`]). Funds stay locked in the contract until
    /// the dispute is resolved (resolution / arbitration is out of scope for
    /// v1 — handled off-chain).
    ///
    /// # Arguments
    /// * `trade_id` — identifier of the trade.
    /// * `caller` — address raising the dispute; must be a trade party.
    /// * `reason` — free-text description of the dispute.
    ///
    /// # Errors
    /// * [`Error::TradeNotFound`] — no trade with this id exists.
    /// * [`Error::UnauthorizedCaller`] — `caller` is neither party.
    /// * [`Error::InvalidState`] — the trade is not funded or delivered; a
    ///   `Pending` trade (no money moved) should be cancelled instead.
    pub fn raise_dispute(
        env: Env,
        trade_id: Symbol,
        caller: Address,
        reason: String,
    ) -> Result<Trade, Error> {
        let mut trade = load_trade(&env, &trade_id)?;

        // Only one of the two parties may raise a dispute.
        if caller != trade.farmer && caller != trade.buyer {
            return Err(Error::UnauthorizedCaller);
        }
        caller.require_auth();

        // Disputes only make sense once funds are locked in escrow.
        if trade.state != TradeState::Funded && trade.state != TradeState::Delivered {
            return Err(Error::InvalidState);
        }

        trade.state = TradeState::Disputed;
        store_trade(&env, &trade);

        TradeDisputed {
            trade_id,
            caller,
            reason,
        }
        .publish(&env);

        Ok(trade)
    }

    /// Cancels a [`TradeState::Pending`] trade ([`TradeState::Cancelled`]).
    ///
    /// Allowed only *before* funding — since no funds have moved yet, nothing
    /// needs to be refunded. The cancelled record is kept on-chain for
    /// auditability.
    ///
    /// # Arguments
    /// * `trade_id` — identifier of the trade.
    /// * `caller` — address requesting the cancellation; must be a trade
    ///   party. (The caller is passed explicitly because either party may
    ///   cancel; [`Address::require_auth`] still proves their key signed it.)
    ///
    /// # Errors
    /// * [`Error::TradeNotFound`] — no trade with this id exists.
    /// * [`Error::UnauthorizedCaller`] — `caller` is neither party.
    /// * [`Error::InvalidState`] — the trade is not [`TradeState::Pending`]
    ///   (once funded, the escrow must complete or be disputed).
    pub fn cancel_trade(env: Env, trade_id: Symbol, caller: Address) -> Result<Trade, Error> {
        let mut trade = load_trade(&env, &trade_id)?;

        // Either party may cancel a not-yet-funded trade.
        if caller != trade.farmer && caller != trade.buyer {
            return Err(Error::UnauthorizedCaller);
        }
        caller.require_auth();

        // Only before funding.
        if trade.state != TradeState::Pending {
            return Err(Error::InvalidState);
        }

        trade.state = TradeState::Cancelled;
        store_trade(&env, &trade);

        TradeCancelled { trade_id, caller }.publish(&env);

        Ok(trade)
    }

    /// View function: returns the full `Trade` struct for `trade_id`.
    ///
    /// No authorization is required — reading on-chain state is public.
    ///
    /// # Arguments
    /// * `trade_id` — identifier of the trade to read.
    ///
    /// # Errors
    /// * [`Error::TradeNotFound`] — no trade with this id exists.
    pub fn get_trade(env: Env, trade_id: Symbol) -> Result<Trade, Error> {
        load_trade(&env, &trade_id)
    }
}
