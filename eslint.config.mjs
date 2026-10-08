import js from '@eslint/js';
import { defineConfig, globalIgnores } from 'eslint/config';
import prettierConfig from 'eslint-config-prettier';
import eslintReact from '@eslint-react/eslint-plugin';
import reactHooks from 'eslint-plugin-react-hooks';
import reactRefresh from 'eslint-plugin-react-refresh';
import globals from 'globals';
import tseslint from 'typescript-eslint';

export default defineConfig([
  globalIgnores([
    'dist/**',
    'coverage/**',
    'node_modules/**',
    'src-tauri/target/**',
    'src-tauri/gen/**',
    '*.generated.*',
  ]),

  js.configs.recommended,
  tseslint.configs.recommended,
  eslintReact.configs['recommended-typescript'],
  // Hooks rules come from the official plugin (React Compiler rules) below, not twice.
  eslintReact.configs['disable-conflict-eslint-plugin-react-hooks'],
  reactHooks.configs.flat['recommended-latest'],
  reactRefresh.configs.vite,
  prettierConfig,

  {
    languageOptions: {
      globals: { ...globals.browser },
    },
  },

  {
    files: ['*.config.{js,mjs,ts,mts}'],
    languageOptions: { globals: { ...globals.node } },
  },

  {
    files: ['**/*.{js,jsx,ts,tsx,mjs,cjs}'],
    rules: {
      // Code Quality
      'no-console': ['warn', { allow: ['warn', 'error'] }],
      'no-debugger': 'error',
      'no-alert': 'warn',
      'no-var': 'error',
      'prefer-const': 'error',
      'prefer-arrow-callback': 'error',
      'no-duplicate-imports': 'error',
      'prefer-template': 'error',
      'object-shorthand': 'error',

      // TypeScript
      '@typescript-eslint/consistent-type-imports': 'error',
      '@typescript-eslint/no-unused-vars': [
        'error',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
      ],

      // React
      '@eslint-react/jsx-no-useless-fragment': 'error',
      '@eslint-react/no-array-index-key': 'warn',

      // Best Practices
      eqeqeq: ['error', 'always', { null: 'ignore' }],
      'default-case': 'warn',
      'no-else-return': 'error',
      'no-throw-literal': 'error',
      'prefer-promise-reject-errors': 'error',
    },
  },
]);
