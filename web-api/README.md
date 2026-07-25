# Loremetry Web API

The Web API handles:
- User authentication via Clerk
- License/subscription validation
- Token usage tracking and billing

## Quick Start

```bash
cd web-api

# Install dependencies
npm install

# Setup environment
cp .env.example .env
# Edit .env with your Clerk keys and database URL

# Development
npm run dev

# Build for production
npm run build
npm start
```

## API Endpoints

### Authentication
- `GET /api/auth/me` - Get current user
- `POST /api/auth/verify` - Verify token

### Licensing
- `POST /api/license/check` - Check license validity and token quota

### Usage Tracking
- `POST /api/usage/report` - Report token consumption
- `GET /api/usage/history` - Get usage history

## Environment Variables

See `.env.example` for required configuration.

## Database

Migrations are in the `migrations/` folder. Each table structure is defined in `src/db/schema.ts`.
