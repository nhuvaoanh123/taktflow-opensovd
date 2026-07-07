<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- Taktflow OpenSOVD - Live SIL Operations Dashboard -->
<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { Boxes, Gauge, RefreshCw, TriangleAlert } from 'lucide-svelte';

	import { getGatewayHealth } from '$lib/api/sovdClient';
	import { subscribe } from '$lib/api/wsClient';
	import type { DtcEntry, EcuId, GatewayHealth, TelemetryFrame } from '$lib/types/sovd';

	import UC01DtcList from '$lib/widgets/UC01DtcList.svelte';
	import UC02DtcDetail from '$lib/widgets/UC02DtcDetail.svelte';
	import UC03ClearFaults from '$lib/widgets/UC03ClearFaults.svelte';
	import UC04Pagination from '$lib/widgets/UC04Pagination.svelte';
	import UC05FaultsTimeline from '$lib/widgets/UC05FaultsTimeline.svelte';
	import UC06Operations from '$lib/widgets/UC06Operations.svelte';
	import UC08ComponentCards from '$lib/widgets/UC08ComponentCards.svelte';
	import UC09HwSwVersion from '$lib/widgets/UC09HwSwVersion.svelte';
	import UC10LiveDidReads from '$lib/widgets/UC10LiveDidReads.svelte';
	import UC15Session from '$lib/widgets/UC15Session.svelte';
	import UC16AuditLog from '$lib/widgets/UC16AuditLog.svelte';
	import UC18GatewayRouting from '$lib/widgets/UC18GatewayRouting.svelte';
	import UC19Historical from '$lib/widgets/UC19Historical.svelte';
	import SystemTopology from '$lib/widgets/SystemTopology.svelte';

	let selectedEcu = $state<EcuId>('cvc');
	let selectedDtc = $state<DtcEntry | null>(null);
	let dtcPage = $state(0);
	let showHistorical = $state(false);
	let filteredCount = $state(0);
	let faultRefreshNonce = $state(0);
	let liveFaults = $state<DtcEntry[]>([]);
	let health = $state<GatewayHealth | null>(null);
	let healthChecked = $state(false);
	let healthTimer: ReturnType<typeof setInterval> | null = null;
	let componentCount = $state<number | null>(null);
	let activeFaultCount = $state<number | null>(null);

	const PAGE_SIZE = 5;
	const MUTATIONS_ENABLED = import.meta.env.VITE_SIL_MUTATIONS_ENABLED === 'true';

	async function pollHealth() {
		health = await getGatewayHealth();
		healthChecked = true;
	}

	onMount(() => {
		void pollHealth();
		healthTimer = setInterval(() => {
			void pollHealth();
		}, 15_000);
		return subscribe((frame: TelemetryFrame) => {
			if (frame.type !== 'dtc') {
				return;
			}
			liveFaults = [frame.payload as DtcEntry, ...liveFaults].slice(0, 50);
		});
	});

	onDestroy(() => {
		if (healthTimer) clearInterval(healthTimer);
	});
</script>

<svelte:head>
	<title>Taktflow OpenSOVD - Live SIL</title>
</svelte:head>

<UC02DtcDetail dtc={selectedDtc} onClose={() => (selectedDtc = null)} />

