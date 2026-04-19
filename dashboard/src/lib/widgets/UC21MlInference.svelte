<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UP3-06 - Observer ML widget scaffold (ADR-0028 / ADR-0029) -->
<script lang="ts">
	import type { EcuId, MlInferenceResult } from '$lib/types/sovd';
	import { runMlInference } from '$lib/api/sovdClient';

	interface Props {
		componentId: EcuId;
	}

	let { componentId }: Props = $props();

	let result = $state<MlInferenceResult | null>(null);
	let loading = $state(false);

	const predictionChip: Record<MlInferenceResult['prediction'], string> = {
		normal: 'bg-emerald-700 text-white',
		warning: 'bg-amber-600 text-black',
		critical: 'bg-red-700 text-white'
	};

	async function refresh() {
		loading = true;
		try {
			result = await runMlInference(componentId);
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		result = null;
		void refresh();
	});
</script>

<div class="rounded-lg border border-border bg-card p-3 text-xs">
	<div class="mb-2 flex items-center justify-between gap-2">
		<h3 class="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
			ML Inference - {componentId.toUpperCase()}
		</h3>
		<button
			onclick={refresh}
			class="rounded bg-primary px-2 py-0.5 text-[10px] text-primary-foreground hover:bg-primary/80 disabled:opacity-60"
			disabled={loading}
		>
			{loading ? 'Running...' : 'Run inference'}
		</button>
	</div>

	{#if result}
		<div class="space-y-2">
			<div class="flex items-center justify-between gap-2">
				<div>
					<p class="font-medium">{result.modelName}</p>
					<p class="text-[10px] text-muted-foreground">
						v{result.modelVersion} · {result.source === 'live' ? 'live SOVD path' : 'stub fallback'}
					</p>
				</div>
				<span class="rounded px-1.5 py-0.5 text-[10px] font-semibold {predictionChip[result.prediction]}">
					{result.prediction}
				</span>
			</div>

			<dl class="grid grid-cols-2 gap-x-3 gap-y-0.5">
				<dt class="text-muted-foreground">Confidence</dt>
				<dd class="font-mono">{(result.confidence * 100).toFixed(1)}%</dd>

				<dt class="text-muted-foreground">Status</dt>
				<dd class="font-mono">{result.status}</dd>

				<dt class="text-muted-foreground">Fingerprint</dt>
				<dd class="truncate font-mono text-[10px]">{result.fingerprint}</dd>

				<dt class="text-muted-foreground">Updated</dt>
				<dd class="tabular-nums">{new Date(result.updatedAt).toLocaleTimeString()}</dd>
			</dl>

			<p class="text-[10px] text-muted-foreground">
				Request path: <code>/sovd/v1/components/{componentId}/operations/ml-inference/executions</code>
			</p>
		</div>
	{:else}
		<p class="text-muted-foreground">Loading inference result...</p>
	{/if}
</div>
