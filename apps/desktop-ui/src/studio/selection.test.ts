import { describe, expect, it } from 'vitest';
import { sanitizeSelection, sameRefs } from './selection';
import type { ResearchItem, TranscriptSummary } from '../types';

const research = [{ id: 'r1', title: 'Source' }] as ResearchItem[];
const transcripts = [
  { id: 't1', projectId: 'p1', title: 'One' },
  { id: 't2', projectId: 'p2', title: 'Two' }
] as TranscriptSummary[];

describe('Studio source identity boundary', () => {
  it('rejects foreign project transcripts, unknown IDs, and duplicates', () => {
    expect(sanitizeSelection([
      { kind: 'transcript', id: 't2' }, { kind: 'research', id: 'r1' },
      { kind: 'research', id: 'r1' }, { kind: 'transcript', id: 't1' },
      { kind: 'transcript', id: 'unknown' }
    ], research, transcripts, 'p1')).toEqual([
      { kind: 'research', id: 'r1' }, { kind: 'transcript', id: 't1' }
    ]);
  });
  it('preserves stable ordered selection', () => {
    expect(sameRefs([{ kind: 'transcript', id: 't1' }], [{ kind: 'transcript', id: 't1' }])).toBe(true);
    expect(sameRefs([{ kind: 'research', id: 'r1' }], [{ kind: 'transcript', id: 'r1' }])).toBe(false);
  });
});
