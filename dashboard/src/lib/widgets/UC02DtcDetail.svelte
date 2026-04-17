<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC02 — Single-DTC drill-in modal (FR-1.2) -->
<script lang="ts">
	import type { DtcEntry } from '$lib/types/sovd';

	interface Props {
		dtc: DtcEntry | null;
		onClose: () => void;
	}

	let { dtc, onClose }: Props = $props();

	function fmt(iso: string): string {
		return new Date(iso).toLocaleString();
	}
</script>

{#if dtc}
	<!-- Backdrop -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
		onclick={onClose}
		onkeydown={(e) => e.key === 'Escape' && onClose()}
		role="dialog"
		aria-modal="true"
		aria-label="DTC Detail"
		tabindex="-1"
	>
		<!-- Modal panel — stop propagation so clicking inside doesn't close -->
		<!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
		<div
			class="relative w-full max-w-md rounded-xl border border-border bg-card p-5 text-card-foreground shadow-2xl"
			role="document"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<button
				class="absolute right-3 top-3 text-muted-foreground hover:text-foreground"
				onclick={onClose}
				aria-label="Close"
			>
				✕
			</button>

			<h2 class="mb-1 font-mono text-lg font-bold">{dtc.code}</h2>
			<p class="mb-3 text-sm text-muted-foreground">{dtc.description}</p>

			<dl class="grid grid-cols-2 gap-x-4 gap-y-1 text-xs">
				<dt class="text-muted-foreground">Component</dt>
				<dd class="font-medium uppercase">{dtc.component}</dd>

				<dt class="text-muted-foreground">ECU Address</dt>
				<dd class="font-mono">0x{dtc.ecuAddress.toString(16).toUpperCase()}</dd>

				<dt class="text-muted-foreground">Severity</dt>
				<dd class="font-semibold capitalize">{dtc.severity}</dd>

				<dt class="text-muted-foreground">Status</dt>
				<dd class="capitalize">{dtc.status.replace('_', ' ')}</dd>

				<dt class="text-muted-foreground">First Seen</dt>
				<dd>{fmt(dtc.firstSeen)}</dd>

				<dt class="text-muted-foreground">Last Seen</dt>
				<dd>{fmt(dtc.lastSeen)}</dd>

				<dt class="text-muted-foreground">Occurrences</dt>
				<dd class="font-semibold">{dtc.occurrences}</dd>
			</dl>

			{#if dtc.freezeFrame}
				<div class="mt-3">
					<p class="mb-1 text-xs font-semibold text-muted-foreground">Freeze Frame</p>
					<div class="rounded bg-muted p-2">
						{#each Object.entries(dtc.freezeFrame) as [k, v] (k)}
							<div class="flex justify-between text-xs">
								<span class="text-muted-foreground">{k}</span>
								<span class="font-mono">{v}</span>
							</div>
						{/each}
					</div>
				</div>
			{/if}
		</div>
	</div>
{/if}
