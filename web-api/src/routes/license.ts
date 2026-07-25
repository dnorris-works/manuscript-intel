import { Router, Request, Response } from 'express';
import { requireAuth } from '../middleware/clerk-auth.js';

const router = Router();

/**
 * POST /api/license/check
 * Verify if user has valid license and check token quota
 * Returns: { valid: boolean, tokensRemaining: number, plan: string }
 */
router.post('/check', requireAuth, (req: Request, res: Response) => {
  try {
    const userId = req.auth?.userId;

    if (!userId) {
      return res.status(401).json({ error: 'No user ID found' });
    }

    // TODO: Query database for user's subscription
    // - Check if user exists
    // - Get subscription plan
    // - Calculate tokens remaining
    // - Return quota status

    // Temporary placeholder response
    res.json({
      valid: true,
      tokensRemaining: 950000,
      plan: 'pro',
      message: 'License valid - DB integration pending'
    });
  } catch (error) {
    console.error('License check error:', error);
    res.status(500).json({ error: 'Internal server error' });
  }
});

export default router;
