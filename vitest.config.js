/// <reference types="vitest" />
import { defineConfig } from 'vite';
import { sveltekit } from '@sveltejs/kit/vite';

export default defineConfig({
  plugins: [sveltekit()],

  test: {
    include: ['ui/lib/**/*.test.ts'],
    exclude: [
      '**/node_modules/**',
      '**/.claude/worktrees/**',
      '**/tauri/src/**',
      '**/*.component.test.ts',
      'e2e/**',
    ],
    environment: 'jsdom',
  },
  define: {
    'import.meta.env.VITE_MOCK': '"true"',
  },
});
