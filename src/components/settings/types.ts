export type SettingsTab =
  | 'general'
  | 'ai'
  | 'folders'
  | 'canopy'
  | 'dataforseo'
  | 'winningcat'
  | 'database';

export const SETTINGS_TABS: { id: SettingsTab; label: string }[] = [
  { id: 'general', label: 'General' },
  { id: 'ai', label: 'AI Models' },
  { id: 'folders', label: 'Folders' },
  { id: 'canopy', label: 'Canopy' },
  { id: 'dataforseo', label: 'DataForSEO' },
  { id: 'winningcat', label: 'WinningCat' },
  { id: 'database', label: 'Database' },
];
