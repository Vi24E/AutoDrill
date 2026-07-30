declare module '/wasm/pkg/drill_wasm.js' {
  const init: (input?: unknown) => Promise<unknown>;
  export default init;
  export const generate_problem: (input: string) => string;
  export const generate_worksheet: (input: string) => string;
  export const apply_editor_action: (input: string) => string;
  export const normalize_answer: (input: string) => string;
  export const grade_answer: (input: string) => string;
  export const calculate_effort: (input: string) => string;
}
