// ── Shared types for Manuscript Intel UI ──────────────────────────────────────

export interface Story {
  id: string;
  name: string;
  folder: string;
  created: string;
  bible_path: string;
}

export interface StoriesResult {
  success: boolean;
  stories: Story[];
  error: string;
}

export interface GenreResult {
  success: boolean;
  report: string;
  error: string;
  run_ts: string;
}

export interface ModelInfo {
  id: string;
  owned_by: string;
  input_price: number | null;
  output_price: number | null;
}

export interface ModelsResult {
  success: boolean;
  models: ModelInfo[];
  error: string;
}

export type SummaryFileStatus = 'pending' | 'active' | 'done' | 'skipped';

export interface SummaryChapterProgress {
  filename: string;
  status: 'started' | 'done' | 'skipped';
}

export interface AnalysisState {
  has_folder: boolean;
  summary_count: number;
  summary_chapter_count: number;
  summary_missing_count: number;
  summary_stale_count: number;
  summary_missing_files: string[];
  summary_stale_files: string[];
  has_genre_data: boolean;
  has_full_report: boolean;
  has_keywords: boolean;
  has_search_terms: boolean;
  has_competition: boolean;
  has_categories: boolean;
  has_genre_ranking: boolean;
  has_mapped_verified: boolean;
  has_bisac: boolean;
  has_discovery_keywords: boolean;
  has_keyword_search_results: boolean;
  has_zeigarnik: boolean;
  has_continuity_check: boolean;
  has_show_dont_tell: boolean;
  has_ai_isms: boolean;
  existing_docs: string[];
}

export interface SeriesRow {
  id: number;
  name: string;
  book_count: number;
}

export interface SeriesBookRow {
  story_folder: string;
  story_name: string;
  book_order: number;
}

export interface Settings {
  provider: string;
  apiKey: string;
  model: string;
  canopyApiKey: string;
}

export interface DocMeta {
  id: number;
  doc_type: string;
  label: string;
  generated_at: string;
}

export interface SavedReportMeta {
  id: number;
  doc_type: string;
  version: number;
  label: string;
  saved_at: string;
}

export interface ReportEnvelope {
  id: number;
  doc_type: string;
  label: string;
  format: string;
  content: string;
  generated_at: string;
}

export interface LogLine {
  type: string;
  icon: string;
  text: string;
}

export interface SidebarReportVersion {
  id: number;
  generated_at: string;
}

export interface SidebarReportGroup {
  doc_type: string;
  label: string;
  description: string;
  count: number;
  versions: SidebarReportVersion[];
}

export interface Finding {
  filePath: string;
  chapterTitle: string;
  tellingText: string;
  context: string;
  why: string;
  severity: string;
  reportType: 'show_dont_tell' | 'ai_isms' | 'continuity';
  entity?: string;
  attribute?: string;
  explanation?: string;
  occurrences?: { story_name: string; file: string; chapter_title: string; value: string; snippet: string }[];
}

export interface WinningCatImportResult {
  success: boolean;
  imported: number;
  skipped_other_department: number;
  skipped_unparseable: number;
  stale_count: number;
  imported_at: string;
  error: string;
}

export interface StaleCleanupResult {
  success: boolean;
  removed: number;
  error: string;
}

export interface ReportTypeDef {
  id: string;
  label: string;
  description: string;
  platforms: string[];
  depends_on: string[];
  model_slot: string;
  min_tier: string;
}

export interface SeriesBook {
  story_folder: string;
  story_name: string;
  book_order: number;
}

export interface Series {
  id: number;
  name: string;
  created_at: string;
  bible_path: string;
  books: SeriesBook[];
}

export interface SeriesResult {
  success: boolean;
  series: Series[];
  error: string;
}

// ── Ad campaigns (Marketing mode) ─────────────────────────────────────────────

export interface AdPlatformAccount {
  id: number;
  platform: string;
  account_id: string;
  pixel_id: string;
  tracking_notes: string;
  payment_notes: string;
  created_at: string;
  updated_at: string;
}

export interface AdLandingPage {
  id: number;
  story_folder: string;
  name: string;
  url: string;
  conversion_rate: number | null;
  notes: string;
  created_at: string;
  updated_at: string;
}

export interface AdCampaign {
  id: number;
  story_folder: string;
  name: string;
  platform: string;
  platform_account_id: number | null;
  objective: string;
  status: string;
  budget: number | null;
  budget_period: string;
  start_date: string;
  end_date: string;
  target_audience: string;
  landing_page_id: number | null;
  notes: string;
  created_at: string;
  updated_at: string;
  total_spend?: number;
}

export interface AdCreative {
  id: number;
  campaign_id: number;
  name: string;
  creative_type: string;
  version: string;
  platform_format: string;
  status: string;
  asset_path: string;
  body_text: string;
  notes: string;
  created_at: string;
  updated_at: string;
}

export interface AdPerformanceSnapshot {
  id: number;
  campaign_id: number;
  creative_id: number | null;
  snapshot_date: string;
  impressions: number;
  clicks: number;
  conversions: number;
  ctr: number;
  cpc: number;
  cpa: number;
  spend: number;
  notes: string;
  created_at: string;
}

export interface AdSpendEntry {
  id: number;
  campaign_id: number;
  platform: string;
  amount: number;
  spent_at: string;
  notes: string;
  created_at: string;
}

export interface AdAudienceNote {
  id: number;
  campaign_id: number;
  label: string;
  demographics: string;
  interests: string;
  lookalike_notes: string;
  outcome: string;
  notes: string;
  created_at: string;
}

export interface AdCampaignDetail {
  campaign: AdCampaign;
  creatives: AdCreative[];
  snapshots: AdPerformanceSnapshot[];
  spend_entries: AdSpendEntry[];
  audience_notes: AdAudienceNote[];
  landing_page: AdLandingPage | null;
}
