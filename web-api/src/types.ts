import { Request } from 'express';

/**
 * Extend Express Request type to include Clerk auth
 */
declare global {
  namespace Express {
    interface Request {
      auth?: {
        userId?: string;
        sessionId?: string;
        sessionClaims?: Record<string, any>;
      };
    }
  }
}

export {};
