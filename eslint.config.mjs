import js from '@eslint/js';
import { defineConfig, globalIgnores } from 'eslint/config';
import prettierConfig from 'eslint-config-prettier';
import react from 'eslint-plugin-react';
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
  react.configs.flat.recommended,
  react.configs.flat['jsx-runtime'],
  reactHooks.configs.flat['recommended-latest'],
  reactRefresh.configs.vite,
  prettierConfig,

  {
    languageOptions: {
      globals: { ...globals.browser },
    },
    settings: { react: { version: 'detect' } },
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
      'react/prop-types': 'off',
      'react/jsx-boolean-value': ['error', 'never'],
      'react/jsx-no-useless-fragment': 'error',
      'react/self-closing-comp': 'error',
      'react/jsx-pascal-case': 'error',
      'react/no-array-index-key': 'warn',
      'react/display-name': 'off',

      // Best Practices
      eqeqeq: ['error', 'always', { null: 'ignore' }],
      'default-case': 'warn',
      'no-else-return': 'error',
      'no-throw-literal': 'error',
      'prefer-promise-reject-errors': 'error',
    },
  },
]);
