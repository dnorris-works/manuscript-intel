export type ThemeMode = 'dark' | 'light';

/** Shared design tokens for CSS variables and Naive UI theme overrides. */
export interface ThemeTokens {
  bg: string;
  surface: string;
  surface2: string;
  border: string;
  text: string;
  textMuted: string;
  accent: string;
  accentHover: string;
  accentPressed: string;
  success: string;
  danger: string;
  hover: string;
  radius: string;
}

export const darkTokens: ThemeTokens = {
  bg: '#1a1a1a',
  surface: '#242424',
  surface2: '#2e2e2e',
  border: '#3a3a3a',
  text: '#e8e8e8',
  textMuted: '#888888',
  accent: '#e8612c',
  accentHover: '#f07040',
  accentPressed: '#b04820',
  success: '#4caf7d',
  danger: '#cf6679',
  hover: 'rgba(255, 255, 255, 0.06)',
  radius: '8px',
};

export const lightTokens: ThemeTokens = {
  bg: '#e4e2dc',
  surface: '#eceae4',
  surface2: '#dddbd4',
  border: '#c4c1b8',
  text: '#2a2926',
  textMuted: '#6a6760',
  accent: '#d45520',
  accentHover: '#e8612c',
  accentPressed: '#b0481c',
  success: '#2f8f5b',
  danger: '#c0392b',
  hover: 'rgba(0, 0, 0, 0.05)',
  radius: '8px',
};

export function getThemeTokens(mode: ThemeMode): ThemeTokens {
  return mode === 'light' ? lightTokens : darkTokens;
}

const CSS_VAR_MAP: Record<keyof ThemeTokens, string> = {
  bg: '--bg',
  surface: '--surface',
  surface2: '--surface2',
  border: '--border',
  text: '--text',
  textMuted: '--text-muted',
  accent: '--accent',
  accentHover: '--accent-hover',
  accentPressed: '--accent-dim',
  success: '--success',
  danger: '--danger',
  hover: '--overlay',
  radius: '--radius',
};

/** Push token values onto :root so custom CSS and Naive UI stay aligned. */
export function applyThemeTokens(mode: ThemeMode): void {
  const tokens = getThemeTokens(mode);
  const root = document.documentElement;
  root.setAttribute('data-theme', mode);
  for (const [key, cssVar] of Object.entries(CSS_VAR_MAP) as [keyof ThemeTokens, string][]) {
    root.style.setProperty(cssVar, tokens[key]);
  }
}
