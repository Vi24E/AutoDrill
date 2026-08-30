import type { ReactNode } from 'react';
import { MathLiveStatic } from '@/components/MathLiveMath';
import { arithmeticLeafText, liarPersonLabel, liarStatementText, problemExpression } from '@/domain/problem-format';
import { problemExpressionLatex } from '@/domain/mathlive-format';
import { answerNodeText, type AnswerNode, type ArithmeticOperator, type ProblemDto } from '@/domain/drill-engine';


function ColumnGridCharacters({ text }: { text: string }) {
  return (
    <>
      {[...text].map((character, index) => character === '.' ? (
        <span className="column-arithmetic-decimal-marker" key={`decimal-${index}`} aria-hidden="true" />
      ) : (
        <span
          className={`column-arithmetic-digit-cell ${character === ' ' ? 'column-arithmetic-digit-cell-empty' : ''}`}
          key={`${character}-${index}`}
          aria-hidden={character === ' '}
        >
          {character === ' ' ? '\u00a0' : character}
        </span>
      ))}
    </>
  );
}

function ColumnGridValue({ text, className = '' }: { text: string; className?: string }) {
  return (
    <span className={`column-arithmetic-value ${className}`.trim()}>
      <ColumnGridCharacters text={text} />
    </span>
  );
}

function splitDecimal(text: string): { whole: string; fraction: string | null } {
  const dot = text.indexOf('.');
  return dot < 0
    ? { whole: text, fraction: null }
    : { whole: text.slice(0, dot), fraction: text.slice(dot + 1) };
}


function LongDivisionBracket({ children }: { children: ReactNode }) {
  return (
    <span className="column-division-bracket">
      <svg className="column-division-bracket-mark" viewBox="0 0 100 28" preserveAspectRatio="none" aria-hidden="true" focusable="false">
        <path d="M 0 28 C 7 21 7 7 0 0 L 100 0" />
      </svg>
      {children}
    </span>
  );
}
function AlignedColumnValue({ text, wholeWidth, fractionWidth }: { text: string; wholeWidth: number; fractionWidth: number }) {
  const { whole, fraction } = splitDecimal(text);
  const aligned = `${' '.repeat(Math.max(0, wholeWidth - whole.length))}${whole}${fraction === null ? ' '.repeat(fractionWidth) : `.${fraction.padEnd(fractionWidth, ' ')}`}`;
  return <ColumnGridValue text={aligned} className="column-arithmetic-value-decimal" />;
}


function formatScaledDigits(coefficient: bigint, scale: number): string {
  const negative = coefficient < 0n;
  const digits = (negative ? -coefficient : coefficient).toString().padStart(scale + 1, '0');
  if (scale === 0) return `${negative ? '−' : ''}${digits}`;
  const split = digits.length - scale;
  return `${negative ? '−' : ''}${digits.slice(0, split)}.${digits.slice(split)}`;
}

function answerScalarText(answer: AnswerNode): string {
  if (answer.type === 'tuple') return answer.value[0] ? answerNodeText(answer.value[0]) : '';
  return answerNodeText(answer);
}

function ColumnAddSubtractSolution({ problem }: { problem: ProblemDto }) {
  if (problem.prompt.kind !== 'column_arithmetic') return null;
  const { operator, left, right } = problem.prompt;
  const leftText = arithmeticLeafText(left);
  const rightText = arithmeticLeafText(right);
  const answerText = answerScalarText(problem.canonical_answer);
  const parts = [leftText, rightText, answerText].map(splitDecimal);
  const alignDecimal = parts.some((part) => part.fraction !== null);
  const wholeWidth = Math.max(...parts.map((part) => part.whole.length));
  const fractionWidth = Math.max(...parts.map((part) => part.fraction?.length ?? 0));
  const value = (text: string) => alignDecimal
    ? <AlignedColumnValue text={text} wholeWidth={wholeWidth} fractionWidth={fractionWidth} />
    : <ColumnGridValue text={text} />;
  return (
    <span className={`column-arithmetic column-arithmetic-${operator} column-arithmetic-solution`} data-column-arithmetic={operator} data-column-solution="true">
      <span className="column-arithmetic-row"><span className="column-arithmetic-operator-placeholder" />{value(leftText)}</span>
      <span className="column-arithmetic-row"><span className="column-arithmetic-operator">{operator === 'add' ? '＋' : '−'}</span>{value(rightText)}</span>
      <span className="column-arithmetic-rule" />
      <span className="column-arithmetic-row column-arithmetic-solution-result"><span className="column-arithmetic-operator-placeholder" />{value(answerText)}</span>
    </span>
  );
}

