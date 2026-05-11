# Frontend Authentication Guide

This document is the frontend contract for IMS authentication and authorization.
Use it as the source of truth when wiring login, token refresh, route guards,
menus, and permission-gated buttons.

## Runtime Contract

- Base auth path: `/api/auth`.
- Access token type: `Bearer`.
- Access token lifetime: `expires_in` seconds, currently 900 seconds by
  default.
- Access token model: short-lived self-contained JWT. The frontend should treat
  permissions in the login/refresh response as valid until the access token
  expires.
- Permission changes and disabled users take effect at login/refresh. An already
  issued access token can remain valid until expiry.
- Refresh tokens rotate on every successful refresh. Always replace both stored
  tokens with the newest response. Never reuse an older refresh token.
- `x-request-id` is returned as a response header. Include it in frontend error
  logs and support reports.

## Response Shapes

Successful responses use:

```json
{
  "success": true,
  "data": {},
  "message": "OK"
}
```

Errors use:

```json
{
  "success": false,
  "error_code": "UNAUTHORIZED",
  "message": "TOKEN_EXPIRED"
}
```

Frontend code should branch by HTTP status first, then `error_code`/`message`
for display or recovery.

## Login

`POST /api/auth/login`

Request:

```json
{
  "username": "admin",
  "password": "password"
}
```

Success `data`:

```json
{
  "access_token": "<jwt>",
  "refresh_token": "<selector.secret>",
  "token_type": "Bearer",
  "expires_in": 900,
  "refresh_expires_in": 2592000,
  "user": {
    "user_id": "00000000-0000-0000-0000-000000000000",
    "username": "admin",
    "display_name": "Admin",
    "email": "admin@example.com",
    "roles": ["ADMIN"],
    "permissions": ["SYS_ALL"]
  }
}
```

After login:

- Store `access_token`, `refresh_token`, and `user`.
- Use `Authorization: Bearer <access_token>` for protected APIs.
- Build menus and buttons from `user.roles` and `user.permissions`.

## Current User

`GET /api/auth/me`

Headers:

```http
Authorization: Bearer <access_token>
```

Use this endpoint after page reload if local user data is missing or stale. It
returns the current user profile and fresh roles/permissions from PostgreSQL.

## Current Roles And Permissions

These endpoints are for the current logged-in user. They are not system role or
permission administration APIs.

`GET /api/auth/roles`

Success `data`:

```json
{
  "user_id": "00000000-0000-0000-0000-000000000000",
  "username": "admin",
  "roles": ["ADMIN"]
}
```

`GET /api/auth/permissions`

Success `data`:

```json
{
  "user_id": "00000000-0000-0000-0000-000000000000",
  "username": "admin",
  "permissions": ["SYS_ALL"]
}
```

Use these endpoints when the frontend only needs one side of the current user's
authorization state. Use `/api/auth/me` when both profile and authorization are
needed together.

System role administration is under `/api/system/roles` and requires `ADMIN`.

## Refresh

`POST /api/auth/refresh`

Request:

```json
{
  "refresh_token": "<current refresh token>"
}
```

Success response has the same `data` shape as login. Replace all auth state with
the returned values:

- `access_token`
- `refresh_token`
- `user`

If refresh fails with `401`, clear auth state and send the user back to login.

## Error Handling

Recommended handling:

- `401 TOKEN_EXPIRED`: try one refresh, then retry the original request.
- `401 TOKEN_INVALID`, `401 REFRESH_TOKEN_INVALID`, `401 UNAUTHORIZED`: clear
  auth state and redirect to login.
- `403 PERMISSION_DENIED`: do not auto-refresh by default. Show a no-permission
  state. Optionally refresh once only if the user just changed accounts or was
  recently granted permissions.
- `400 VALIDATION_ERROR`: show form validation feedback.
- `404 NOT_FOUND`: show empty/not-found state.
- `409` business conflicts: show the returned `message`.
- `500`: show a generic failure message and log `x-request-id`.

Avoid multiple simultaneous refresh requests. Gate refresh behind one shared
promise and let all failed requests wait for it.

## TypeScript Client Sketch

