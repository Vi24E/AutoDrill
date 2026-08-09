import type { AnswerInputStructure } from '@/domain/drill-engine';

type MathTemplateIconProps = {
  structure: Exclude<AnswerInputStructure, 'decimal'>;
};

const slot = () => <mtext className="math-template-slot">□</mtext>;

/** Mathematical keypad previews use the same native MathML layout as answers. */
export function MathTemplateIcon({ structure }: MathTemplateIconProps) {
  let content;
  switch (structure) {
    case 'fraction':
      content = <mfrac>{slot()}{slot()}</mfrac>;
      break;
    case 'mixed_fraction':
      content = <mrow>{slot()}<mfrac>{slot()}{slot()}</mfrac></mrow>;
      break;
    case 'root':
      content = <msqrt>{slot()}</msqrt>;
      break;
    case 'negative':
      content = <mrow><mo>−</mo>{slot()}</mrow>;
      break;
    case 'plus_minus':
      content = <mrow><mo>±</mo>{slot()}</mrow>;
      break;
    case 'tuple':
      content = <mrow>{slot()}<mo>,</mo>{slot()}</mrow>;
      break;
  }
  return <math className="math-template-icon" aria-hidden="true">{content}</math>;
}
