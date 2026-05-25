import { expect, test } from '@playwright/test';
import { createMockSession, gotoMockApp } from './helpers';

test.describe('Feed tool diffs', () => {
  test.beforeEach(async ({ page }) => {
    await gotoMockApp(page);
  });

  test('shows edit diff lines for [fixture:edit] trigger', async ({ page }) => {
    await createMockSession(page, { prompt: '[fixture:edit]' });

    const feed = page.getByTestId('session-feed');
    await expect(feed.locator('.diff-line.add').first()).toBeVisible({ timeout: 15_000 });
    await expect(feed.locator('.diff-line.rem')).toBeVisible();
    await expect(feed.getByText('src/example.ts')).toBeVisible();
  });

  test('shows write diff for [fixture:write] trigger', async ({ page }) => {
    await createMockSession(page, { prompt: '[fixture:write]' });

    const feed = page.getByTestId('session-feed');
    await expect(feed.locator('.diff-line.add').first()).toBeVisible({ timeout: 15_000 });
    await expect(feed.getByText('src/new-file.ts')).toBeVisible();
  });
});
