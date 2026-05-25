import { expect, test } from '@playwright/test';
import { createMockSession, gotoMockApp } from './helpers';

test.describe('Orbit smoke flows', () => {
  test.beforeEach(async ({ page }) => {
    await gotoMockApp(page);
  });

  test('creates a session from the mock UI', async ({ page }) => {
    await createMockSession(page, {
      path: '/tmp/orbit-e2e-project',
      prompt: 'Create an E2E smoke response',
    });

    await expect(page.getByTestId('quiet-sidebar')).toBeVisible();
    await expect(page.getByTestId('session-feed')).toBeVisible();
    await expect(page.locator('.timeline')).toBeVisible();
    await expect(page.getByTitle('Toggle inspector panel')).toBeVisible();
  });
});
