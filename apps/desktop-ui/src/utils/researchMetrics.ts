import type { ResearchItem } from '../types';

/**
 * Observable interactions per view, not an engagement forecast or unique-person rate.
 * Unknown or invalid source values must never become invented zero counts.
 */
export function observedInteractionRate(item: Pick<ResearchItem, 'views' | 'likes' | 'comments'>): number | null {
  const { views, likes, comments } = item;
  if (
    typeof views !== 'number' || !Number.isFinite(views) || views <= 0 ||
    typeof likes !== 'number' || !Number.isFinite(likes) || likes < 0 ||
    typeof comments !== 'number' || !Number.isFinite(comments) || comments < 0
  ) return null;
  return (likes + comments) / views;
}
