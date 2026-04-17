<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC03 — Clear DTCs button (FR-1.3) -->
<script lang="ts">
	import type { EcuId } from '$lib/types/sovd';
	import { clearFaults } from '$lib/api/sovdClient';

	interface Props {
		componentId: EcuId;
		onCleared?: () => void;
	}

	let { componentId, onCleared }: Props = $props();

	let loading = $state(false);
	let message = $state<string | null>(null);

	async function handleClear() {
		loading = true;
		message = null;
		try {
			await clearFaults(componentId);
			message = 'Faults cleared — audit entry written.';
			onCleared?.();
		} catch {
			message = 'Error: could not reach SOVD server.';
		} finally {
			loading = false;
		}
	}
</script>

<div class="flex flex-col gap-1">
	<button
		onclick={handleClear}
		disabled={loading}
		class="rounded bg-destructive px-3 py-1 text-xs font-semibold text-destructive-foreground hover:bg-destructive/80 disabled:opacity-50"
	>
		{loading ? 'Clearing…' : 'Clear Faults'}
	</button>
	{#if message}
		<p class="text-[10px] text-muted-foreground">{message}</p>
	{/if}
</div>
