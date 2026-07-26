import { ref, computed } from 'vue';

export type PlatformId = 'kdp' | 'wide' | 'craft' | 'publish';

const platform = ref<PlatformId>(
  (localStorage.getItem('platform') as PlatformId) || 'kdp'
);

const isKdp = computed(() => platform.value === 'kdp');

function setPlatform(p: PlatformId): void {
  platform.value = p;
  localStorage.setItem('platform', p);
}

export function usePlatform() {
  return {
    platform,
    isKdp,
    setPlatform,
  };
}
