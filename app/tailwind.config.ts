import type { Config } from 'tailwindcss'

const config: Config = {
  content: ['./src/renderer/**/*.{js,ts,jsx,tsx,html}'],
  theme: {
    extend: {
      colors: {
        navy: {
          DEFAULT: '#1c1c1c',
          light: '#2c2c2c',
          surface: '#363636',
          border: '#3a3a3a',
        },
        gold: {
          DEFAULT: '#c9a84c',
          light: '#e8c97e',
          dark: '#a07c30',
        }
      },
      fontFamily: {
        sans: ['-apple-system', 'BlinkMacSystemFont', '"Segoe UI"', 'sans-serif'],
        mono: ['"SF Mono"', 'Consolas', 'monospace'],
      }
    }
  },
  plugins: []
}

export default config
