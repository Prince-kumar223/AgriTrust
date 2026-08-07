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

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        symbol_short,
        testutils::{Address as _, Events as _, MockAuth, MockAuthInvoke},
        xdr::{ContractEventBody, ScVal},
        IntoVal, TryFromVal, Val, Vec,
    };

    const TRADE_ID: &str = "trade_1";
    const PRICE: i128 = 1_000_000; // in the token's smallest unit (e.g. stroops)
    const CROP_DETAILS: &str = "2t wheat, Grade A";

    // -----------------------------------------------------------------------
    // Minimal mock token (SEP-41 subset) so tests exercise real cross-contract
    // `transfer` calls without needing a deployed asset. `mint` is test-only;
    // `transfer` panics on insufficient balance like a real token contract.
    // -----------------------------------------------------------------------

    #[contract]
    pub struct MockToken;

    #[contracttype]
    enum MockTokenDataKey {
        Balance(Address),
    }

    #[contractimpl]
    impl MockToken {
        /// Mint `amount` tokens to `to` (no admin checks — test-only).
        pub fn mint(env: Env, to: Address, amount: i128) {
            let key = MockTokenDataKey::Balance(to);
            let balance: i128 = env.storage().persistent().get(&key).unwrap_or(0);
            env.storage().persistent().set(&key, &(balance + amount));
        }

        /// Returns the token balance of `id`.
        pub fn balance(env: Env, id: Address) -> i128 {
            env.storage()
                .persistent()
                .get(&MockTokenDataKey::Balance(id))
                .unwrap_or(0)
        }

        /// Moves `amount` tokens from `from` to `to`.
        pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
            let from_key = MockTokenDataKey::Balance(from);
            let from_balance: i128 = env.storage().persistent().get(&from_key).unwrap_or(0);
            if from_balance < amount {
                panic!("insufficient balance");
            }
            let to_key = MockTokenDataKey::Balance(to);
            let to_balance: i128 = env.storage().persistent().get(&to_key).unwrap_or(0);
            env.storage()
                .persistent()
                .set(&from_key, &(from_balance - amount));
            env.storage()
                .persistent()
                .set(&to_key, &(to_balance + amount));
        }
    }

    // -----------------------------------------------------------------------
    // Test harness
    // -----------------------------------------------------------------------

    /// Fresh environment + deployed escrow contract + funded mock token.
    struct TestEnv {
        env: Env,
        contract_id: Address,
        /// Generated client for the escrow contract (see `client()`).
        farmer: Address,
        buyer: Address,
        third_party: Address,
        token: Address,
    }

    impl TestEnv {
        fn new() -> Self {
            let env = Env::default();
            env.mock_all_auths(); // authorize every call; auth is tested separately

            let farmer = Address::generate(&env);
            let buyer = Address::generate(&env);
            let third_party = Address::generate(&env);

            let token = env.register(MockToken, ());
            let token_client = MockTokenClient::new(&env, &token);
            // Fund both parties so they can pay into / receive from the escrow.
            token_client.mint(&farmer, &(PRICE * 10));
            token_client.mint(&buyer, &(PRICE * 10));

            let contract_id = env.register(AgriTrustContract, ());

            TestEnv {
                env,
                contract_id,
                farmer,
                buyer,
                third_party,
                token,
            }
        }

        /// The escrow contract client.
        fn client(&self) -> AgriTrustContractClient<'static> {
            AgriTrustContractClient::new(&self.env, &self.contract_id)
        }

        /// The mock token client.
        fn token_client(&self) -> MockTokenClient<'static> {
            MockTokenClient::new(&self.env, &self.token)
        }

        fn trade_id(&self) -> Symbol {
            Symbol::new(&self.env, TRADE_ID)
        }

        fn create_default_trade(&self) -> Trade {
            self.client().create_trade(
                &self.trade_id(),
                &self.farmer,
                &self.buyer,
                &self.token,
                &String::from_str(&self.env, CROP_DETAILS),
                &PRICE,
            )
        }

        fn balance_of(&self, address: &Address) -> i128 {
            self.token_client().balance(address)
        }

        fn contract_balance(&self) -> i128 {
            self.balance_of(&self.contract_id)
        }
    }

    // -----------------------------------------------------------------------
    // Helpers for event and auth assertions
    // -----------------------------------------------------------------------

    /// First topic of the most recently published event.
    fn last_event_topic(env: &Env) -> ScVal {
        let events = env.events().all();
        let last = events.events().last().expect("expected an emitted event");
        match &last.body {
            ContractEventBody::V0(v0) => v0.topics[0].clone(),
        }
    }

    /// Asserts the most recent event's fixed topic is `name` (the snake_case
    /// struct name of the `#[contractevent]`).
    fn assert_last_event(env: &Env, name: &str) {
        let topic: Val = Symbol::new(env, name).into_val(env);
        let expected: ScVal = ScVal::try_from_val(env, &topic).unwrap();
        assert_eq!(
            last_event_topic(env),
            expected,
            "expected event topic '{name}'"
        );
    }

    /// Mocks authorization for exactly one caller on one contract function, so
    /// `require_auth()` inside the contract rejects everyone else.
    fn mock_auth_only_for(
        env: &Env,
        contract_id: &Address,
        caller: &Address,
        fn_name: &str,
        fn_args: &[Val],
    ) {
        let mut args: Vec<Val> = Vec::new(env);
        for arg in fn_args {
            args.push_back(*arg); // `Val` is `Copy`
        }
        let invoke = MockAuthInvoke {
            contract: contract_id,
            fn_name,
            args,
            sub_invokes: &[],
        };
        env.mock_auths(&[MockAuth {
            address: caller,
            invoke: &invoke,
        }]);
    }

    // -----------------------------------------------------------------------
    // Happy path
    // -----------------------------------------------------------------------

    #[test]
    fn test_full_happy_path() {
        let t = TestEnv::new();
        let trade_id = t.trade_id();

        // 1. Farmer creates the trade (Pending, no funds move).
        let trade = t.create_default_trade();
        assert_eq!(trade.state, TradeState::Pending);
        assert_eq!(trade.farmer, t.farmer);
        assert_eq!(trade.buyer, t.buyer);
        assert_eq!(trade.asset, t.token);
        assert_eq!(trade.price, PRICE);
        assert_last_event(&t.env, "trade_created");

        // 2. Buyer funds the escrow: their tokens move into the contract.
        //    (Note: `env.events().all()` only surfaces the most recent call's
        //    events, so the event must be asserted before any further calls.)
        t.client().deposit_payment(&trade_id, &t.buyer);
        assert_last_event(&t.env, "trade_funded");
        assert_eq!(t.balance_of(&t.buyer), PRICE * 10 - PRICE);
        assert_eq!(t.contract_balance(), PRICE);

        // 3. Farmer marks the crop delivered.
        t.client().confirm_delivery(&trade_id, &t.farmer);
        assert_last_event(&t.env, "trade_delivered");

        // 4. Buyer confirms receipt; payment is released to the farmer.
        t.client().confirm_receipt(&trade_id, &t.buyer);
        assert_last_event(&t.env, "trade_completed");
        assert_eq!(t.contract_balance(), 0);
        assert_eq!(t.balance_of(&t.farmer), PRICE * 10 + PRICE);

        // 5. Final on-chain state.
        let trade = t.client().get_trade(&trade_id);
        assert_eq!(trade.state, TradeState::Completed);
    }

    #[test]
    fn test_get_trade_returns_full_struct() {
        let t = TestEnv::new();
        t.create_default_trade();

        let trade = t.client().get_trade(&t.trade_id());
        assert_eq!(trade.trade_id, t.trade_id());
        assert_eq!(trade.farmer, t.farmer);
        assert_eq!(trade.buyer, t.buyer);
        assert_eq!(trade.asset, t.token);
        assert_eq!(trade.crop_details, String::from_str(&t.env, CROP_DETAILS));
        assert_eq!(trade.price, PRICE);
        assert_eq!(trade.state, TradeState::Pending);
    }

    // -----------------------------------------------------------------------
    // State-machine failures (caller authorized, but transition is illegal)
    // -----------------------------------------------------------------------

    #[test]
    fn test_double_funding_rejected() {
        let t = TestEnv::new();
        t.create_default_trade();
        t.client().deposit_payment(&t.trade_id(), &t.buyer);

        // A second deposit must be rejected by the state machine.
        let result = t.client().try_deposit_payment(&t.trade_id(), &t.buyer);
        assert!(matches!(result, Err(Ok(Error::InvalidState))));
        // The contract still holds exactly one deposit's worth of funds.
        assert_eq!(t.contract_balance(), PRICE);
    }

    #[test]
    fn test_confirm_delivery_before_funding_rejected() {
        let t = TestEnv::new();
        t.create_default_trade();

        let result = t.client().try_confirm_delivery(&t.trade_id(), &t.farmer);
        assert!(matches!(result, Err(Ok(Error::InvalidState))));
    }

    #[test]
    fn test_confirm_receipt_before_delivery_rejected() {
        let t = TestEnv::new();
        t.create_default_trade();
        t.client().deposit_payment(&t.trade_id(), &t.buyer);

        // Buyer tries to confirm receipt while the crop is not yet delivered.
        let result = t.client().try_confirm_receipt(&t.trade_id(), &t.buyer);
        assert!(matches!(result, Err(Ok(Error::InvalidState))));
        assert_eq!(t.contract_balance(), PRICE); // funds untouched
    }

    #[test]
    fn test_cancel_after_funding_rejected() {
        let t = TestEnv::new();
        t.create_default_trade();
        t.client().deposit_payment(&t.trade_id(), &t.buyer);

        let result = t.client().try_cancel_trade(&t.trade_id(), &t.farmer);
        assert!(matches!(result, Err(Ok(Error::InvalidState))));
    }

    #[test]
    fn test_duplicate_trade_id_rejected() {
        let t = TestEnv::new();
        t.create_default_trade();

        let result = t.client().try_create_trade(
            &t.trade_id(),
            &t.farmer,
            &t.buyer,
            &t.token,
            &String::from_str(&t.env, CROP_DETAILS),
            &PRICE,
        );
        assert!(matches!(result, Err(Ok(Error::TradeAlreadyExists))));
    }

    #[test]
    fn test_zero_price_rejected() {
        let t = TestEnv::new();

        let result = t.client().try_create_trade(
            &t.trade_id(),
            &t.farmer,
            &t.buyer,
            &t.token,
            &String::from_str(&t.env, CROP_DETAILS),
            &0,
        );
        assert!(matches!(result, Err(Ok(Error::InvalidPrice))));
    }

    #[test]
    fn test_get_trade_not_found() {
        let t = TestEnv::new();
        let result = t.client().try_get_trade(&symbol_short!("missing"));
        assert!(matches!(result, Err(Ok(Error::TradeNotFound))));
    }

    #[test]
    fn test_deposit_by_unregistered_buyer_rejected() {
        let t = TestEnv::new();
        t.create_default_trade();

        // `third_party` is not the trade's buyer, even with auth mocked.
        let result = t
            .client()
            .try_deposit_payment(&t.trade_id(), &t.third_party);
        assert!(matches!(result, Err(Ok(Error::UnauthorizedCaller))));
        assert_eq!(t.contract_balance(), 0);
    }

    // -----------------------------------------------------------------------
    // Disputes
    // -----------------------------------------------------------------------

    #[test]
    fn test_dispute_freezes_funds() {
        let t = TestEnv::new();
        t.create_default_trade();
        t.client().deposit_payment(&t.trade_id(), &t.buyer);

        let disputed = t.client().raise_dispute(
            &t.trade_id(),
            &t.farmer,
            &String::from_str(&t.env, "crop quality dispute"),
        );
        assert_eq!(disputed.state, TradeState::Disputed);
        assert_last_event(&t.env, "trade_disputed");

        // Funds stay locked in the contract.
        assert_eq!(t.contract_balance(), PRICE);

        // No further transitions are possible while disputed.
        let receipt = t.client().try_confirm_receipt(&t.trade_id(), &t.buyer);
        assert!(matches!(receipt, Err(Ok(Error::InvalidState))));
        let delivery = t.client().try_confirm_delivery(&t.trade_id(), &t.farmer);
        assert!(matches!(delivery, Err(Ok(Error::InvalidState))));
        let cancel = t.client().try_cancel_trade(&t.trade_id(), &t.buyer);
        assert!(matches!(cancel, Err(Ok(Error::InvalidState))));
    }

    #[test]
    fn test_buyer_can_also_raise_dispute() {
        let t = TestEnv::new();
        t.create_default_trade();
        t.client().deposit_payment(&t.trade_id(), &t.buyer);

        let result = t.client().raise_dispute(
            &t.trade_id(),
            &t.buyer,
            &String::from_str(&t.env, "never received the goods"),
        );
        assert_eq!(result.state, TradeState::Disputed);
    }

    #[test]
    fn test_dispute_before_funding_rejected() {
        let t = TestEnv::new();
        t.create_default_trade();

        // Pending trades should be cancelled, not disputed.
        let result = t.client().try_raise_dispute(
            &t.trade_id(),
            &t.buyer,
            &String::from_str(&t.env, "premature"),
        );
        assert!(matches!(result, Err(Ok(Error::InvalidState))));
    }

    // -----------------------------------------------------------------------
    // Cancellation
    // -----------------------------------------------------------------------

    #[test]
    fn test_cancel_pending_trade() {
        let t = TestEnv::new();
        t.create_default_trade();

        let cancelled = t.client().cancel_trade(&t.trade_id(), &t.buyer);
        assert_eq!(cancelled.state, TradeState::Cancelled);
        assert_last_event(&t.env, "trade_cancelled");
        assert_eq!(t.contract_balance(), 0); // nothing moved, nothing to refund
    }

    // -----------------------------------------------------------------------
    // Authorization failures (require_auth rejects the caller)
    // -----------------------------------------------------------------------

    #[test]
    #[should_panic] // `require_auth(buyer)` fails: the farmer's auth is not enough
    fn test_deposit_by_farmer_fails_auth() {
        let t = TestEnv::new();
        t.create_default_trade();

        let trade_id = t.trade_id();
        let args: [Val; 2] = [
            trade_id.clone().into_val(&t.env),
            t.buyer.clone().into_val(&t.env),
        ];
        mock_auth_only_for(&t.env, &t.contract_id, &t.farmer, "deposit_payment", &args);

        t.client().deposit_payment(&trade_id, &t.buyer);
    }

    #[test]
    #[should_panic] // `require_auth(farmer)` fails: the buyer's auth is not enough
    fn test_confirm_delivery_by_buyer_fails_auth() {
        let t = TestEnv::new();
        t.create_default_trade();
        t.client().deposit_payment(&t.trade_id(), &t.buyer);

        let trade_id = t.trade_id();
        let args: [Val; 2] = [
            trade_id.clone().into_val(&t.env),
            t.farmer.clone().into_val(&t.env),
        ];
        mock_auth_only_for(&t.env, &t.contract_id, &t.buyer, "confirm_delivery", &args);

        t.client().confirm_delivery(&trade_id, &t.farmer);
    }

    #[test]
    #[should_panic] // `require_auth(buyer)` fails
    fn test_confirm_receipt_by_farmer_fails_auth() {
        let t = TestEnv::new();
        t.create_default_trade();
        t.client().deposit_payment(&t.trade_id(), &t.buyer);
        t.client().confirm_delivery(&t.trade_id(), &t.farmer);

        let trade_id = t.trade_id();
        let args: [Val; 2] = [
            trade_id.clone().into_val(&t.env),
            t.buyer.clone().into_val(&t.env),
        ];
        mock_auth_only_for(&t.env, &t.contract_id, &t.farmer, "confirm_receipt", &args);

        t.client().confirm_receipt(&trade_id, &t.buyer);
    }

    #[test]
    #[should_panic] // `require_auth(caller)` fails: third party is not a trade party
    fn test_raise_dispute_by_third_party_fails_auth() {
        let t = TestEnv::new();
        t.create_default_trade();
        t.client().deposit_payment(&t.trade_id(), &t.buyer);

        let trade_id = t.trade_id();
        let reason = String::from_str(&t.env, "impostor");
        let args: [Val; 3] = [
            trade_id.clone().into_val(&t.env),
            t.third_party.clone().into_val(&t.env),
            reason.clone().into_val(&t.env),
        ];
        mock_auth_only_for(
            &t.env,
            &t.contract_id,
            &t.third_party,
            "raise_dispute",
            &args,
        );

        t.client().raise_dispute(&trade_id, &t.third_party, &reason);
    }

    #[test]
    #[should_panic] // `require_auth(caller)` fails: third party is not a trade party
    fn test_cancel_by_third_party_fails_auth() {
        let t = TestEnv::new();
        t.create_default_trade();

        let trade_id = t.trade_id();
        let args: [Val; 2] = [
            trade_id.clone().into_val(&t.env),
            t.third_party.clone().into_val(&t.env),
        ];
        mock_auth_only_for(
            &t.env,
            &t.contract_id,
            &t.third_party,
            "cancel_trade",
            &args,
        );

        t.client().cancel_trade(&trade_id, &t.third_party);
    }

    // -----------------------------------------------------------------------
    // Token edge cases
    // -----------------------------------------------------------------------

    #[test]
    #[should_panic] // the mock token panics on insufficient balance
    fn test_deposit_with_insufficient_funds_panics() {
        let t = TestEnv::new();
        let trade_id = Symbol::new(&t.env, "unfunded_buyer");
        // `third_party` was never minted any tokens.
        t.client().create_trade(
            &trade_id,
            &t.farmer,
            &t.third_party,
            &t.token,
            &String::from_str(&t.env, "very expensive crop"),
            &(PRICE * 1000),
        );

        t.client().deposit_payment(&trade_id, &t.third_party);
    }
}
