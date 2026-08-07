# AgriTrust Architecture

AgriTrust is an agricultural escrow platform: farmers list produce, buyers make
offers, funds are held in escrow by a **Soroban smart contract** on Stellar,
and payment is released only after delivery is confirmed. This document
describes the system at a high level. **It is a design document — the codebase
is still in the scaffolding phase and none of the flows below are implemented
yet.**

## 1. Three-tier architecture

AgriTrust is split into three tiers, one per top-level directory:

| Tier | Directory | Stack | Responsibility |
|---|---|---|---|
| Presentation | `frontend/` | React 19 + Vite + TypeScript + Tailwind CSS | Browser UI, Freighter wallet connection, signing |
| Application | `backend/` | Django REST Framework + PostgreSQL | REST API, business logic, Django sessions, DB persistence |
| Contract | `contracts/` | Rust + Soroban SDK (Stellar) | On-chain escrow: hold funds, release, refund, dispute handling |

```
┌─────────────────────┐        ┌──────────────────────┐        ┌──────────────────────┐
│     Frontend        │  HTTPS │        Backend       │  RPC   │       Contract       │
│   React + Vite      ├───────►│  Django REST         ├───────►│   Soroban (Rust)     │
│   Tailwind          │  JSON  │  Framework           │  Soroban│   Escrow contract    │
│   Freighter wallet  │        │  PostgreSQL          │  calls  │   Stellar network    │
└─────────────────────┘        └──────────────────────┘        └──────────────────────┘
        browser                       server                        blockchain
```

- **Frontend** never talks to the blockchain directly. It signs messages with
  the Freighter wallet (challenge-based auth, see §3) and renders state served
  by the REST API.
- **Backend** owns the domain models (users, listings, offers, escrow records,
  deliveries) in PostgreSQL and is the *only* tier that invokes the Soroban
  contract, using the escrow contract address plus RPC (e.g. `soroban-rpc` /
  Futurenet or a local `stellar core` sandbox in development).
- **Contract** is the source of truth for *funds*. The backend mirrors escrow
  lifecycle state in Postgres for querying, but a payout only happens through
  a contract call — the backend can never release funds by itself.

## 2. Core data flow: listing → offer → escrow → delivery → release

The escrow lifecycle is the heart of the product:

```
 LISTING        OFFER          ESCROW             DELIVERY             RELEASE
┌────────┐    ┌────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│ Farmer │    │ Buyer  │    │ Buyer funds │    │ Farmer      │    │ Buyer       │
│ lists  ├───►│ makes  ├───►│ escrow      ├───►│ marks       ├───►│ confirms    │
│ produce│    │ offer  │    │ contract    │    │ delivered   │    │ delivery →  │
│ (price,│    │ (price │    │ holds funds │    │ (proof:     │    │ contract    │
│ qty,   │    │ terms) │    │             │    │ photos,     │    │ pays out to │
│ dates) │    │        │    │             │    │ tracking)   │    │ farmer      │
└────────┘    └────────┘    └─────────────┘    └─────────────┘    └─────────────┘
```

1. **Listing** — A farmer creates a listing (crop, quantity, price, delivery
   window, location). Persisted in Postgres; status `open`.
2. **Offer** — A buyer makes an offer on a listing. The seller may accept one
   offer; all others are rejected. Offer acceptance moves the listing to
   `in_escrow`.
3. **Escrow** — On acceptance, the backend creates an escrow *record* in
   Postgres and instructs the buyer to fund the on-chain escrow contract
   (`escrow.deposit`). The contract holds the funds; nobody can withdraw them
   unilaterally. Escrow record status: `funded`.
4. **Delivery** — The farmer ships and marks the escrow `delivered` with
   proof (tracking number, photos). The buyer can inspect the proof.
5. **Release / Refund / Dispute** —
   - Buyer confirms delivery → backend calls `escrow.release`, the contract
     pays the farmer, escrow record → `released`.
   - Buyer cancels before delivery with seller consent → `escrow.refund`, the
     contract returns funds to the buyer, record → `refunded`.
   - Either party disputes → record → `disputed`; the contract freezes funds
     until the dispute is resolved (arbitration outside the scope of v1; the
     contract supports a `resolve` call by a designated arbiter key).

