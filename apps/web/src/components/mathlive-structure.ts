import type { MathfieldElement } from 'mathlive';

const EMPTY_PLACEHOLDER_RE = /^\\placeholder(?:\[[^\]]*\])?\{\}$/;

function rangeLatex(mathfield: MathfieldElement, start: number, end: number): string {
  return mathfield.getValue([start, end]);
}

function isStructuredRangeLatex(latex: string): boolean {
  return latex.includes('\\frac')
    || latex.includes('\\sqrt')
    || latex.startsWith('-')
    || latex.startsWith('\\pm')
    || latex.includes(',');
}

/**
 * MathLive keeps an empty placeholder alive when deleteBackward is invoked
 * inside it. Select the smallest MathLive structural range containing the
 * active empty placeholder and delete that range. This module imports only the
 * MathLive type, so q1 can retain the compatibility helper without loading the
 * MathLive runtime bundle.
 */
export function deleteEmptyMathLiveStructureBackward(mathfield: MathfieldElement): boolean {
  const position = mathfield.position;
  const lastOffset = mathfield.lastOffset;
  let promptRange: [number, number] | null = null;

  for (let width = 1; width <= lastOffset; width += 1) {
    for (let start = Math.max(0, position - width); start <= position; start += 1) {
      const end = start + width;
      if (end > lastOffset || position < start || position > end) continue;
      if (EMPTY_PLACEHOLDER_RE.test(rangeLatex(mathfield, start, end))) {
        promptRange = [start, end];
        break;
      }
    }
    if (promptRange) break;
  }
  if (!promptRange) return false;

  const [promptStart, promptEnd] = promptRange;
  let structureRange: [number, number] | null = null;
  let structureWidth = Number.POSITIVE_INFINITY;
  for (let start = 0; start <= promptStart; start += 1) {
    for (let end = promptEnd; end <= lastOffset; end += 1) {
      const width = end - start;
      if (width <= 0 || width >= structureWidth) continue;
      const latex = rangeLatex(mathfield, start, end);
      if (!isStructuredRangeLatex(latex)) continue;
      structureRange = [start, end];
      structureWidth = width;
    }
  }
  if (!structureRange) return false;

  mathfield.selection = { ranges: [structureRange], direction: 'none' };
  return mathfield.executeCommand('deleteBackward');
}
