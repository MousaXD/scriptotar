import { describe, expect, it } from 'vitest';
import type { StudyEvidence } from './contract';
import { evidencePromptContext } from './evidencePrompt';

const observed: StudyEvidence = {
  id: 'evidence-1', projectId: 'project-1', studyId: 'study-1',
  source: { kind: 'transcript', id: 'transcript-1' }, sourceAvailable: true,
  sourceLabel: 'transcript', claim: 'The speaker uses a question in the opening',
  locator: '0:00–0:07', timestampSeconds: 0, certainty: 'unverified',
  uncertaintyNote: 'Review the full context', createdAt: '2026-11-15T00:00:00Z'
};

describe('Study evidence → local writing prompt', () => {
  it('retains exact selected source references and explicit non-verification', () => {
    const text = evidencePromptContext([observed], 'project-1', 'study-1',
      [{ kind: 'transcript', id: 'transcript-1' }]);
    expect(text).toContain('NOT INDEPENDENTLY FACT-CHECKED');
    expect(text).toContain('0:00–0:07');
    expect(text).toContain('The speaker uses a question');
    expect(text).toContain('"recorded_certainty":"unverified"');
    expect(text).toContain('"source_id":"transcript-1"');
  });

  it('refuses foreign Study results, omitted origins, and unavailable source content', () => {
    expect(() => evidencePromptContext([{ ...observed, projectId: 'foreign' }],
      'project-1', 'study-1', [observed.source!])).toThrow('another project');
    expect(evidencePromptContext([observed], 'project-1', 'study-1', [] )).toBe('');
    expect(evidencePromptContext([{ ...observed, sourceAvailable: false }],
      'project-1', 'study-1', [observed.source!])).toBe('');
    expect(evidencePromptContext([observed], 'project-1', 'study-1',
      [{ kind: 'transcript', id: 'another-transcript' }])).toBe('');
  });

  it('never puts disputed claims into a writing prompt', () => {
    expect(evidencePromptContext([{ ...observed, certainty: 'disputed',
      uncertaintyNote: 'Contradicted by the original recording' }],
      'project-1', 'study-1', [observed.source!])).toBe('');
    const mixed = evidencePromptContext([
      { ...observed, id: 'e-bad', claim: 'DISPUTED FALSEHOOD', certainty: 'disputed' },
      observed
    ], 'project-1', 'study-1', [observed.source!]);
    expect(mixed).not.toContain('DISPUTED FALSEHOOD');
    expect(mixed).toContain('The speaker uses a question');
  });

  it('caps creator-supplied material and never upgrades a certainty label', () => {
    const many = Array.from({ length: 40 }, (_, i) => ({
      ...observed, id: 'evidence-' + i, claim: 'x'.repeat(2000),
      certainty: 'corroborated' as const
    }));
    const text = evidencePromptContext(many, 'project-1', 'study-1', [observed.source!]);
    expect(text).toContain('recorded certainty labels');
    expect(text).not.toContain('"evidence_id":"evidence-16"');
    expect(text).not.toContain('x'.repeat(651));
    expect(text).toContain('"recorded_certainty":"corroborated"');
  });
});
