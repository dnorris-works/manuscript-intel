import { ref, computed, type Ref } from 'vue';
import type { ModelInfo } from '../types';
import type { ReportTypeDef } from '../types';
import { hasModelPricing } from '../reportCostPricing';

export type ModelSort = 'price' | 'provider';
export type Tier = 'basic' | 'capable' | 'strong';

const TIER_RANK: Record<Tier, number> = { basic: 0, capable: 1, strong: 2 };

function formatUsd(value: number): string {
  if (value === 0) return '0';
  if (value < 0.001) return value.toFixed(4);
  if (value < 0.01) return value.toFixed(3);
  return value.toFixed(2);
}

export function isModelFreeLike(m: ModelInfo): boolean {
  const hasBothPrices = m.input_price != null && m.output_price != null;
  const byPrice = hasBothPrices && m.input_price === 0 && m.output_price === 0;
  const byName = /(^|[\\/:_-])free([\\/:_-]|$)/i.test(m.id) || /free/i.test(m.owned_by || '');
  return byPrice || byName;
}

export function modelPriceLabel(m: ModelInfo): string {
  const inPrice = m.input_price;
  const outPrice = m.output_price;

  if (inPrice == null && outPrice == null) {
    if (isModelFreeLike(m)) return 'FREE (provider-labeled; price not published)';
    return 'pricing unavailable';
  }

  const inText = inPrice == null ? 'in unknown' : `$${formatUsd(inPrice)} in`;
  const outText = outPrice == null ? 'out unknown' : `$${formatUsd(outPrice)} out`;

  if (isModelFreeLike(m) && inPrice === 0 && outPrice === 0) {
    return 'FREE (0/1K tokens)';
  }

  if (inPrice != null && outPrice != null) {
    const total = inPrice + outPrice;
    return `${inText} + ${outText} = ~$${formatUsd(total)} /1K`;
  }

  return `${inText} + ${outText} /1K`;
}

function modelTier(m: ModelInfo): Tier {
  const price = m.input_price ?? 0;
  if (price <= 0.001) return 'basic';
  if (price <= 0.01) return 'capable';
  return 'strong';
}

export function useSettingsModels(
  models: Ref<ModelInfo[]>,
  reportTypes: Ref<ReportTypeDef[]>,
) {
  const modelSort = ref<ModelSort>('price');
  const pricedOnly = ref(false);
  const freeOnly = ref(false);
  const modelFetchStatus = ref('');

  const sortedModels = computed(() => {
    return [...models.value].sort((a, b) => {
      if (modelSort.value === 'provider') {
        const provA = a.owned_by.toLowerCase();
        const provB = b.owned_by.toLowerCase();
        if (provA !== provB) return provA.localeCompare(provB);
      }
      const priceA = a.input_price ?? Infinity;
      const priceB = b.input_price ?? Infinity;
      return priceA - priceB;
    });
  });

  const unpricedModelCount = computed(() =>
    sortedModels.value.filter(m => !hasModelPricing(m)).length,
  );

  /** Only models with published input/output pricing can be assigned. */
  const selectableModels = computed(() => {
    return sortedModels.value.filter(hasModelPricing).filter((m) => {
      const isFree = isModelFreeLike(m);
      if (freeOnly.value) return isFree;
      if (pricedOnly.value) return !isFree;
      return true;
    });
  });

  const filteredModels = selectableModels;

  function minTierFor(fnKey: string): Tier {
    if (fnKey === 'prose') return 'strong';
    const tiers = reportTypes.value
      .filter(r => r.model_slot === fnKey)
      .map(r => (r.min_tier as Tier) || 'basic');
    if (tiers.length === 0) return 'basic';
    return tiers.reduce((best, t) => (TIER_RANK[t] > TIER_RANK[best] ? t : best), 'basic' as Tier);
  }

  function modelFitLabel(m: ModelInfo, fnKey: string): string {
    const tier = modelTier(m);
    const min = minTierFor(fnKey);
    if (TIER_RANK[tier] >= TIER_RANK[min]) return ' ✓';
    return ' ⚠';
  }

  function fnOptionLabel(m: ModelInfo, fnKey: string): string {
    return `${m.id} (${modelPriceLabel(m)})${modelFitLabel(m, fnKey)}`;
  }

  function modelLabel(m: ModelInfo): string {
    let label = m.id;
    if (m.owned_by) label += ` (${m.owned_by})`;
    label += ` — ${modelPriceLabel(m)}`;
    return label;
  }

  const modelSelectOptions = computed(() =>
    filteredModels.value.map(m => ({ label: modelLabel(m), value: m.id })),
  );

  function fnSelectOptions(fnKey: string) {
    return filteredModels.value.map(m => ({
      label: fnOptionLabel(m, fnKey),
      value: m.id,
    }));
  }

  return {
    modelSort,
    pricedOnly,
    freeOnly,
    modelFetchStatus,
    sortedModels,
    selectableModels,
    unpricedModelCount,
    filteredModels,
    modelSelectOptions,
    fnSelectOptions,
    modelLabel,
    fnOptionLabel,
  };
}
