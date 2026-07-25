import './types.js';
import express from 'express';
import cors from 'cors';
import { ClerkExpressWithAuth } from '@clerk/express';
import 'dotenv/config';

// Import routes
import authRoutes from './routes/auth.js';
import licenseRoutes from './routes/license.js';
import usageRoutes from './routes/usage.js';

const app = express();
const PORT = process.env.PORT || 3000;

// ─────────────────────────────────────────────────────────
// Middleware
// ─────────────────────────────────────────────────────────
app.use(cors());
app.use(express.json());
app.use(ClerkExpressWithAuth());

// Health check
app.get('/health', (req, res) => {
  res.json({ status: 'ok', timestamp: new Date().toISOString() });
});

// ─────────────────────────────────────────────────────────
// Routes
// ─────────────────────────────────────────────────────────
app.use('/api/auth', authRoutes);
app.use('/api/license', licenseRoutes);
app.use('/api/usage', usageRoutes);

// 404 handler
app.use((req, res) => {
  res.status(404).json({ error: 'Not found' });
});

// Error handler
app.use((err: any, req: express.Request, res: express.Response, next: express.NextFunction) => {
  console.error('Unhandled error:', err);
  res.status(500).json({ error: 'Internal server error' });
});

// ─────────────────────────────────────────────────────────
// Start server
// ─────────────────────────────────────────────────────────
app.listen(PORT, () => {
  console.log(`🚀 Loremetry Web API running on http://localhost:${PORT}`);
  console.log(`📝 Environment: ${process.env.NODE_ENV || 'development'}`);
  console.log(`🔐 Clerk integration: ${process.env.CLERK_SECRET_KEY ? 'enabled' : 'disabled'}`);
});
