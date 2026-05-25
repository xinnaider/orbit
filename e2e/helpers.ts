import { expect, type Page } from '@playwright/test';

export interface CreateMockSessionOptions {
  path?: string;
  prompt?: string;
}

/** Close the changelog modal when it appears (also pre-seeds localStorage before navigation). */
export async function dismissChangelog(page: Page, opts?: { skipInitScript?: boolean }) {
  if (!opts?.skipInitScript) {
    await page.addInitScript(() => {
      localStorage.setItem('orbit:lastSeenChangelogVersion', '0.0.0');
    });
  }

  const changelogTitle = page.getByText("what's new in orbit");
  if (await changelogTitle.isVisible({ timeout: 2_000 }).catch(() => false)) {
    await page.locator('.close-btn').click();
    await expect(changelogTitle).toBeHidden();
  }
}

function newSessionButton(page: Page) {
  return page.getByTestId('new-session-button').or(page.getByRole('button', { name: 'New session' }));
}

/** Navigate to the mock dev server and wait for the shell to be ready. */
export async function gotoMockApp(page: Page) {
  await dismissChangelog(page);
  await page.goto('/');
  await dismissChangelog(page);
  await expect(newSessionButton(page)).toBeVisible({ timeout: 15_000 });
}

/** Create a session through the new-session modal (mock backend). */
export async function createMockSession(page: Page, options: CreateMockSessionOptions = {}) {
  const path = options.path ?? '/tmp/orbit-e2e';
  const prompt = options.prompt ?? 'hello';

  await newSessionButton(page).click();
  await expect(page.getByText('new session', { exact: true })).toBeVisible();
  await page.getByTestId('new-session-path').fill(path);
  await page.getByTestId('new-session-prompt').fill(prompt);
  await page.getByTestId('start-session-button').click();

  await expect(page.getByText(prompt)).toBeVisible({ timeout: 15_000 });
  await expect(page.getByTestId('message-input')).toBeVisible({ timeout: 15_000 });
}
