# Web API - Database Migrations

This folder will contain SQL migration files for the Loremetry Web API database.

## Migration Strategy

- Use a migration tool like Flyway, Liquibase, or TypeORM migrations
- Each migration file should be numbered/dated: `001-create-users.sql`, `002-create-usage.sql`, etc.
- Migrations are applied in order on startup or via CLI

## Planned Tables

See `src/db/schema.ts` for TypeScript type definitions that correspond to these tables:

- `users` - Clerk user data + subscription info
- `usage_events` - Token consumption tracking
- `subscription_plans` - Plan definitions and pricing