function ColumnMultiplySolution({ problem }: { problem: ProblemDto }) {
  if (problem.prompt.kind !== 'column_arithmetic' || problem.prompt.operator !== 'multiply') return null;
  const leftText = arithmeticLeafText(problem.prompt.left);
  const rightText = arithmeticLeafText(problem.prompt.right);
  const worked = problem.worked_solution?.kind === 'column_multiplication' ? problem.worked_solution : null;
  const partials = worked?.partial_products ?? [];
  const answerText = answerScalarText(problem.canonical_answer);
  return (
    <span className="column-arithmetic column-arithmetic-multiply column-arithmetic-solution column-arithmetic-multiply-solution" data-column-arithmetic="multiply" data-column-solution="true">
      <span className="column-arithmetic-row"><span className="column-arithmetic-operator-placeholder" /><ColumnGridValue text={leftText} /></span>
      <span className="column-arithmetic-row"><span className="column-arithmetic-operator">×</span><ColumnGridValue text={rightText} /></span>
      <span className="column-arithmetic-rule" />
      {partials.length <= 1 ? (
        <span className="column-arithmetic-row column-arithmetic-solution-result"><span className="column-arithmetic-operator-placeholder" /><ColumnGridValue text={answerText} /></span>
      ) : (
        <>
          <span className="column-multiply-partials">
            {partials.map((partial, index) => (
              <span className="column-multiply-partial" style={{ paddingRight: `calc(${partial.place} * var(--column-digit-cell))` }} key={`${problem.problem_id}-partial-${index}`}><ColumnGridCharacters text={String(partial.value)} /></span>
            ))}
          </span>
          <span className="column-arithmetic-final-rule" />
          <span className="column-arithmetic-row column-arithmetic-solution-result"><span className="column-arithmetic-operator-placeholder" /><ColumnGridValue text={answerText} /></span>
        </>
      )}
    </span>
  );
}

type LongDivisionStep = { product: string; after: string; productOffset: number; afterOffset: number; ruleCells: number };

function quotientAnswer(problem: ProblemDto): AnswerNode {
  return problem.canonical_answer.type === 'tuple' ? (problem.canonical_answer.value[0] ?? { type: 'empty' }) : problem.canonical_answer;
}


function alignLongDivisionPartial(text: string, rawRightOffset: number, targetScale: number): { text: string; visualRightOffset: number } {
  if (targetScale <= 0 || rawRightOffset >= targetScale) return { text, visualRightOffset: rawRightOffset };
  const fractionalCellsInside = targetScale - rawRightOffset;
  if (fractionalCellsInside < text.length) {
    const split = text.length - fractionalCellsInside;
    return { text: `${text.slice(0, split)}.${text.slice(split)}`, visualRightOffset: rawRightOffset };
  }
  return { text, visualRightOffset: rawRightOffset };
}

function columnGridCellCount(text: string): number {
  return [...text].filter((character) => character !== '.').length;
}

function buildLongDivision(problem: ProblemDto): { divisorText: string; dividendText: string; quotientText: string; steps: LongDivisionStep[] } {
  if (problem.prompt.kind !== 'column_arithmetic' || problem.prompt.operator !== 'divide') throw new Error('Long division requires a division prompt.');
  const worked = problem.worked_solution;
  if (!worked || worked.kind !== 'long_division') {
    return {
      divisorText: arithmeticLeafText(problem.prompt.right),
      dividendText: arithmeticLeafText(problem.prompt.left),
      quotientText: answerNodeText(quotientAnswer(problem)),
      steps: [],
    };
  }
  const quotient = quotientAnswer(problem);
  const steps = worked.steps.map((step) => {
    const alignedProduct = alignLongDivisionPartial(String(step.product), step.product_offset, worked.dividend_scale);
    const alignedAfter = alignLongDivisionPartial(String(step.after), step.after_offset, worked.dividend_scale);
    return {
      product: alignedProduct.text,
      after: alignedAfter.text,
      productOffset: alignedProduct.visualRightOffset,
      afterOffset: alignedAfter.visualRightOffset,
      ruleCells: Math.max(2, columnGridCellCount(alignedProduct.text), columnGridCellCount(alignedAfter.text)),
    };
  });
  return {
    divisorText: String(worked.divisor),
    dividendText: formatScaledDigits(BigInt(worked.dividend_coefficient), worked.dividend_scale),
    quotientText: `${answerNodeText(quotient)}${' '.repeat(worked.quotient_trailing_cells)}`,
    steps,
  };
}

