import { Request, Response, NextFunction } from 'express';
import { ClerkExpressWithAuth } from '@clerk/express';

/**
 * Middleware to verify Clerk JWT token in Authorization header
 * Expects: Authorization: Bearer <token>
 */
export const clerkAuth = ClerkExpressWithAuth();

/**
 * Extract and verify user ID from Clerk session
 */
export const requireAuth = (req: Request, res: Response, next: NextFunction) => {
  if (!req.auth || !req.auth.userId) {
    return res.status(401).json({ error: 'Unauthorized: No valid Clerk token' });
  }
  next();
};
