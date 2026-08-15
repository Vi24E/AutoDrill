import type { ReactNode } from 'react';
import { MathLiveStatic } from '@/components/MathLiveMath';
import { arithmeticLeafText, liarPersonLabel, liarStatementText, problemExpression } from '@/domain/problem-format';
import { problemExpressionLatex } from '@/domain/mathlive-format';
import { columnDivisionTargetScale } from '@/domain/column-arithmetic-presentation';
import { answerNodeText, type AnswerNode, type ArithmeticExpression, type ArithmeticOperator, type ProblemDto } from '@/domain/drill-engine';


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
      <svg className="column-division-bracket-mark" viewBox="0 0 10 28" preserveAspectRatio="none" aria-hidden="true" focusable="false">
        <path d="M 0 0 C 6.5 5 8.5 18 1.2 28" />
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

function leafScaledInteger(expression: ArithmeticExpression): { coefficient: bigint; scale: number } {
  if (expression.kind === 'integer') return { coefficient: BigInt(expression.value), scale: 0 };
  if (expression.kind === 'exact_decimal') return { coefficient: BigInt(expression.coefficient), scale: expression.scale };
  throw new Error(`Column arithmetic solution requires a numeric leaf, received ${expression.kind}.`);
}

function pow10(power: number): bigint { return 10n ** BigInt(power); }

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
  const left = leafScaledInteger(problem.prompt.left);
  const right = leafScaledInteger(problem.prompt.right);
  const multiplicand = left.coefficient < 0n ? -left.coefficient : left.coefficient;
  const multiplierDigits = (right.coefficient < 0n ? -right.coefficient : right.coefficient).toString();
  const partials = [...multiplierDigits].reverse().map((digit, place) => ({ text: (multiplicand * BigInt(digit)).toString(), place }));
  const answerText = answerScalarText(problem.canonical_answer);
  return (
    <span className="column-arithmetic column-arithmetic-multiply column-arithmetic-solution column-arithmetic-multiply-solution" data-column-arithmetic="multiply" data-column-solution="true">
      <span className="column-arithmetic-row"><span className="column-arithmetic-operator-placeholder" /><ColumnGridValue text={leftText} /></span>
      <span className="column-arithmetic-row"><span className="column-arithmetic-operator">×</span><ColumnGridValue text={rightText} /></span>
      <span className="column-arithmetic-rule" />
      {partials.length === 1 ? (
        <span className="column-arithmetic-row column-arithmetic-solution-result"><span className="column-arithmetic-operator-placeholder" /><ColumnGridValue text={answerText} /></span>
      ) : (
        <>
          <span className="column-multiply-partials">
            {partials.map((partial, index) => (
              <span className="column-multiply-partial" style={{ paddingRight: `calc(${partial.place} * var(--column-digit-cell))` }} key={`${problem.problem_id}-partial-${index}`}><ColumnGridCharacters text={partial.text} /></span>
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

function quotientScale(answer: AnswerNode): number { return answer.type === 'exact_decimal' ? answer.value.scale : 0; }

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
  const left = leafScaledInteger(problem.prompt.left);
  const right = leafScaledInteger(problem.prompt.right);
  const quotient = quotientAnswer(problem);
  const targetQuotientScale = quotientScale(quotient);
  let normalizedDividendCoefficient = left.coefficient;
  let normalizedDividendScale: number;
  if (right.scale <= left.scale) normalizedDividendScale = left.scale - right.scale;
  else {
    normalizedDividendCoefficient *= pow10(right.scale - left.scale);
    normalizedDividendScale = 0;
  }
  const divisor = right.coefficient < 0n ? -right.coefficient : right.coefficient;
  const dividendMagnitude = normalizedDividendCoefficient < 0n ? -normalizedDividendCoefficient : normalizedDividendCoefficient;
  const targetDividendScale = columnDivisionTargetScale(problem);
  const appendedZeros = targetDividendScale - normalizedDividendScale;
  const baseDigits = dividendMagnitude.toString().padStart(normalizedDividendScale + 1, '0');
  const digits = `${baseDigits}${'0'.repeat(appendedZeros)}`;
  const dividendText = formatScaledDigits(dividendMagnitude * pow10(appendedZeros), targetDividendScale);
  const steps: LongDivisionStep[] = [];
  let current = 0n;
  let started = false;
  for (let index = 0; index < digits.length; index += 1) {
    current = current * 10n + BigInt(digits[index]!);
    const q = current / divisor;
    const hasMore = index < digits.length - 1;
    if (!started && q === 0n && hasMore) continue;
    started = true;
    const product = q * divisor;
    const remainder = current - product;
    const rawProductOffset = digits.length - index - 1;
    const after = hasMore ? remainder * 10n + BigInt(digits[index + 1]!) : remainder;
    const rawAfterOffset = hasMore ? Math.max(0, rawProductOffset - 1) : rawProductOffset;
    const alignedProduct = alignLongDivisionPartial(product.toString(), rawProductOffset, targetDividendScale);
    const alignedAfter = alignLongDivisionPartial(after.toString(), rawAfterOffset, targetDividendScale);
    steps.push({
      product: alignedProduct.text,
      after: alignedAfter.text,
      productOffset: alignedProduct.visualRightOffset,
      afterOffset: alignedAfter.visualRightOffset,
      ruleCells: Math.max(2, columnGridCellCount(alignedProduct.text), columnGridCellCount(alignedAfter.text)),
    });
    current = remainder;
  }
  return { divisorText: (right.coefficient < 0n ? -right.coefficient : right.coefficient).toString(), dividendText, quotientText: `${answerNodeText(quotient)}${' '.repeat(Math.max(0, targetDividendScale - targetQuotientScale))}`, steps };
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
                  <span className="column-division-solution-product-value" style={{ marginRight: `calc(${step.productOffset} * var(--column-digit-cell))` }}><span className="column-division-solution-minus">−</span><ColumnGridCharacters text={step.product} /></span>
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
      {operator === 'multiply' ? <span className="column-arithmetic-work-space column-arithmetic-work-space-multiply" aria-hidden="true" /> : null}
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
