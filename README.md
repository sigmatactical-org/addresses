# sigma-addresses

Billing and shipping address service for Sigma Tactical Group identity users. Every address is owned by exactly one identity `user_id`; there is no admin view and no anonymous access — every route requires an active identity session. [sigma-payments](https://github.com/sigmatactical-org/payments) reads addresses over the internal JSON API to validate that a `billing_address_id` belongs to the payment method's owner before saving.

Repository: https://github.com/sigmatactical-org/addresses

Shared site chrome comes from [sigma-theme](https://github.com/sigmatactical-org/sigma-theme).

## Public vs internal

- **Session-gated web UI** (`addresses.sigma-tactical.com`): every route under `/` requires an identity session cookie. Visitors without one are redirected to identity sign-in and returned here afterward. All reads and writes are scoped to the signed-in user's own `user_id` — there is no cross-user or admin view.
- **Internal only**: the JSON API under `/api`, gated by the shared `SIGMA_INTERNAL_TOKEN`. Used by other services (payments) rather than browsers.

## Features

- **Billing and shipping addresses** — CRUD, grouped by category, one address list per identity user
- **Default address per category** — "Make default" promotes an address to the default billing (or shipping) address for its owner; the database enforces at most one default per `(user_id, category)` via a partial unique index, so promoting a new default clears the previous one in the same transaction
- **Strict per-user scoping** — every store method takes the caller's verified `user_id`; a lookup for another user's address id returns 404, not 403, so existence can't be probed
- **Internal JSON API** — `GET /api/users/{user_id}/addresses[?category=]` and `GET /api/users/{user_id}/addresses/{id}` for payments (and future services) to read and validate addresses

## Configuration

| Variable | Purpose |
|----------|---------|
| `PORT` | Listen port (default `8080`) |
| `DATABASE_URL` | PostgreSQL connection URL (default `postgres://sigma:sigma@127.0.0.1:5432/sigma`) |
| `ADDRESSES_PUBLIC_BASE_URL` | Canonical public URL of this service, for sign-in return links (default `http://127.0.0.1:8089/`) |
| `ADDRESSES_IDENTITY_PUBLIC_URL` | Public identity BFF base URL for the sign-in redirect (default `http://127.0.0.1:3000/`) |
| `ADDRESSES_IDENTITY_INTERNAL_URL` | Cluster-internal identity BFF base URL for server-to-server session checks (falls back to `ADDRESSES_IDENTITY_PUBLIC_URL`) |
| `ADDRESSES_CONTACT_PUBLIC_URL` | Public contact service URL for the navbar link (default `http://127.0.0.1:8083/`) |
| `ADDRESSES_CART_PUBLIC_URL` | Public cart service URL for the navbar link (default `http://127.0.0.1:8084/`) |
| `SIGMA_INTERNAL_TOKEN` | Shared secret for the internal JSON API (see [sigma-pg](https://github.com/sigmatactical-org/sigma-pg)) |

## Data model

Each address has:

- `user_id` — identity user id (owner; every read/write is scoped to this)
- `category` — `billing` or `shipping`, fixed at creation
- optional `label`, `recipient_name`, `line2`, `region`
- `line1`, `city`, `postal_code`, `country` — required
- `is_default` — at most one `true` per `(user_id, category)`, enforced by a partial unique index

Data lives in the shared PostgreSQL `addresses` schema (`addresses.addresses`), owned by the `addresses` role. Schema and role are provisioned by [sigma-pg](https://github.com/sigmatactical-org/sigma-pg)'s migrations, not by this service.

## Admin + JSON API

There is no admin web UI — the web UI at `/` *is* the end-user UI, scoped to whoever is signed in.

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/` | List the signed-in user's addresses, grouped by category |
| `GET` | `/new?category=billing\|shipping` | New address form |
| `POST` | `/` | Create an address |
| `GET` | `/{id}/edit` | Edit form (category is fixed and shown read-only) |
| `POST` | `/{id}` | Update an address |
| `POST` | `/{id}/delete` | Delete an address |
| `POST` | `/{id}/default` | Promote to the default for its category |
| `GET` | `/api/users/{user_id}/addresses` | List a user's addresses (optional `?category=`) — internal token required |
| `GET` | `/api/users/{user_id}/addresses/{id}` | Get one address scoped to `user_id`, 404 if it belongs to someone else — internal token required |

### Behind sigma-identity

Every web route requires an identity session cookie; visitors without one are redirected to:

```
{ADDRESSES_IDENTITY_PUBLIC_URL}/auth/login?app_uri=...&redirect_uri=...
```

and returned to the page they started on after signing in.

## Development

Standalone clone:

```bash
./scripts/prepare-local.sh
cargo run -p sigma-addresses
```

Under the sigma workspace (`sigma/it/addresses`):

```bash
cd sigma/it/addresses && ./scripts/prepare-local.sh && cargo run -p sigma-addresses
```

Open http://localhost:8080

## Docker

Release is in **`.github/workflows/release.yml`** when configured. Locally:

```bash
./scripts/docker-build.sh
docker build -f Dockerfile -t sigma-addresses:local build/image
```

Data is stored in the shared PostgreSQL `addresses` schema (`addresses.addresses`). Postgres runs in the [platform](https://github.com/sigmatactical-org/platform) kind stack — port-forward for local `cargo run`:

```bash
cd platform && ./scripts/postgres-dev.sh port-forward-bg && ./scripts/postgres-dev.sh migrate
```

## Brand & artwork

© Sigma Tactical Group. **All rights reserved.**

The Sigma Tactical Group name, logos, marks, artwork, and visual identity are **proprietary**. They are not covered by this repository's source-code license. See [BRANDING.md](BRANDING.md).

## License

MIT OR Apache-2.0 for **source code** only. Branding remains proprietary.