```ts
type ApiSuccess<T> = {
  success: true;
  data: T;
  message?: string;
};

type ApiFailure = {
  success: false;
  error_code: string;
  message: string;
};

type ApiResponse<T> = ApiSuccess<T> | ApiFailure;

type AuthUser = {
  user_id: string;
  username: string;
  display_name: string | null;
  email: string | null;
  roles: string[];
  permissions: string[];
};

type LoginResponse = {
  access_token: string;
  refresh_token: string;
  token_type: "Bearer";
  expires_in: number;
  refresh_expires_in: number;
  user: AuthUser;
};

let accessToken = localStorage.getItem("ims_access_token");
let refreshToken = localStorage.getItem("ims_refresh_token");
let refreshInFlight: Promise<boolean> | null = null;

function saveAuth(data: LoginResponse) {
  accessToken = data.access_token;
  refreshToken = data.refresh_token;
  localStorage.setItem("ims_access_token", data.access_token);
  localStorage.setItem("ims_refresh_token", data.refresh_token);
  localStorage.setItem("ims_user", JSON.stringify(data.user));
}

function clearAuth() {
  accessToken = null;
  refreshToken = null;
  localStorage.removeItem("ims_access_token");
  localStorage.removeItem("ims_refresh_token");
  localStorage.removeItem("ims_user");
}

async function refreshAccessToken(): Promise<boolean> {
  if (!refreshToken) return false;
  if (refreshInFlight) return refreshInFlight;

  refreshInFlight = fetch("/api/auth/refresh", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ refresh_token: refreshToken }),
  })
    .then(async (response) => {
      const body = (await response.json()) as ApiResponse<LoginResponse>;
      if (!response.ok || !body.success) return false;
      saveAuth(body.data);
      return true;
    })
    .catch(() => false)
    .finally(() => {
      refreshInFlight = null;
    });

  return refreshInFlight;
}

async function apiFetch<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers);
  if (!headers.has("Content-Type") && init.body) {
    headers.set("Content-Type", "application/json");
  }
  if (accessToken) headers.set("Authorization", `Bearer ${accessToken}`);

  let response = await fetch(path, { ...init, headers });
  let body = (await response.json()) as ApiResponse<T>;

  if (response.status === 401 && refreshToken) {
    const refreshed = await refreshAccessToken();
    if (refreshed && accessToken) {
      headers.set("Authorization", `Bearer ${accessToken}`);
      response = await fetch(path, { ...init, headers });
      body = (await response.json()) as ApiResponse<T>;
    }
  }

  if (!response.ok || !body.success) {
    const requestId = response.headers.get("x-request-id");
    const message = "message" in body ? body.message : "Request failed";
    const error = new Error(requestId ? `${message} (${requestId})` : message);
    throw error;
  }

  return body.data;
}
```

## Permission Checks

Backend behavior:

- Role `ADMIN` allows all permissions.
- Permission `SYS_ALL` or `*` allows all permissions.
- Module wildcard permission such as `inventory:*` allows
  `inventory:read`, `inventory:post`, etc.
- Exact permissions are case-insensitive in the backend.

Frontend helper:

```ts
function hasRole(user: AuthUser | null, role: string): boolean {
  return !!user?.roles.some((r) => r.toLowerCase() === role.toLowerCase());
}

function hasPermission(user: AuthUser | null, permission: string): boolean {
  if (!user) return false;
  if (hasRole(user, "ADMIN")) return true;

  const required = permission.trim().toLowerCase();
  return user.permissions.some((raw) => {
    const granted = raw.trim().toLowerCase();
    if (granted === "*" || granted === "sys_all") return true;
    if (granted === required) return true;
    if (granted.endsWith(":*")) {
      const module = granted.slice(0, -2);
      return required.startsWith(`${module}:`);
    }
    return false;
  });
}

function hasAnyPermission(user: AuthUser | null, permissions: string[]): boolean {
  return permissions.some((permission) => hasPermission(user, permission));
}
```

Use permission checks to hide or disable UI actions, but still handle `403`
responses because the backend is authoritative.

## Common Permissions

| Area | Permission |
| --- | --- |
| Master data read | `master-data:read` |
| Master data write | `master-data:write` |
| Inventory read | `inventory:read` |
| Inventory history | `inventory:history` |
| Inventory post | `inventory:post` |
| Inventory transfer | `inventory:transfer` |
| Batch read | `batch:read` |
| Batch history | `batch:history` |
| Inventory count read | `inventory-count:read` |
| Inventory count write | `inventory-count:write` |
| Inventory count submit | `inventory-count:submit` |
| Inventory count approve | `inventory-count:approve` |
| Inventory count post | `inventory-count:post` |
| Inventory count close | `inventory-count:close` |
| Purchase write | `purchase:write` |
| Purchase receipt | `purchase:receipt` |
| Sales write | `sales:write` |
| Sales shipment | `sales:shipment` |
| Production read | `production:read` |
| Production write | `production:write` |
| Production release | `production:release` |
| Production complete | `production:complete` |
| Quality read | `quality:read` |
| Quality write | `quality:write` |
| Quality decision | `quality:decision` |
| Traceability read | `traceability:read` |
| MRP run | `mrp:run` |
| MRP read | `mrp:read` |
| MRP suggestion read | `mrp:suggestion-read` |
| MRP suggestion confirm | `mrp:suggestion-confirm` |
| Report read | `report:read` |
| Report refresh | `report:refresh` |
| Report export | `report:export` |
| Audit read | `audit:read` |
| System parameter read | `system-param:read` |
| System parameter write | `system-param:write` |

## Logout

There is no server-side logout endpoint yet. Frontend logout should clear local
auth state and navigate to the login screen. The current refresh token remains
valid until expiry or until server-side revocation is added.

## Frontend Checklist

- Store only the latest refresh token after login/refresh.
- Retry a request at most once after a successful refresh.
- Do not send refresh requests in parallel.
- Treat `403` as authorization failure, not as normal token expiry.
- Read `x-request-id` from response headers when logging failures.
- Rebuild menus and action buttons after every login, refresh, or `/me` load.
