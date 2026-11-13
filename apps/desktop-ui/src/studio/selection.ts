import type { ResearchItem, TranscriptSummary } from '../types';
import type { StudySourceRef } from './contract';

/** Stable IDs only; reject unavailable sources before sending a mutation. */
export function sanitizeSelection(
  refs: StudySourceRef[],
  research: ResearchItem[],
  transcripts: TranscriptSummary[],
  projectId: string
): StudySourceRef[] {
  const seen = new Set<string>();
  const allowedResearch = new Set(research.map((item) => item.id));
  const allowedTranscripts = new Set(transcripts.filter((item) => item.projectId === projectId).map((item) => item.id));
  return refs.filter((ref) => {
    const key = ref.kind + ':' + ref.id;
    if (!ref.id || seen.has(key)) return false;
    if (ref.kind === 'research' && !allowedResearch.has(ref.id)) return false;
    if (ref.kind === 'transcript' && !allowedTranscripts.has(ref.id)) return false;
    seen.add(key);
    return true;
  });
}

export function sourceLabel(ref: StudySourceRef, research: ResearchItem[], transcripts: TranscriptSummary[]): string {
  if (ref.kind === 'research') return research.find((item) => item.id === ref.id)?.title || ref.id;
  return transcripts.find((item) => item.id === ref.id)?.title || ref.id;
}

/** Source attachments form an identity set, not a positional list. SQLite
 * orders equal-timestamp source rows by UUID, which must not cause resaves. */
export function sameRefs(a: StudySourceRef[], b: StudySourceRef[]): boolean {
  if (a.length !== b.length) return false;
  const seen = new Set(a.map((ref) => ref.kind + ':' + ref.id));
  return seen.size === a.length && b.every((ref) => seen.has(ref.kind + ':' + ref.id));
}
