import { expect, test } from '@playwright/test';
import { createMockSession, gotoMockApp } from './helpers';

test.describe('Chat composer', () => {
  test.beforeEach(async ({ page }) => {
    await gotoMockApp(page);
    await createMockSession(page, { prompt: `E2E chat ${Date.now()}` });
  });

  test('sends a follow-up message', async ({ page }) => {
    const followUp = 'Follow-up from E2E';
    await page.getByTestId('message-input').fill(followUp);
    await page.getByTestId('send-message-button').click();

    await expect(page.getByText(followUp)).toBeVisible({ timeout: 15_000 });
    await expect(page.getByTestId('session-feed')).toBeVisible();
    await expect(page.locator('.timeline')).toBeVisible();
  });

  test('shows slash command picker when typing /', async ({ page }) => {
    await page.getByTestId('message-input').fill('/');
    await expect(page.locator('.dropdown')).toBeVisible();
    await expect(page.locator('.drop-item').first()).toBeVisible();
  });

  test('opens slash picker via composer chip', async ({ page }) => {
    const input = page.getByTestId('message-input');
    await page.getByRole('button', { name: '/ command' }).click();
    await expect(input).toHaveValue('/ ');
    // Chip adds a trailing space; picker lists commands only for lone `/`.
    await input.fill('/');
    await expect(page.locator('.dropdown .drop-item').first()).toBeVisible({ timeout: 5_000 });
  });
});
