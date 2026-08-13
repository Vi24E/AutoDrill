import { MathLiveStatic } from '@/components/MathLiveMath';
import { liarPersonLabel, liarStatementText, problemExpression } from '@/domain/problem-format';
import { problemExpressionLatex } from '@/domain/mathlive-format';
import type { ProblemDto } from '@/domain/drill-engine';

export function ProblemExpression({ problem, includeAnswerEquals = true }: { problem: ProblemDto; includeAnswerEquals?: boolean }) {
  if (problem.prompt.kind === 'liar_puzzle') {
    return (
      <span className="liar-statements" aria-label={problemExpression(problem, false)}>
        {problem.prompt.statements.map((statement, index) => (
          <span className="liar-statement-row" key={`${problem.problem_id}-statement-${index}`}>
            <strong>{liarPersonLabel(index + 1)}さん：</strong>{liarStatementText(statement)}
          </span>
        ))}
      </span>
    );
  }
  return (
    <MathLiveStatic
      className="problem-math-expression"
      latex={problemExpressionLatex(problem, includeAnswerEquals)}
      ariaLabel={problemExpression(problem, includeAnswerEquals)}
    />
  );
}
