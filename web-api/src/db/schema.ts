/**
 * Database schema types for Loremetry
 * Actual migrations/SQL files should be in /migrations
 */

export interface User {
  id: string; // Clerk user ID
  email: string;
  subscriptionPlan: string; // 'free' | 'pro' | 'enterprise'
  monthlyTokenQuota: number; // Tokens allowed per month
  tokensUsedThisMonth: number; // Current usage
  currentPeriodStart: Date;
  currentPeriodEnd: Date;
  createdAt: Date;
  updatedAt: Date;
}

export interface UsageEvent {
  id: string;
  userId: string; // Clerk user ID
  deviceId: string; // Desktop device identifier
  analysisType: string; // 'chapter_summary', 'zeigarnik', etc.
  tokensConsumed: number;
  status: 'success' | 'failed';
  timestamp: Date;
}

export interface SubscriptionPlan {
  id: string;
  name: string; // 'Free', 'Pro', 'Enterprise'
  monthlyTokens: number;
  pricePerMonth: number;
  maxDevices: number;
  features: string[];
}
