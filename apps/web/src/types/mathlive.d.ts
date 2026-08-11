import type * as React from 'react';
import type { MathfieldElement, MathSpanElement } from 'mathlive';

declare module 'react' {
  namespace JSX {
    interface IntrinsicElements {
      'math-field': React.DetailedHTMLProps<React.HTMLAttributes<MathfieldElement>, MathfieldElement> & {
        class?: string;
        'read-only'?: string;
      };
      'math-span': React.DetailedHTMLProps<React.HTMLAttributes<MathSpanElement>, MathSpanElement> & {
        class?: string;
        mode?: 'textstyle' | 'displaystyle';
        format?: 'latex' | 'ascii-math' | 'math-json';
      };
    }
  }
}

export {};
