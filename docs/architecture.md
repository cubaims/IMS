# IMS Workspace Architecture

This codebase follows module-first DDD + Clean Architecture.

Each bounded context is a crate named `cuba-{module}` and contains four internal layers:

1. `domain`: business vocabulary, value objects, entities, domain errors.
2. `application`: use cases, commands, ports, orchestration.
3. `infrastructure`: PostgreSQL adapters and external integrations.
4. `interface`: DTOs, HTTP handlers, routing.

The API crate is only a delivery/composition crate. It wires bounded contexts and exposes HTTP routes.

## Authentication And Invalidation

IMS currently uses a lightweight access-token invalidation model:

- Access tokens are short-lived self-contained JWTs. The API auth middleware
  validates signature, issuer, expiry, and token type, then trusts embedded
  roles and permissions until the token expires.
- The default access-token lifetime is 900 seconds (`JWT_EXPIRES_SECONDS`, 15
  minutes). Refresh tokens default to 30 days (`JWT_REFRESH_EXPIRES_SECONDS`).
- Login and refresh load the current user, roles, and permissions from
  PostgreSQL. Disabled users cannot log in or refresh. Permission changes take
  effect when a new access token is issued by login/refresh; already-issued
  access tokens can remain valid until expiry.
- Refresh token exchange rotates the refresh token. The previous refresh token
  is revoked and must not be accepted again.

For strong immediate invalidation, replace the middleware policy with a
database-backed check on every authenticated request: confirm the user is still
enabled and optionally compare a token version claim with the current user
record. That is a different consistency/performance trade-off and is not the
current runtime model.
