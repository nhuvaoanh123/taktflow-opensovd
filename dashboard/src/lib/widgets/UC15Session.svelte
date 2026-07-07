<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC15 - Session id, security level, timeout countdown (FR-7.1, FR-7.2) -->
<script lang="ts">
	import { KeyRound } from 'lucide-svelte';
	import { onDestroy, onMount } from 'svelte';

	import { getSession } from '$lib/api/sovdClient';
	import type { SessionInfo } from '$lib/types/sovd';
	import Panel from './Panel.svelte';

	let session = $state<SessionInfo | null>(null);
	let loading = $state(true);
	let remaining = $state<number | null>(null);
	let timer: ReturnType<typeof setInterval> | null = null;

	function calcRemaining(): number | null {
		if (!session?.expiresAt) {
			return null;
		}
		return Math.max(0, Math.floor((new Date(session.expiresAt).getTime() - Date.now()) / 1000));
	}

	async function load() {
		session = await getSession();
		loading = false;
		remaining = calcRemaining();
	}

	onMount(() => {
		void load();
		let ticks = 0;
		timer = setInterval(() => {
			remaining = calcRemaining();
			ticks += 1;
			if (ticks >= 5 || remaining === 0 || session?.active === false) {
				ticks = 0;
				void load();
			}
		}, 1000);
	});

	onDestroy(() => {
		if (timer) clearInterval(timer);
	});

	const LEVEL_COLOR: Record<string, string> = {
		default: 'text-slate-700',
		programming: 'text-amber-700',
		extended: 'text-blue-700'
	};
</script>

<Panel
	title="Session"
	meta={session ? `${session.level}, L${session.securityLevel}` : undefined}
	hint="The session issued to this page by the gateway — the diagnostic level and security state determine which operations are permitted."
	open={false}
	chip="bg-violet-50 text-violet-600"
>
	{#snippet icon()}<KeyRound class="h-3.5 w-3.5" />{/snippet}
	{#if session}
		<dl class="space-y-1 text-sm">
			<div class="flex justify-between gap-3">
				<dt class="text-muted-foreground">Session ID</dt>
				<dd class="truncate font-mono text-xs leading-5">{session.sessionId}</dd>
			</div>
			<div class="flex justify-between">
				<dt class="text-muted-foreground">Level</dt>
				<dd class="font-semibold {session.active === false ? 'text-muted-foreground' : LEVEL_COLOR[session.level]}">
					{session.active === false ? 'inactive' : session.level}
				</dd>
			</div>
			<div class="flex justify-between">
				<dt class="text-muted-foreground">Security</dt>
				<dd class="font-medium">Security level {session.securityLevel}</dd>
			</div>
			<div class="flex justify-between">
				<dt class="text-muted-foreground">Expires in</dt>
				<dd
					class="tabular-nums font-semibold {session.active === false || remaining === null
						? 'text-muted-foreground'
						: remaining < 30
							? 'text-red-700'
							: 'text-emerald-700'}"
				>
					{remaining !== null ? `${remaining}s` : '--'}
				</dd>
			</div>
		</dl>
	{:else}
		<p class="text-sm text-muted-foreground">
			{loading ? 'Loading session...' : 'Session route unavailable.'}
		</p>
	{/if}
</Panel>
