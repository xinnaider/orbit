import { expect, test, type Page } from '@playwright/test';
import { createMockSession, gotoMockApp } from './helpers';

// SKIPPED: the Git overview tab is temporarily disabled (chats only for now), so
// it can no longer be opened from the "+" add menu. Re-enable these tests when
// Terminal and Git overview come back (see TabAddMenu / PaneContainer).
test.describe.skip('Git panel (mock)', () => {
  test.beforeEach(async ({ page }) => {
    await gotoMockApp(page);
    await createMockSession(page, {
      path: '/tmp/orbit-e2e-git',
      prompt: 'Git panel E2E session',
    });
  });

  async function openGitTab(page: Page) {
    await page.getByRole('button', { name: 'Add tab' }).click();
    await page.getByTestId('add-git-tab-option').click();
    await expect(page.getByTestId('git-panel')).toBeVisible({ timeout: 10_000 });
  }

  test('opens git tab and shows changed files', async ({ page }) => {
    await openGitTab(page);

    await expect(page.getByTestId('git-file-list')).toBeVisible();
    await expect(page.getByTestId('git-file-row').first()).toBeVisible();
    await expect(page.getByTestId('git-diff-viewer')).toBeVisible();
  });

  test('stages all and commits via mock toolbar', async ({ page }) => {
    await openGitTab(page);

    await page.getByTestId('git-stage-all-button').click();
    await expect(page.getByTestId('git-file-row').first()).toBeVisible();

    // Commit inline: type a message and commit the staged changes directly.
    await page.getByTestId('git-inline-commit-message').fill('e2e: test commit');
    await page.getByTestId('git-commit-button').click();

    await expect(page.getByTestId('git-file-list').getByText('No changes').first()).toBeVisible({
      timeout: 10_000,
    });
  });
});
