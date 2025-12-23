# Curadesk Support System

A support ticket system with separate identity management and SQLCipher-encrypted databases.

## Architecture

```
┌─────────────────────────────────────────┐
│         identity-server                  │
│  - User accounts & roles                │
│  - API key management                   │
│  - Subscription status                  │
│  - SQLCipher encrypted (identity.db)    │
│                                         │
│  CLI: create-user, create-key, revoke   │
│  HTTP: POST /validate                   │
└──────────────────┬──────────────────────┘
                   │ validates API keys
                   ▼
┌─────────────────────────────────────────┐
│         support-server                   │
│  - Tickets & comments                   │
│  - File attachments (ZIP)               │
│  - SQLCipher encrypted (support.db)     │
│                                         │
│  HTTP: /tickets, /admin/tickets/*       │
└──────────────────┬──────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────┐
│         support-cli                      │
│  - TUI for support staff                │
│  - View/manage tickets                  │
└─────────────────────────────────────────┘
```

## Components

### identity-server

Manages users, roles, and API keys. Keys are stored hashed (SHA-256) and shown only once at creation.

**User roles:**
- `admin` - Full access to all tickets and admin endpoints
- `support` - Support staff (currently same as admin)
- `customer` - Can only access own tickets

### support-server

Handles support tickets with file attachments. Validates all requests against the identity service.

### support-cli

Terminal UI for support staff to view and manage tickets.

## Quick Start

### 1. Build all components

```bash
cargo build --release
```

### 2. Initialize identity service

```bash
# Set encryption key (use a strong key in production)
export IDENTITY_DB_KEY="your-secret-encryption-key"

# Seed development data (creates 3 users with keys)
cargo run -p identity-server -- seed

# Or create users manually
cargo run -p identity-server -- create-user --email admin@example.com --role admin
cargo run -p identity-server -- create-key --user-id 1
# Save the displayed key - it won't be shown again!
```

### 3. Start identity service

```bash
export IDENTITY_DB_KEY="your-secret-encryption-key"
cargo run -p identity-server -- serve --port 3001
```

### 4. Start support server

```bash
export SUPPORT_DB_KEY="another-encryption-key"
export IDENTITY_SERVICE_URL="http://localhost:3001"

# With seed data for development
cargo run -p support-server -- --seed

# Or without seed data
cargo run -p support-server
```

### 5. Use the CLI

```bash
export SUPPORT_API_KEY="sk_xxxxx_yyyyyyyyyyyyyyyyyyy"
export SUPPORT_URL="http://localhost:3000"
cargo run -p support-cli
```

## Configuration

### Environment Variables

| Variable | Service | Required | Default | Description |
|----------|---------|----------|---------|-------------|
| `IDENTITY_DB_KEY` | identity-server | Yes | - | SQLCipher encryption key |
| `SUPPORT_DB_KEY` | support-server | Yes | - | SQLCipher encryption key |
| `IDENTITY_SERVICE_URL` | support-server | No | `http://localhost:3001` | Identity service URL |
| `SUPPORT_API_KEY` | support-cli | Yes | - | API key for authentication |
| `SUPPORT_URL` | support-cli | No | `http://localhost:3000` | Support server URL |

### Command-line Arguments

#### identity-server

```
identity-server [OPTIONS] <COMMAND>

Commands:
  serve        Start the HTTP server
  create-user  Create a new user
  create-key   Create an API key for a user
  revoke-key   Revoke an API key by prefix
  list-users   List all users
  list-keys    List all API keys
  seed         Seed development data

Options:
  --db-key <KEY>     SQLCipher encryption key [env: IDENTITY_DB_KEY]
  --db-path <PATH>   Database file path [default: identity.db]
```

#### support-server

```
support-server [OPTIONS]

Options:
  --seed                    Seed the database with test data
  --db-key <KEY>            SQLCipher encryption key [env: SUPPORT_DB_KEY]
  --db-path <PATH>          Database file path [default: support.db]
  --identity-url <URL>      Identity service URL [env: IDENTITY_SERVICE_URL]
  --port <PORT>             Port to listen on [default: 3000]
```

## API Endpoints

### Identity Server

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/validate` | Validate API key, returns user info |

**Request:**
```json
{ "api_key": "sk_xxxxxxxx_yyyyyyyyyyyyyyyyyyyyyyyyyyyy" }
```

**Response (valid):**
```json
{
  "valid": true,
  "user": {
    "id": 1,
    "email": "admin@example.com",
    "role": "admin",
    "subscription_status": "active"
  }
}
```

### Support Server

#### User Endpoints (any valid API key)

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/tickets` | Create a ticket (multipart: description + zip) |
| GET | `/tickets` | List own tickets |
| GET | `/tickets/{id}` | Get ticket details (own tickets only) |

#### Admin Endpoints (requires admin role)

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/admin/tickets` | List all tickets |
| GET | `/admin/tickets/{id}` | Get any ticket details |
| PUT | `/admin/tickets/{id}/state` | Update ticket state |
| POST | `/admin/tickets/{id}/comments` | Add comment to ticket |
| GET | `/admin/tickets/{id}/zip` | Download ticket attachments |

## API Key Management

### Key Format

Keys follow the format: `sk_<8-char-prefix>_<32-char-random>`

Example: `sk_qnULokmO_C4nvQn6ZKSZU5nXSddpS9IDQHQGXYmYw`

The prefix (`sk_qnULokmO`) can be used to identify keys without exposing the full key.

### Creating Keys

```bash
# Create a user first
cargo run -p identity-server -- create-user --email user@example.com --role support

# Create a key for the user (note the user ID from previous command)
cargo run -p identity-server -- create-key --user-id 1
```

**Important:** The full key is only displayed once. Store it securely.

### Revoking Keys

```bash
# Revoke by prefix
cargo run -p identity-server -- revoke-key --prefix sk_qnULokmO
```

### Listing Keys

```bash
cargo run -p identity-server -- list-keys
```

Output shows prefix, user, and status (active/revoked) - never the full key.

## Security Notes

- **Database encryption:** Both services use SQLCipher for at-rest encryption
- **Key hashing:** API keys are stored as SHA-256 hashes, not plaintext
- **Network security:** In production, identity-server should only be accessible from support-server (internal network)
- **Encryption keys:** Use strong, unique encryption keys for each database
- **Key rotation:** Create new keys and revoke old ones; existing keys cannot be recovered

## Development

### Running Tests

```bash
cargo test
```

### Building for Release

```bash
cargo build --release
```

Binaries will be in `target/release/`.
