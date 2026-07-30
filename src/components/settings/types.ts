export type SettingsTab =
  | 'general'
  | 'ai'
  | 'folders'
  | 'canopy'
  | 'dataforseo'
  | 'winningcat'
  | 'storydata'
  | 'archived'
  | 'database';

export const SETTINGS_TABS: { id: SettingsTab; label: string }[] = [
  { id: 'general', label: 'General' },
  { id: 'ai', label: 'AI Models' },
  { id: 'folders', label: 'Folders' },
  { id: 'canopy', label: 'Canopy' },
  { id: 'dataforseo', label: 'DataForSEO' },
  { id: 'winningcat', label: 'WinningCat' },
  { id: 'storydata', label: 'Story Data' },
  { id: 'archived', label: 'Archived Reports' },
  { id: 'database', label: 'Database' },
];
