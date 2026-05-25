import { test } from '@playwright/test';

/**
 * Real Tauri WebDriver smoke — requires a built debug app and tauri-driver.
 * Run locally: npm run test:e2e:tauri
 * Skipped in default `npm run test:e2e` (mock-only).
 */
test.describe('Tauri app smoke', () => {
  test.skip(
    !process.env.TAURI_E2E,
    'Set TAURI_E2E=1 and run npm run test:e2e:tauri after building the debug app'
  );

  test('opens the desktop window', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: 'New session' }).waitFor({ timeout: 30_000 });
  });
});
