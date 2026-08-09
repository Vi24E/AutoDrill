import type React from 'react';

type MathMLAttributes = React.Attributes & React.DOMAttributes<MathMLElement> & React.AriaAttributes & {
  className?: string;
  id?: string;
  style?: React.CSSProperties;
  children?: React.ReactNode;
  title?: string;
  width?: string;
  height?: string;
  depth?: string;
  [key: `data-${string}`]: string | number | undefined;
};

declare global {
  namespace JSX {
    interface IntrinsicElements {
      math: MathMLAttributes;
      mrow: MathMLAttributes;
      mtext: MathMLAttributes;
      mo: MathMLAttributes;
      mi: MathMLAttributes;
      mn: MathMLAttributes;
      mfrac: MathMLAttributes;
      msqrt: MathMLAttributes;
      mroot: MathMLAttributes;
      mpadded: MathMLAttributes;
    }
  }
}

export {};