function ColumnDivideSolution({ problem }: { problem: ProblemDto }) {
  const solution = buildLongDivision(problem);
  return (
    <span className="column-arithmetic column-arithmetic-division column-arithmetic-solution column-division-solution" data-column-arithmetic="division" data-column-solution="true">
      <span className="column-division-solution-grid">
        <span className="column-division-solution-quotient-spacer" aria-hidden="true" />
        <span className="column-division-solution-quotient"><ColumnGridCharacters text={solution.quotientText} /></span>
        <span className="column-division-divisor"><ColumnGridCharacters text={solution.divisorText} /></span>
        <span className="column-division-solution-right">
          <LongDivisionBracket><span className="column-division-dividend"><ColumnGridCharacters text={solution.dividendText} /></span></LongDivisionBracket>
          <span className="column-division-solution-work">
            {solution.steps.map((step, index) => (
              <span className="column-division-solution-step" key={`${problem.problem_id}-division-step-${index}`}>
                <span className="column-division-solution-product">
                  <span className="column-division-solution-product-value" style={{ marginRight: `calc(${step.productOffset} * var(--column-digit-cell))` }}><ColumnGridCharacters text={step.product} /></span>
                </span>
                <span className="column-division-solution-rule" style={{ width: `calc(${step.ruleCells} * var(--column-digit-cell))`, marginRight: `calc(${step.productOffset} * var(--column-digit-cell))` }} />
                <span className="column-division-solution-after"><span style={{ marginRight: `calc(${step.afterOffset} * var(--column-digit-cell))` }}><ColumnGridCharacters text={step.after} /></span></span>
              </span>
            ))}
          </span>
        </span>
      </span>
    </span>
  );
}

function ColumnArithmeticExpression({ problem, solution = false }: { problem: ProblemDto; solution?: boolean }) {
  if (problem.prompt.kind !== 'column_arithmetic') return null;
  const { operator, left, right } = problem.prompt;
  if (solution) {
    if (operator === 'add' || operator === 'subtract') return <ColumnAddSubtractSolution problem={problem} />;
    if (operator === 'multiply') return <ColumnMultiplySolution problem={problem} />;
    return <ColumnDivideSolution problem={problem} />;
  }
  const leftText = arithmeticLeafText(left);
  const rightText = arithmeticLeafText(right);
  const ariaLabel = problemExpression(problem, false);
  if (operator === 'divide') {
    return (
      <span className="column-arithmetic column-arithmetic-division" aria-label={ariaLabel} data-column-arithmetic="division">
        <span className="column-division-body"><span className="column-division-divisor"><ColumnGridCharacters text={rightText} /></span><LongDivisionBracket><span className="column-division-dividend"><ColumnGridCharacters text={leftText} /></span></LongDivisionBracket></span>
        <span className="column-division-work-space" aria-hidden="true" />
      </span>
    );
  }
  const leftParts = splitDecimal(leftText);
  const rightParts = splitDecimal(rightText);
  const alignDecimal = (operator === 'add' || operator === 'subtract') && (leftParts.fraction !== null || rightParts.fraction !== null);
  const wholeWidth = Math.max(leftParts.whole.length, rightParts.whole.length);
  const fractionWidth = Math.max(leftParts.fraction?.length ?? 0, rightParts.fraction?.length ?? 0);
  const operatorText: Record<Exclude<ArithmeticOperator, 'divide'>, string> = { add: '＋', subtract: '−', multiply: '×' };
  const value = (text: string) => alignDecimal ? <AlignedColumnValue text={text} wholeWidth={wholeWidth} fractionWidth={fractionWidth} /> : <ColumnGridValue text={text} />;
  return (
    <span className={`column-arithmetic column-arithmetic-${operator}`} aria-label={ariaLabel} data-column-arithmetic={operator}>
      <span className="column-arithmetic-row column-arithmetic-row-top"><span className="column-arithmetic-operator-placeholder" aria-hidden="true" />{value(leftText)}</span>
      <span className="column-arithmetic-row column-arithmetic-row-bottom"><span className="column-arithmetic-operator" aria-hidden="true">{operatorText[operator]}</span>{value(rightText)}</span>
      <span className="column-arithmetic-rule" aria-hidden="true" />
    </span>
  );
}

export function ProblemExpression({ problem, includeAnswerEquals = true, solution = false }: { problem: ProblemDto; includeAnswerEquals?: boolean; solution?: boolean }) {
  if (problem.prompt.kind === 'liar_puzzle') {
    return (
      <span className="liar-statements" aria-label={problemExpression(problem, false)}>
        {problem.prompt.statements.map((statement, index) => <span className="liar-statement-row" key={`${problem.problem_id}-statement-${index}`}><strong>{liarPersonLabel(index + 1)}さん：</strong>{liarStatementText(statement)}</span>)}
      </span>
    );
  }
  if (problem.prompt.kind === 'column_arithmetic') return <ColumnArithmeticExpression problem={problem} solution={solution} />;
  return <MathLiveStatic className="problem-math-expression" latex={problemExpressionLatex(problem, includeAnswerEquals)} ariaLabel={problemExpression(problem, includeAnswerEquals)} />;
}
