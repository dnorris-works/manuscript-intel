import { Router, Request, Response } from 'express';
import { requireAuth } from '../middleware/clerk-auth.js';
import { z } from 'zod';

const router = Router();

// Validate usage report payload
const UsageReportSchema = z.object({
  tokensConsumed: z.number().positive(),
  analysisType: z.string(),
  deviceId: z.string(),
  status: z.enum(['success', 'failed']).default('success')
});

type UsageReport = z.infer<typeof UsageReportSchema>;

/**
 * POST /api/usage/report
 * Report token consumption from desktop application
 */
router.post('/report', requireAuth, (req: Request, res: Response) => {
  try {
    const userId = req.auth?.userId;

    if (!userId) {
      return res.status(401).json({ error: 'No user ID found' });
    }

    // Validate request body
    const payload = UsageReportSchema.parse(req.body);

    // TODO: Database operations
    // - Insert usage event record
    // - Update user's tokensUsedThisMonth
    // - Check if quota exceeded
    // - Return confirmation

    console.log(`Usage reported - User: ${userId}, Tokens: ${payload.tokensConsumed}`);

    res.json({
      success: true,
      tokensDeducted: payload.tokensConsumed,
      message: 'Usage recorded - DB integration pending'
    });
  } catch (error) {
    if (error instanceof z.ZodError) {
      return res.status(400).json({ error: 'Invalid request body', details: error.errors });
    }

    console.error('Usage report error:', error);
    res.status(500).json({ error: 'Internal server error' });
  }
});

/**
 * GET /api/usage/history
 * Get user's usage history and current quota
 */
router.get('/history', requireAuth, (req: Request, res: Response) => {
  try {
    const userId = req.auth?.userId;

    if (!userId) {
      return res.status(401).json({ error: 'No user ID found' });
    }

    // TODO: Query database
    // - Get user's current subscription
    // - Get usage events for current period
    // - Calculate tokens remaining
    // - Return formatted history

    res.json({
      plan: 'pro',
      tokensThisMonth: 50000,
      tokensRemaining: 950000,
      periodStart: new Date(),
      periodEnd: new Date(),
      recentUsage: [],
      message: 'History retrieval - DB integration pending'
    });
  } catch (error) {
    console.error('Usage history error:', error);
    res.status(500).json({ error: 'Internal server error' });
  }
});

export default router;