State machine (record status in Postgres mirrors contract state):

```
open ──► in_escrow ──► funded ──► delivered ──► released
  │          │           │            │
  └─►closed  └─►rejected └─►refunded  └─►disputed ──► resolved
```

## 3. Freighter wallet auth + Django sessions

AgriTrust does not use passwords. Identity is a Stellar keypair controlled by
the user's **Freighter** browser extension. The backend authenticates the
wallet by verifying a signature, then issues a normal **Django session** so the
rest of the request lifecycle is standard server-side auth.

```
 Frontend (Freighter)              Backend (Django)                PostgreSQL
 ─────────────────────              ────────────────                ──────────
 1. "Connect" click
 2. getPublicKey() ────────────────► POST /api/auth/challenge
      { public_key }                 store nonce (challenge) ─────► sessions/challenges
                                     ◄──────────────── 200 { challenge }
 3. signMessage(challenge) ─────────► POST /api/auth/verify
      { public_key, signature }      verify sig w/ Stellar SDK
                                     create/get User(public_key)
                                     login(request, user) ─────────► django_session row
                                     ◄──────────────── 200 + Set-Cookie: sessionid
 4. Subsequent calls ───────────────► GET /api/...  (session cookie)
      Cookie: sessionid              SessionAuthentication → user
```

Details:

- **Challenge (nonce):** short-lived, single-use random string stored
  server-side (or signed with a server secret), with an expiry. Prevents
  replay attacks and forces the wallet holder to prove key possession.
- **Verification:** the backend verifies the Stellar ed25519 signature over
  the challenge using the Stellar SDK (e.g. `stellar-sdk` / `soroban-client`
  in Python) — **no passphrase or private key ever leaves the wallet**.
- **Session:** on success the backend calls `django.contrib.auth.login(...)`,
  creating a standard `django_session` row. The `sessionid` cookie is
  `HttpOnly`, `SameSite=Lax`; production switches on `Secure` via the
  `SESSION_COOKIE_SECURE` / `CSRF_COOKIE_SECURE` environment variables — the
  wallet signature proves identity once; every later request is authenticated
  by the cookie alone via DRF `SessionAuthentication`.
- **CSRF:** because the frontend uses a session cookie, all mutating requests
  must send the CSRF token (Django's `CsrfViewTrustedOrigin` / token header).
  CORS is configured with `CORS_ALLOW_CREDENTIALS = True` and the Vite origin
  in `CORS_ALLOWED_ORIGINS`; the frontend fetch/axios client must send
  `credentials: 'include'`.
- **Wallet / user link:** each Stellar `public_key` maps 1:1 to a `User`
  record; a user may hold multiple wallets in a later phase.

## 4. Directory layout

```
AgriTrust/
├── contracts/          # Soroban smart contract (Rust)
│   └── src/lib.rs      # contract skeleton (no feature logic yet)
├── backend/            # Django REST Framework + PostgreSQL
│   ├── config/         # Django project (settings, urls, wsgi/asgi)
│   └── tests/          # pytest smoke tests
├── frontend/           # React + Vite + Tailwind + Freighter
│   └── src/lib/        # freighter.ts integration surface (placeholder)
├── docs/               # this document + api-spec.md
├── .github/workflows/  # CI (lint + test on push)
└── docker-compose.yml  # local PostgreSQL
```

## 5. Key decisions & assumptions

- Funds are **only** ever moved by the Soroban contract; Postgres state is a
  queryable mirror, not an authority over money.
- v1 escrow supports a single arbiter key for disputes; multi-sig arbitration
  is out of scope.
- Backend ↔ contract communication uses Soroban RPC; the backend holds no
  user private keys, only its own (or a shared) funding key with strict
  permissions.
- Payments are denominated in an on-chain asset (native XLM or a stablecoin);
  exact asset choice is TBD.
