# Loremetry Desktop ↔ Web API Setup Complete

Everything has been set up to connect the desktop app to the Web API for Clerk authentication and usage tracking.

## What Was Created

### Desktop App (Rust/Tauri)
- **`src/api_client.rs`** - HTTP client for Web API calls (license check, usage reporting)
- **`src/api_config.rs`** - Configuration management (dev: localhost:3000, prod: https://api.loremetry.com)
- **`src/api_types.rs`** - Request/response type definitions
- **`src/settings.rs`** - User auth state and device ID storage
- **`src/api_examples.rs`** - Template examples of how to use the API
- **Updated `Cargo.toml`** - Added uuid and reqwest dependencies
- **`API_INTEGRATION.md`** - Comprehensive integration guide

### Web API (Node.js/Express)
- **`web-api/src/index.ts`** - Express server with routes
- **`web-api/src/middleware/clerk-auth.ts`** - Clerk JWT verification
- **`web-api/src/routes/auth.ts`** - /api/auth/* endpoints
- **`web-api/src/routes/license.ts`** - /api/license/check endpoint
- **`web-api/src/routes/usage.ts`** - /api/usage/* endpoints
- **`web-api/src/db/schema.ts`** - Database type definitions
- **`web-api/tsconfig.json`** - TypeScript configuration
- **`web-api/package.json`** - Dependencies and scripts
- **`web-api/.env.example`** - Environment configuration template
- **`web-api/README.md`** - Quick start guide

### Root
- **`package.json`** - Monorepo workspace configuration

## Quick Start

### 1. Set up Web API

```bash
cd web-api

# Copy environment template
cp .env.example .env

# Edit .env with your Clerk keys
# CLERK_SECRET_KEY=your_key_here
# CLERK_PUBLISHABLE_KEY=your_key_here
# DATABASE_URL=postgresql://user:pass@localhost:5432/loremetry

# Install dependencies
npm install

# Start dev server
npm run dev
# API will run on http://localhost:3000
```

### 2. Desktop App automatically connects

The desktop app will:
- **In dev:** Connect to `http://localhost:3000`
- **In production:** Connect to `https://api.loremetry.com` (or `LOREMETRY_API_URL` env var)

### 3. Use API in commands

See `src/api_examples.rs` for template code showing how to:
- Check license before running analysis
- Report token usage after analysis
- Get usage history/quota status

## API Endpoints Ready to Use

### Authentication
- `POST /api/auth/verify` - Verify Clerk token
- `GET /api/auth/me` - Get authenticated user info

### Licensing
- `POST /api/license/check` - Check license validity & token quota

### Usage Tracking
- `POST /api/usage/report` - Report token consumption
- `GET /api/usage/history` - Get usage stats and quota

## Architecture

```
Desktop (Loremetry Desktop)
  ├─ Tauri/Rust backend
  ├─ Vue UI
  └─ Calls Web API when:
     ├─ User launches app (verify auth token)
     ├─ Before running analysis (check license/quota)
     └─ After analysis (report token usage)

Web API (Node.js/Express)
  ├─ Clerk authentication middleware
  ├─ License validation (per user/subscription)
  ├─ Usage tracking (per-token billing)
  └─ Database (PostgreSQL)
     ├─ users table (Clerk ID + subscription)
     ├─ usage_events table (token consumption)
     └─ subscription_plans table (plan definitions)
```

## Next Steps

1. **Database Setup:**
   - Create PostgreSQL database: `loremetry`
   - Create migrations for users, usage_events, subscription_plans tables
   - See `web-api/migrations/README.md`

2. **Integrate into Commands:**
   - Add license checking to your analysis commands
   - Wrap AI calls with token reporting
   - Follow patterns in `src/api_examples.rs`

3. **Frontend Integration:**
   - Add Clerk SDK to Vue UI for login/logout
   - Display token quota to user
   - Show usage stats in settings/dashboard

4. **Testing:**
   - Test dev flow: desktop → localhost:3000 → local DB
   - Test prod flow: desktop → https://api.loremetry.com → prod DB

5. **Deployment:**
   - Deploy web-api via Miget (it pulls from git repo)
   - Build desktop app with production API URL
   - Set up billing/subscription management

## File Locations

```
Manuscript Intel/
├── web-api/
│   ├── src/
│   │   ├── index.ts              ← Main server
│   │   ├── middleware/clerk-auth.ts
│   │   ├── routes/auth.ts
│   │   ├── routes/license.ts
│   │   ├── routes/usage.ts
│   │   ├── db/schema.ts
│   │   └── types.ts
│   ├── migrations/
│   ├── package.json
│   ├── tsconfig.json
│   ├── .env.example
│   └── README.md
│
├── src-tauri/src/
│   ├── api_client.rs             ← HTTP client
│   ├── api_config.rs             ← URL config
│   ├── api_types.rs              ← Request/response types
│   ├── api_examples.rs           ← Usage examples
│   ├── settings.rs               ← Auth state storage
│   ├── lib.rs                    ← Updated module list
│   └── Cargo.toml                ← Updated dependencies
│
├── ui/                           ← Vue frontend
├── package.json                  ← Monorepo config
└── API_INTEGRATION.md            ← This guide
```

## Questions?

Refer to `API_INTEGRATION.md` for detailed integration patterns and troubleshooting.
