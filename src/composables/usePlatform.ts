import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export type PlatformId = 'kdp' | 'wide' | 'craft' | 'publish';

interface AppStateRow {
  platform: string;
  active_story_id: string;
}

const platform = ref<PlatformId>('kdp');

const isKdp = computed(() => platform.value === 'kdp');

async function hydratePlatform(): Promise<void> {
  try {
    const state = await invoke<AppStateRow>('load_app_state');
    if (state.platform === 'kdp' || state.platform === 'wide' || state.platform === 'craft' || state.platform === 'publish') {
      platform.value = state.platform;
    }
  } catch {
    // keep default
  }
}

void hydratePlatform();

function setPlatform(p: PlatformId): void {
  platform.value = p;
  void (async () => {
    try {
      const state = await invoke<AppStateRow>('load_app_state');
      await invoke<AppStateRow>('save_app_state', {
        state: { platform: p, active_story_id: state.active_story_id || '' },
      });
    } catch {
      await invoke<AppStateRow>('save_app_state', {
        state: { platform: p, active_story_id: '' },
      });
    }
  })();
}

export function usePlatform() {
  return {
    platform,
    isKdp,
    setPlatform,
  };
}
