import type { DrillWasmRuntime } from '@/domain/wasm-adapter';

let generatedRuntimePromise: Promise<DrillWasmRuntime> | null = null;

async function initializeGeneratedWasmRuntime(): Promise<DrillWasmRuntime> {
  // The module is emitted only by scripts/build-wasm.sh and is intentionally
  // absent from normal source checkouts.
  // Keep the specifier out of both Next/Webpack and Vitest/Vite's static module
  // graph; the file is a public asset created after this source is compiled.
  const generatedPath = ['/wasm/pkg', 'drill_wasm.js'].join('/');
  const generated = (await import(/* webpackIgnore: true */ generatedPath)) as {
    default?: (input?: unknown) => Promise<unknown>;
    [key: string]: unknown;
  };
  if (typeof generated.default === 'function') {
    await generated.default();
  }
  return generated as unknown as DrillWasmRuntime;
}

/**
 * Load the ignored wasm-pack web package produced by `scripts/build-wasm.sh`.
 *
 * The dynamic import keeps the optional generated file out of the Next.js and
 * Vitest compile graphs, so a normal source checkout remains buildable while a
 * locally generated package is still loadable at runtime. The generated web
 * glue initializes itself relative to `/wasm/pkg/drill_wasm_bg.wasm`.
 *
 * React StrictMode may mount the preload effect twice before the first async
 * initialization completes. wasm-bindgen initialization is therefore shared
 * through one module-level promise; a failed attempt clears the promise so a
 * later explicit retry can try again.
 */
export async function loadGeneratedWasmRuntime(): Promise<DrillWasmRuntime> {
  if (typeof window === 'undefined') {
    throw new Error('The generated WASM runtime can only load in a browser.');
  }

  if (!generatedRuntimePromise) {
    generatedRuntimePromise = initializeGeneratedWasmRuntime().catch((error: unknown) => {
      generatedRuntimePromise = null;
      throw error;
    });
  }
  return generatedRuntimePromise;
}
