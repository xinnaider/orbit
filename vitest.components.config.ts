import { defineConfig } from 'vitest/config';
import path from 'path';

export default defineConfig(async () => {
  const { svelte } = await import('@sveltejs/vite-plugin-svelte');

  return {
    plugins: [
      svelte({
        compilerOptions: {
          dev: true,
        },
        hot: false,
        preprocess: false,
      }),
    ],
    test: {
      include: ['ui/lib/**/*.component.test.ts', 'ui/components/**/*.component.test.ts'],
      setupFiles: ['./ui/test-setup.ts'],
      environment: 'happy-dom',
      alias: {
        $lib: path.resolve(__dirname, 'ui/lib'),
      },
    },
    resolve: {
      conditions: ['browser'],
    },
  };
});
