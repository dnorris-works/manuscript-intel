import type { GlobalThemeOverrides } from 'naive-ui';
import { getThemeTokens, type ThemeMode } from './theme/tokens';

const FONT_FAMILY = 'Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif';
const FONT_FAMILY_MONO = 'Fira Code, "SF Mono", monospace';

/** Build Naive UI theme overrides from the shared design tokens. */
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
      tableColor: t.surface,
      tableHeaderColor: t.surface2,
      inputColor: t.surface2,
      codeColor: t.surface2,
      tabColor: t.surface2,
      actionColor: t.surface2,
      tagColor: t.surface2,
      avatarColor: t.surface2,
      hoverColor: t.hover,
      pressedColor: t.hover,
      borderColor: t.border,
      dividerColor: t.border,
      textColorBase: t.text,
      textColor1: t.text,
      textColor2: t.text,
      textColor3: t.textMuted,
      placeholderColor: t.textMuted,
      iconColor: t.textMuted,
      closeIconColor: t.textMuted,
      scrollbarColor: t.border,
      scrollbarColorHover: t.textMuted,
      fontFamily: FONT_FAMILY,
      fontFamilyMono: FONT_FAMILY_MONO,
      borderRadius: t.radius,
      borderRadiusSmall: '6px',
      fontSize: '14px',
      fontSizeSmall: '12px',
      heightMedium: '32px',
      heightSmall: '28px',
    },
    Button: {
      borderRadiusMedium: t.radius,
      borderRadiusSmall: '6px',
    },
    Input: {
      borderRadius: t.radius,
      color: t.surface2,
      colorFocus: t.surface2,
    },
    Select: {
      peers: {
        InternalSelection: {
          borderRadius: t.radius,
          color: t.surface2,
        },
      },
    },
    Card: {
      borderRadius: t.radius,
      color: t.surface,
      colorEmbedded: t.surface2,
    },
    Dialog: {
      borderRadius: t.radius,
      color: t.surface,
    },
    Tabs: {
      tabBorderRadius: t.radius,
    },
    Tag: {
      borderRadius: '6px',
    },
    DataTable: {
      borderRadius: t.radius,
      thColor: t.surface2,
      tdColor: t.surface,
    },
  };
}
