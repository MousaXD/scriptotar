import { describe, expect, it } from 'vitest';
import { observedInteractionRate } from './researchMetrics';

describe('observed creator interaction rate', () => {
  it('uses both known interaction counts and a positive observed view count', () => {
    expect(observedInteractionRate({ views: 200, likes: 10, comments: 2 })).toBe(0.06);
    expect(observedInteractionRate({ views: 100, likes: 0, comments: 0 })).toBe(0);
  });

  it('never silently treats missing, negative, or invalid metrics as zero', () => {
    expect(observedInteractionRate({ views: 100, likes: 2 })).toBeNull();
    expect(observedInteractionRate({ views: 0, likes: 1, comments: 1 })).toBeNull();
    expect(observedInteractionRate({ views: -3, likes: 1, comments: 1 })).toBeNull();
    expect(observedInteractionRate({ views: 100, likes: -1, comments: 1 })).toBeNull();
    expect(observedInteractionRate({ views: Infinity, likes: 1, comments: 1 })).toBeNull();
  });
});
