import { expect, test } from '@playwright/test';
import { dismissChangelog, gotoMockApp } from './helpers';

test.describe('Changelog helper', () => {
  test('dismissChangelog closes the modal when it appears', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByText("what's new in orbit")).toBeVisible({ timeout: 15_000 });
    await dismissChangelog(page, { skipInitScript: true });
    await expect(page.getByText("what's new in orbit")).toBeHidden();
    await expect(
      page.getByTestId('new-session-button').or(page.getByRole('button', { name: 'New session' }))
    ).toBeVisible();
  });

  test('gotoMockApp leaves changelog dismissed', async ({ page }) => {
    await gotoMockApp(page);
    await expect(page.getByText("what's new in orbit")).toBeHidden();
    await expect(
      page.getByTestId('new-session-button').or(page.getByRole('button', { name: 'New session' }))
    ).toBeVisible();
  });
});
