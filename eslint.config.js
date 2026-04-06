import js from '@eslint/js';
import ts from '@typescript-eslint/eslint-plugin';
import tsParser from '@typescript-eslint/parser';
import svelte from 'eslint-plugin-svelte';
import svelteParser from 'svelte-eslint-parser';
import globals from 'globals';

/** @type {import('eslint').Linter.Config[]} */
export default [
  js.configs.recommended,

  // TypeScript source files (not declarations)
  {
    files: ['api/**/*.ts'],
    ignores: ['api/**/*.d.ts'],
    languageOptions: {
      parser: tsParser,
      globals: { ...globals.browser, ...globals.node },
    },
    plugins: { '@typescript-eslint': ts },
    rules: {
      ...ts.configs['recommended'].rules,
      '@typescript-eslint/no-unused-vars': [
        'error',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_', caughtErrorsIgnorePattern: '^_' },
      ],
      '@typescript-eslint/no-explicit-any': 'warn',
      'no-unused-vars': 'off',
    },
  },

  // Svelte files — type checking is handled by svelte-check, not ESLint
  {
    files: ['api/**/*.svelte'],
    languageOptions: {
      parser: svelteParser,
      parserOptions: {
        parser: tsParser,
        extraFileExtensions: ['.svelte'],
      },
      globals: { ...globals.browser },
    },
    plugins: { svelte },
    rules: {
      ...svelte.configs.recommended.rules,
      'no-undef': 'off',
      'no-unused-vars': 'off',
      // {@html} is intentional for Markdown rendering and bundled SVG assets
      'svelte/no-at-html-tags': 'off',
    },
  },

  // Global ignores
  {
    ignores: [
      'node_modules/**',
      'build/**',
      '.svelte-kit/**',
      'front/target/**',
      'api/**/*.d.ts',
      '*.config.js',
    ],
  },
];
