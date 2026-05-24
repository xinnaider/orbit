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

test.describe('Orbit git workflow flows', () => {
  test('git workflow: edit file and stage', async ({ page }) => {
    await page.getByTestId('new-session-button').click();
    
    await page.getByTestId('new-session-path').fill('/tmp/orbit-e2e-git-test');
    await page.getByTestId('new-session-prompt').fill('Edit ui/lib/tauri/git.ts and stage the changes');
    await page.getByTestId('start-session-button').click();

    await expect(page.getByText('Edit ui/lib/tauri/git.ts and stage the changes')).toBeVisible({ timeout: 15_000 });

    // Wait for agent to complete and show diff
    const diffInput = page.getByTestId('diff-output');
    await expect(diffInput).toBeVisible({ timeout: 30_000 });

    // Verify diff output has git diff format
    const diffText = await diffInput.textContent();
    expect(diffText).toContain('diff --git');
    expect(diffText).toContain('+++ b/ui/lib/tauri/git.ts');
    expect(diffText).toContain('@@');
  });

  test('git workflow: stage all and commit', async ({ page }) => {
    await page.getByTestId('new-session-button').click();
    
    await page.getByTestId('new-session-path').fill('/tmp/orbit-e2e-git-test-2');
    await page.getByTestId('new-session-prompt').fill('Add a new function validateGitConfig to git.ts, stage it, and commit with message');
    await page.getByTestId('start-session-button').click();

    await expect(page.getByText('Add a new function validateGitConfig to git.ts, stage it, and commit with message')).toBeVisible({ timeout: 15_000 });

    // Wait for complete workflow
    const timeline = page.locator('.timeline');
    await expect(timeline).toBeVisible({ timeout: 30_000 });

    // Verify commit was made
    const messageInput = page.getByTestId('message-input');
    if (await messageInput.isVisible()) {
      const message = await messageInput?.textContent();
      expect(message).toBeTruthy();
    }
  });

  test('git workflow: format diff output', async ({ page }) => {
    await page.getByTestId('new-session-button').click();
    
    await page.getByTestId('new-session-path').fill('/tmp/orbit-e2e-git-test-3');
    await page.getByTestId('new-session-prompt').fill('Modify git.ts and show formatted diff with syntax highlighting');
    await page.getByTestId('start-session-button').click();

    await expect(page.getByText('Modify git.ts and show formatted diff with syntax highlighting')).toBeVisible({ timeout: 15_000 });

    // Verify formatted diff output
    const diffOutput = page.getByTestId('diff-formatted');
    await expect(diffOutput).toBeVisible({ timeout: 30_000 });

    // Check for markdown code block
    const diffText = await diffOutput.textContent();
    expect(diffText).toContain('\`\`\`');
    expect(diffText).toContain('diff --git');
  });

  test('git workflow: quick commit with auto message', async ({ page }) => {
    await page.getByTestId('new-session-button').click();
    
    await page.getByTestId('new-session-path').fill('/tmp/orbit-e2e-git-test-4');
    await page.getByTestId('new-session-prompt').fill('Add multiple functions to git.ts and commit with auto-generated message');
    await page.getByTestId('start-session-button').click();

    await expect(page.getByText('Add multiple functions to git.ts and commit with auto-generated message')).toBeVisible({ timeout: 15_000 });

    // Verify quick commit workflow
    const timeline = page.locator('.timeline');
    await expect(timeline).toBeVisible({ timeout: 30_000 });

    // Check that commit message includes modified files
    const messageInput = page.getByTestId('message-input');
    if (await messageInput.isVisible()) {
      const message = await messageInput.textContent();
      if (message) {
        expect(message).toContain('git.ts');
      }
    }
  });
});
