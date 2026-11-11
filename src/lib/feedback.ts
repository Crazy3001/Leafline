// Where players report a problem or suggest an idea: the GitHub issue forms in
// .github/ISSUE_TEMPLATE. Leafline only opens the page; nothing is sent from the app.

const ISSUES = "https://github.com/Crazy3001/Leafline/issues/new";

/** The bug form, with the version filled in when it's known. */
export function reportUrl(version?: string | null): string {
  const url = `${ISSUES}?template=bug_report.yml`;
  return version ? `${url}&version=${encodeURIComponent(version)}` : url;
}

export const IDEA_URL = `${ISSUES}?template=idea.yml`;
