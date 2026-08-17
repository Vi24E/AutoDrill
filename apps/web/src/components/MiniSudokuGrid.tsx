import type { CSSProperties } from 'react';

import { digitGridValues } from '@/domain/digit-grid-input';
import type { AnswerNode, ProblemDto } from '@/domain/drill-engine';

const MINI_SUDOKU_BLOCK_SIDE = 2;
const MINI_SUDOKU_PAGE_GRID_CELL_SPAN = 2;

type MiniSudokuGridProps = {
  problem: ProblemDto;
  answer?: AnswerNode;
  selectedCell?: number | null;
  readOnly?: boolean;
  correctionAnswer?: AnswerNode | null;
  onSelectCell?: (cellIndex: number) => void;
};

export function MiniSudokuGrid({
  problem,
  answer,
  selectedCell = null,
  readOnly = false,
  correctionAnswer = null,
  onSelectCell,
}: MiniSudokuGridProps) {
  if (problem.prompt.kind !== 'mini_sudoku' || problem.input_interface.type !== 'digit_grid') {
    throw new Error('MiniSudokuGrid requires a digit-grid puzzle problem.');
  }
  const input = problem.input_interface;
  const values = answer ? digitGridValues(answer, input) : Array.from({ length: input.cell_count }, () => null);
  const correction = correctionAnswer ? digitGridValues(correctionAnswer, input) : null;
  const side = Math.sqrt(input.cell_count);
  if (!Number.isInteger(side)) throw new Error('Digit-grid cell count must form a square.');

  return (
    <span
      className="digit-grid-answer mini-sudoku-grid"
      style={{ '--digit-grid-side': side, '--digit-grid-cell-span': MINI_SUDOKU_PAGE_GRID_CELL_SPAN } as CSSProperties}
      data-digit-grid-cells={input.cell_count}
      aria-label="4かける4の数独"
    >
      {problem.prompt.givens.map((given, cellIndex) => {
        const value = given ?? values[cellIndex];
        const correctionValue = given === null || given === undefined ? correction?.[cellIndex] ?? null : null;
        const needsCorrection = correctionValue !== null && correctionValue !== value;
        const className = [
          'digit-grid-cell',
          given !== null && given !== undefined ? 'digit-grid-cell-given' : 'digit-grid-cell-answer',
          selectedCell === cellIndex ? 'digit-grid-cell-selected' : '',
          needsCorrection ? 'digit-grid-cell-wrong' : '',
          cellIndex % side === MINI_SUDOKU_BLOCK_SIDE - 1 ? 'digit-grid-cell-block-right' : '',
          Math.floor(cellIndex / side) === MINI_SUDOKU_BLOCK_SIDE - 1 ? 'digit-grid-cell-block-bottom' : '',
        ].filter(Boolean).join(' ');
        const content = (
          <>
            <span className="digit-grid-cell-value">{value ?? ''}</span>
            {needsCorrection ? <span className="digit-grid-cell-correction">{correctionValue}</span> : null}
          </>
        );
        if (given !== null && given !== undefined || readOnly) {
          return (
            <span className={className} data-digit-grid-cell={cellIndex} key={cellIndex}>
              {content}
            </span>
          );
        }
        return (
          <button
            type="button"
            className={className}
            data-digit-grid-cell={cellIndex}
            aria-label={`${cellIndex + 1}番目のマス ${value ?? '未入力'}`}
            aria-pressed={selectedCell === cellIndex}
            onClick={() => onSelectCell?.(cellIndex)}
            key={cellIndex}
          >
            {content}
          </button>
        );
      })}
    </span>
  );
}
