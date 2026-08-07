# AgriTrust REST API Specification (draft)

> **Status: draft / planned.** The backend is scaffolded with Django REST
> Framework but no endpoints are implemented yet. This document is the agreed
> contract for the next phase.

## Conventions

- Base URL (dev): `http://localhost:8000/api`
- Content type: `application/json`
- Auth: session cookie (`sessionid`) obtained via the wallet auth flow; all
  requests must include cookies (`credentials: "include"`) and the CSRF token
  header for mutating requests.
- Errors: DRF-style `{ "detail": "..." }` or `{ "<field>": ["..."] }` with the
  appropriate 4xx/5xx status.
- The escrow lifecycle statuses follow `docs/architecture.md` §2:
  `open`, `in_escrow`, `funded`, `delivered`, `released`, `refunded`,
  `disputed`, `resolved`, `closed`, `rejected`.

## Auth (Freighter wallet)

| Method | Path | Auth | Description |
|---|---|---|---|
| POST | `/api/auth/challenge` | – | Body `{ "public_key": "G..." }`. Returns `{ "challenge": "<nonce>", "expires_at": ... }` |
| POST | `/api/auth/verify` | – | Body `{ "public_key": "G...", "signature": "...", "challenge": "<nonce>" }`. Verifies signature, logs in, sets session cookie. Returns `{ "user": {...} }` |
| POST | `/api/auth/logout` | session | Ends the Django session |
| GET | `/api/auth/me` | session | Returns the authenticated user |

## Listings (farmer posts produce)

| Method | Path | Auth | Description |
|---|---|---|---|
| GET | `/api/listings` | – | Public list, filters: `crop`, `status`, `location`, pagination |
| POST | `/api/listings` | session (farmer) | Create listing `{ crop, quantity, unit, price, currency, location, delivery_window }` |
| GET | `/api/listings/{id}` | – | Detail |
| PATCH | `/api/listings/{id}` | owner | Update (only while `open`) |
| DELETE | `/api/listings/{id}` | owner | Cancel listing (only while `open`) |

## Offers (buyer bids)

| Method | Path | Auth | Description |
|---|---|---|---|
| POST | `/api/listings/{id}/offers` | session (buyer) | Make offer `{ price, quantity, message? }` |
| GET | `/api/listings/{id}/offers` | listing owner | List offers on a listing |
| GET | `/api/offers/mine` | session | Offers I made, with status |
| POST | `/api/offers/{id}/accept` | listing owner | Accept → listing becomes `in_escrow`, other offers `rejected` |

## Escrows (funds held on-chain)

| Method | Path | Auth | Description |
|---|---|---|---|
| POST | `/api/escrows` | session (buyer) | Create from accepted offer. Returns funding instructions: `{ escrow_id, contract_id, deposit_address, amount, deadline }` |
| GET | `/api/escrows` | session | My escrows (buyer or seller), filters by status |
| GET | `/api/escrows/{id}` | party | Detail incl. on-chain status (polled from RPC) |
| POST | `/api/escrows/{id}/deposit-confirmed` | buyer | Called after buyer funded the contract → record `funded` |

## Delivery & release

| Method | Path | Auth | Description |
|---|---|---|---|
| POST | `/api/escrows/{id}/deliveries` | seller | Mark delivered `{ tracking_number?, notes?, photos: [...] }` → `delivered` |
| GET | `/api/escrows/{id}/deliveries` | party | Delivery proof |
| POST | `/api/escrows/{id}/release` | buyer | Confirm delivery → backend invokes contract `release` → `released` |
| POST | `/api/escrows/{id}/refund` | buyer (or seller w/ consent) | Cancel → backend invokes contract `refund` → `refunded` |
| POST | `/api/escrows/{id}/dispute` | party | Freeze → `disputed` |

## Example: wallet verify flow

```
POST /api/auth/challenge
{ "public_key": "GBY..."
→ 200 { "challenge": "a1b2c3...", "expires_at": "2026-08-07T12:00:00Z" }

POST /api/auth/verify
{ "public_key": "GBY...", "signature": "<base64 ed25519 sig>", "challenge": "a1b2c3..." }
→ 200 { "user": { "id": 1, "public_key": "GBY...", "role": "farmer" } }
  Set-Cookie: sessionid=...; HttpOnly; SameSite=Lax
```

## Out of scope for v1

- OAuth / email-password login
- Dispute resolution endpoints beyond the freeze action (manual arbitration)
- Webhooks / Soroban event ingestion (escrow state may be polled instead)