<div class="min-h-screen bg-background text-foreground">
	<header class="bg-slate-900 text-white">
		<div class="mx-auto flex max-w-[1600px] flex-wrap items-center justify-between gap-4 px-6 py-4">
			<div class="flex items-center gap-3">
				<div
					class="flex h-9 w-9 items-center justify-center rounded-md bg-indigo-500 text-sm font-bold tracking-tight"
					aria-hidden="true"
				>
					TF
				</div>
				<div>
					<h1 class="text-lg font-semibold tracking-tight">OpenSOVD SIL Operations</h1>
					<p class="text-xs text-slate-400">
						Live software-in-the-loop bench — sovd-main · CDA · ECU simulator · MQTT
					</p>
				</div>
			</div>
			<div class="flex flex-wrap items-center gap-5">
				<nav class="flex items-center gap-4 text-sm font-medium text-slate-300">
					<a href="https://taktflow-systems.com/" class="hover:text-white">Taktflow Systems</a>
					<a href="/sovd/" class="hover:text-white">Engineering spec</a>
					<a href="/sovd/grafana/" class="hover:text-white">Grafana</a>
				</nav>
				{#if health}
					<span
						class="flex items-center gap-2 rounded-full border border-slate-700 bg-slate-800 px-3 py-1.5 text-xs font-medium"
					>
						<span class="h-2 w-2 rounded-full bg-emerald-400"></span>
						<span>API healthy</span>
						<span class="font-normal text-slate-400">v{health.version} · {health.latencyMs} ms</span>
					</span>
				{:else if healthChecked}
					<span
						class="flex items-center gap-2 rounded-full border border-slate-700 bg-slate-800 px-3 py-1.5 text-xs font-medium"
					>
						<span class="h-2 w-2 rounded-full bg-red-400"></span>
						<span>API unreachable</span>
					</span>
				{/if}
			</div>
		</div>
	</header>

	<main class="mx-auto flex max-w-[1600px] flex-col gap-6 px-6 py-6">
		<!-- What this is / how to use it -->
		<section class="rounded-lg border border-indigo-200 bg-indigo-50/80 px-4 py-3 shadow-sm">
			<p class="text-sm text-slate-800">
				<span class="font-semibold text-indigo-900">What you're looking at:</span>
				a live software-in-the-loop diagnostic bench. A real
				<span class="font-medium">sovd-main</span> gateway, a classic diagnostic adapter (CDA),
				and simulated ECUs run on this host — every panel below reads the public SOVD API in
				real time, nothing is mocked.
			</p>
			<p class="mt-1 text-xs text-slate-600">
				<span class="font-semibold">Try it:</span> select a component card below to scope the
				fault and component panels · click a fault row to open its freeze-frame detail ·
				watch your own reads appear in the audit log on the right. The public build is
				read-only; destructive actions are disabled.
			</p>
		</section>

		<!-- Hero stats -->
		<section class="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
			<div class="flex items-start gap-3 rounded-lg border border-border bg-card p-4 shadow-sm">
				<span class="mt-0.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-indigo-50 text-indigo-600">
					<Boxes class="h-5 w-5" />
				</span>
				<div>
					<p class="text-xs font-medium text-muted-foreground">Components online</p>
					<p class="mt-0.5 text-3xl font-semibold">{componentCount ?? '--'}</p>
					<p class="mt-0.5 text-xs text-muted-foreground">discovered via /sovd/v1/components</p>
				</div>
			</div>
			<div class="flex items-start gap-3 rounded-lg border border-border bg-card p-4 shadow-sm">
				<span class="mt-0.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-red-50 text-red-600">
					<TriangleAlert class="h-5 w-5" />
				</span>
				<div>
					<p class="text-xs font-medium text-muted-foreground">Active faults</p>
					<p class="mt-0.5 text-3xl font-semibold {activeFaultCount ? 'text-red-700' : ''}">
						{activeFaultCount ?? '--'}
					</p>
					<p class="mt-0.5 text-xs text-muted-foreground">across all components on the bench</p>
				</div>
			</div>
			<div class="flex items-start gap-3 rounded-lg border border-border bg-card p-4 shadow-sm">
				<span class="mt-0.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-emerald-50 text-emerald-600">
					<Gauge class="h-5 w-5" />
				</span>
				<div>
					<p class="text-xs font-medium text-muted-foreground">API latency</p>
					<p class="mt-0.5 text-3xl font-semibold">
						{health ? `${health.latencyMs}` : '--'}<span class="ml-1 text-base font-normal text-muted-foreground">ms</span>
					</p>
					<p class="mt-0.5 text-xs text-muted-foreground">
						{health
							? `SOVD DB ${health.sovdDb.status} · fault sink ${health.faultSink.status}`
							: 'gateway health probe'}
					</p>
				</div>
			</div>
			<div class="flex items-start gap-3 rounded-lg border border-border bg-card p-4 shadow-sm">
				<span class="mt-0.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-sky-50 text-sky-600">
					<RefreshCw class="h-5 w-5" />
				</span>
				<div>
					<p class="text-xs font-medium text-muted-foreground">Operation cycle</p>
					<p class="mt-0.5 text-3xl font-semibold capitalize">
						{health ? (health.operationCycle ?? 'idle') : '--'}
					</p>
					<p class="mt-0.5 text-xs text-muted-foreground">
						{MUTATIONS_ENABLED ? 'operator controls enabled' : 'public read-only mode'}
					</p>
				</div>
			</div>
		</section>

		<section class="space-y-3">
			<div>
				<h2 class="text-base font-semibold">Components</h2>
				<p class="text-xs text-muted-foreground">
					Discovered live from the gateway — LOCAL components are served by sovd-main itself,
					CDA components are routed through the classic diagnostic adapter. Select one to
					scope the panels below.
				</p>
			</div>
			<UC08ComponentCards
				selectedId={selectedEcu}
				onSelect={(id) => {
					selectedEcu = id;
					dtcPage = 0;
				}}
				onLoaded={(count) => (componentCount = count)}
			/>
		</section>

		<div class="grid gap-6 xl:grid-cols-[minmax(0,1.1fr)_minmax(0,1fr)_minmax(0,1fr)]">
			<!-- Faults -->
			<div class="flex flex-col gap-6">
				<section class="rounded-lg border border-border bg-card p-5 shadow-sm">
					<UC01DtcList
						componentId={selectedEcu}
						page={dtcPage}
						pageSize={PAGE_SIZE}
						refreshNonce={faultRefreshNonce}
						onSelect={(dtc) => (selectedDtc = dtc)}
						onPage={(pageNumber) => (dtcPage = pageNumber)}
						onTotalChange={(total) => (filteredCount = total)}
					/>
					<div class="mt-3 flex flex-wrap items-center justify-between gap-2 border-t border-border pt-3">
						<UC03ClearFaults
							componentId={selectedEcu}
							mutationsEnabled={MUTATIONS_ENABLED}
							onCleared={() => {
								faultRefreshNonce += 1;
								liveFaults = liveFaults.filter((fault) => fault.component !== selectedEcu);
							}}
						/>
						<UC04Pagination
							total={filteredCount}
							pageSize={PAGE_SIZE}
							page={dtcPage}
							onPage={(pageNumber) => (dtcPage = pageNumber)}
						/>
					</div>
				</section>

				<UC05FaultsTimeline
					extraFaults={liveFaults}
					refreshNonce={faultRefreshNonce}
					onCount={(count) => (activeFaultCount = count)}
				/>
			</div>

			<!-- Selected component -->
			<div class="flex flex-col gap-6">
				<section class="rounded-lg border border-border bg-card p-5 shadow-sm">
					<h3 class="text-base font-semibold">
						Component — <span class="font-mono text-indigo-700">{selectedEcu}</span>
					</h3>
					<p class="mb-3 mt-0.5 text-xs text-muted-foreground">
						Identity read from the entity's capability response; live values are polled
						data identifiers (DIDs). "--" means the component does not publish that value.
					</p>
					<UC09HwSwVersion componentId={selectedEcu} />
					<div class="my-4 border-t border-border"></div>
					<UC10LiveDidReads componentId={selectedEcu} />
				</section>

				<UC06Operations componentId={selectedEcu} controlEnabled={MUTATIONS_ENABLED} />
			</div>

			<!-- System -->
			<div class="flex flex-col gap-6">
				<UC18GatewayRouting />
				<UC15Session />
				<UC16AuditLog />
			</div>
		</div>

		<SystemTopology />

		<section class="space-y-3">
			<div class="flex items-center justify-between gap-3">
				<div>
					<h2 class="text-base font-semibold">Historical trends</h2>
					<p class="text-xs text-muted-foreground">
						Grafana view of bench telemetry over time (opens embedded, or via the header link).
					</p>
				</div>
				<button
					onclick={() => (showHistorical = !showHistorical)}
					class="rounded-md border border-border bg-card px-3 py-1.5 text-xs font-medium shadow-sm hover:bg-muted"
				>
					{showHistorical ? 'Hide panel' : 'Show panel'}
				</button>
			</div>
			<UC19Historical visible={showHistorical} grafanaUrl={import.meta.env.VITE_GRAFANA_URL ?? ''} />
		</section>
	</main>
</div>
