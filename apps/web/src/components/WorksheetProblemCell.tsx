import type { ReactNode } from 'react';

import type { ProblemDto } from '@/domain/drill-engine';
import { A4_PAGE, type CellGeometry } from '@/domain/layout';
import { columnArithmeticGridVariables } from '@/domain/column-arithmetic-presentation';
import { miniSudokuGridVariables } from '@/domain/mini-sudoku-presentation';

type ProblemRenderContext = {
  includeAnswerEquals: boolean;
  solution: boolean;
};

type WorksheetProblemCellProps = {
  problem: ProblemDto;
  index: number;
  position: CellGeometry;
  answerPlacement?: 'inline' | 'below';
  equationLayout: boolean;
  mode: 'web' | 'print';
  layoutColumn?: number;
  graded?: boolean;
  showSolution?: boolean;
  problemNumberAdornment?: ReactNode;
  renderExpression: (context: ProblemRenderContext) => ReactNode;
  answer: ReactNode;
  miniSudoku: ReactNode;
};

function toPagePercent(value: number, total: number): string {
  return `${(value / total) * 100}%`;
}

/**
 * Shared Web/print owner for one worksheet problem shell. Family classification,
 * modifier classes, A4 cell projection, grid variables, problem-number ownership,
 * and expression/puzzle branching must not be reimplemented by individual renderers.
 */
export function WorksheetProblemCell({
  problem,
  index,
  position,
  answerPlacement,
  equationLayout,
  mode,
  layoutColumn,
  graded = false,
  showSolution = false,
  problemNumberAdornment,
  renderExpression,
  answer,
  miniSudoku,
}: WorksheetProblemCellProps) {
  const isLiarPuzzle = problem.prompt.kind === 'liar_puzzle';
  const isColumnArithmetic = problem.prompt.kind === 'column_arithmetic';
  const isMiniSudoku = problem.prompt.kind === 'mini_sudoku';
  const stackAnswerBelow = answerPlacement === 'below' && !equationLayout;
  const context: ProblemRenderContext = {
    includeAnswerEquals: !stackAnswerBelow,
    solution: showSolution && isColumnArithmetic,
  };
  const style = {
    left: toPagePercent(position.x, A4_PAGE.width),
    top: toPagePercent(position.y, A4_PAGE.height),
    width: toPagePercent(position.width, A4_PAGE.width),
    height: toPagePercent(position.height, A4_PAGE.height),
    ...(isColumnArithmetic ? columnArithmeticGridVariables(problem, position) : {}),
    ...(isMiniSudoku ? miniSudokuGridVariables(problem, position) : {}),
  };
  const className = [
    'problem-cell',
    mode === 'print' ? 'worksheet-print-problem-cell' : '',
    equationLayout ? 'problem-cell-linear-equation' : '',
    isLiarPuzzle ? 'problem-cell-liar' : '',
    problem.prompt.kind === 'column_arithmetic' ? `problem-cell-column-arithmetic problem-cell-column-arithmetic-${problem.prompt.operator}` : '',
    isMiniSudoku ? 'problem-cell-mini-sudoku' : '',
    stackAnswerBelow ? 'problem-cell-answer-below' : '',
    graded ? 'problem-cell-graded' : '',
  ].filter(Boolean).join(' ');
  const problemNumber = (
    <span className="problem-number-stack">
      {problemNumberAdornment}
      <span className="problem-number">{index + 1}.</span>
    </span>
  );

  return (
    <div
      className={className}
      data-layout-index={mode === 'web' ? index : undefined}
      data-layout-column={mode === 'web' ? layoutColumn : undefined}
      data-problem-index={mode === 'web' ? index : undefined}
      data-print-problem-index={mode === 'print' ? index : undefined}
      data-testid={mode === 'web' ? `problem-cell-${index}` : undefined}
      style={style}
    >
      {!isColumnArithmetic ? problemNumber : null}
      {isMiniSudoku ? miniSudoku : (
        <>
          <span className="expression">
            {isColumnArithmetic ? problemNumber : null}
            {renderExpression(context)}
          </span>
          {context.solution ? null : answer}
        </>
      )}
    </div>
  );
}
