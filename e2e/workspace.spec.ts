import { expect, test } from '@playwright/test';
import { createMockSession, gotoMockApp } from './helpers';

test.describe('Workspace shell', () => {
  test.beforeEach(async ({ page }) => {
    await gotoMockApp(page);
    await createMockSession(page, { prompt: 'Workspace E2E session' });
  });

  test('sidebar and session feed are visible', async ({ page }) => {
    await expect(page.getByTestId('quiet-sidebar')).toBeVisible();
    await expect(page.getByTestId('session-feed')).toBeVisible();
    await expect(page.locator('.timeline')).toBeVisible();
  });

  test('inspector badge appears when meta panel is hidden', async ({ page }) => {
    await expect(page.getByTitle('Toggle inspector panel')).toBeVisible();
    await page.getByTitle('Toggle inspector panel').click();
    await expect(page.getByTitle('Toggle inspector panel')).toBeHidden({ timeout: 5_000 });
  });
});
