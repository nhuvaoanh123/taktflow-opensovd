<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- Clear DTC action guard. Public dashboard builds default to read-only. -->
<script lang="ts">
	import { clearFaults, getComponent } from '$lib/api/sovdClient';
	import type { ComponentSource, EcuId } from '$lib/types/sovd';

	interface Props {
		componentId: EcuId;
		onCleared?: () => void;
		mutationsEnabled?: boolean;
	}

	let { componentId, onCleared, mutationsEnabled = false }: Props = $props();

	let loading = $state(false);
	let message = $state<string | null>(null);
	let source = $state<ComponentSource>('unknown');

	const canClear = $derived(mutationsEnabled && source === 'local');
	const disabledReason = $derived(
		mutationsEnabled
			? 'Clear is unavailable for routed components'
			: 'Disabled in the public read-only build'
	);

	$effect(() => {
		message = null;
		void loadSource(componentId);
	});

	async function loadSource(id: EcuId) {
		// An unavailable component route reads as 'unknown', which keeps the
		// destructive clear action disabled.
		source = (await getComponent(id))?.source ?? 'unknown';
	}

	async function handleClear() {
		if (!canClear) {
			return;
		}
		loading = true;
		message = null;
		try {
			await clearFaults(componentId);
			message = 'Faults cleared; audit entry written.';
			onCleared?.();
		} catch (cause) {
			message = cause instanceof Error ? cause.message : 'Clear request failed.';
		} finally {
			loading = false;
		}
	}
</script>

<div class="flex flex-col gap-1">
	{#if canClear}
		<button
			onclick={handleClear}
			disabled={loading}
			class="rounded border border-red-300 bg-white px-3 py-1 text-xs font-medium text-red-700 hover:bg-red-50 disabled:opacity-50"
		>
			{loading ? 'Clearing...' : 'Clear faults'}
		</button>
	{:else}
		<!-- Disabled buttons swallow hover events, so the title sits on a wrapper. -->
		<span title={disabledReason} class="inline-block">
			<button
				type="button"
				disabled
				aria-label={`Clear — ${disabledReason}`}
				class="cursor-not-allowed rounded border border-border bg-muted px-3 py-1 text-xs font-medium text-muted-foreground"
			>
				Clear
			</button>
		</span>
	{/if}
	{#if message}
		<p class="max-w-64 text-[10px] text-muted-foreground">{message}</p>
	{/if}
</div>
