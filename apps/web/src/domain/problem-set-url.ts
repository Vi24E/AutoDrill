export const PROBLEM_SET_QUERY_PARAMETER = 'seed';

export function problemSetIdFromSearch(search: string): string | null {
  const value = new URLSearchParams(search).get(PROBLEM_SET_QUERY_PARAMETER)?.trim() ?? '';
  return value === '' ? null : value;
}

function relativeUrl(url: URL): string {
  return `${url.pathname}${url.search}${url.hash}`;
}

export function urlWithProblemSetId(href: string, problemSetId: string): string {
  const url = new URL(href);
  url.searchParams.set(PROBLEM_SET_QUERY_PARAMETER, problemSetId);
  return relativeUrl(url);
}

export function urlWithoutProblemSetId(href: string): string {
  const url = new URL(href);
  url.searchParams.delete(PROBLEM_SET_QUERY_PARAMETER);
  return relativeUrl(url);
}
