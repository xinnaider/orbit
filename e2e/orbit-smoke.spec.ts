import { expect, test } from '@playwright/test';

test.describe('Orbit smoke flows', () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('orbit:lastSeenChangelogVersion', '0.0.0');
    });
    await page.goto('/');

    const changelogTitle = page.getByText("what's new in orbit");
    if (await changelogTitle.isVisible({ timeout: 1_000 }).catch(() => false)) {
      await page.locator('.close-btn').click();
      await expect(changelogTitle).toBeHidden();
    }

    await expect(page.getByTestId('new-session-button')).toBeVisible({ timeout: 15_000 });
  });

  test('creates a session from the mock UI', async ({ page }) => {
    await page.getByTestId('new-session-button').click();

    await expect(page.getByText('new session', { exact: true })).toBeVisible();

    await page.getByTestId('new-session-path').fill('/tmp/orbit-e2e-project');
    await page.getByTestId('new-session-prompt').fill('Create an E2E smoke response');
    await page.getByTestId('start-session-button').click();

    await expect(page.getByText('Create an E2E smoke response')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('message-input')).toBeVisible({ timeout: 15_000 });

    await expect(page.getByTestId('quiet-sidebar')).toBeVisible();
    await expect(page.locator('.timeline')).toBeVisible();
    await expect(page.getByTitle('Toggle inspector panel')).toBeVisible();
  });
});
