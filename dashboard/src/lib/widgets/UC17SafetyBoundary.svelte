<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC17 — Safety boundary indicator: Fault Library active, ASIL-D isolation (SR-1.x, SR-4.x) -->
<script lang="ts">
	import { onMount, onDestroy } from 'svelte';

	// In canned mode simulate a healthy safety state with occasional flicker
	let faultLibActive = $state(true);
	let asilIsolated = $state(true);
	let mqttOnQmSide = $state(true);
	let timer: ReturnType<typeof setInterval> | null = null;

	onMount(() => {
		timer = setInterval(() => {
			// Simulate health heartbeat
			faultLibActive = true;
			asilIsolated = true;
			mqttOnQmSide = true;
		}, 5000);
	});

	onDestroy(() => {
		if (timer) clearInterval(timer);
	});

	const ok = (v: boolean) => v ? 'text-green-400' : 'text-red-400';
	const dot = (v: boolean) => v ? '● HEALTHY' : '● FAULT';
</script>

<div class="rounded-lg border border-border bg-card p-3 text-xs">
	<h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
		Safety Boundary (SR-1.x / SR-4.x)
	</h3>
	<div class="space-y-1">
		<div class="flex items-center justify-between">
			<span class="text-muted-foreground">Fault Library</span>
			<span class="font-semibold {ok(faultLibActive)}">{dot(faultLibActive)}</span>
		</div>
		<div class="flex items-center justify-between">
			<span class="text-muted-foreground">ASIL-D Isolation</span>
			<span class="font-semibold {ok(asilIsolated)}">{dot(asilIsolated)}</span>
		</div>
		<div class="flex items-center justify-between">
			<span class="text-muted-foreground">MQTT on QM side</span>
			<span class="font-semibold {ok(mqttOnQmSide)}">{dot(mqttOnQmSide)}</span>
		</div>
	</div>
	<p class="mt-2 text-[10px] text-muted-foreground">
		MQTT path never crosses ASIL boundary — ADR-0024 §Cross-refs
	</p>
</div>
