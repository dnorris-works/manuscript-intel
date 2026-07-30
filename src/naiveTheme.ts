import type { GlobalThemeOverrides } from 'naive-ui';
import { getThemeTokens, type ThemeMode } from './theme/tokens';

const FONT_FAMILY = 'Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif';
const FONT_FAMILY_MONO = 'Fira Code, "SF Mono", monospace';

/** Naive UI theme — brand colors on top of default component styling. */
export function getNaiveThemeOverrides(mode: ThemeMode): GlobalThemeOverrides {
  const t = getThemeTokens(mode);

  return {
    common: {
      primaryColor: t.accent,
      primaryColorHover: t.accentHover,
      primaryColorPressed: t.accentPressed,
      primaryColorSuppl: t.accent,
      successColor: t.success,
      errorColor: t.danger,
      bodyColor: t.bg,
      cardColor: t.surface,
      modalColor: t.surface,
      popoverColor: t.surface,
      borderColor: t.border,
      dividerColor: t.border,
      textColorBase: t.text,
      textColor1: t.text,
      textColor2: t.text,
      textColor3: t.textMuted,
      fontFamily: FONT_FAMILY,
      fontFamilyMono: FONT_FAMILY_MONO,
      borderRadius: t.radius,
    },
  };
}
