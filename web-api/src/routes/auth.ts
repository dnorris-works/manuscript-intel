import { Router, Request, Response } from 'express';
import { requireAuth } from '../middleware/clerk-auth.js';

const router = Router();

/**
 * GET /api/auth/me
 * Get current authenticated user info
 */
router.get('/me', requireAuth, (req: Request, res: Response) => {
  try {
    const userId = req.auth?.userId;
    const email = req.auth?.sessionClaims?.email as string | undefined;

    res.json({
      userId,
      email,
      authenticated: true
    });
  } catch (error) {
    console.error('Auth info error:', error);
    res.status(500).json({ error: 'Internal server error' });
  }
});

/**
 * POST /api/auth/verify
 * Verify Clerk token validity (simple health check)
 */
router.post('/verify', requireAuth, (req: Request, res: Response) => {
  res.json({ valid: true, userId: req.auth?.userId });
});

export default router;
