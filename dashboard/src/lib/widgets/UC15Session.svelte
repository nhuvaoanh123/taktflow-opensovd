<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC15 - Session id, security level, timeout countdown (FR-7.1, FR-7.2) -->
<script lang="ts">
	import { onDestroy, onMount } from 'svelte';

	import { getSession } from '$lib/api/sovdClient';
	import type { SessionInfo } from '$lib/types/sovd';

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

	function secBar(level: number): string {
		const filled = '#'.repeat(level);
		const empty = '-'.repeat(3 - level);
		return filled + empty;
	}
</script>

<div class="rounded-lg border border-border bg-card p-5 text-sm shadow-sm">
	<h3 class="mb-3 text-base font-semibold">Session</h3>
	{#if session}
		<dl class="space-y-1">
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
				<dd class="font-mono">{secBar(session.securityLevel)} L{session.securityLevel}</dd>
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
		<p class="text-muted-foreground">
			{loading ? 'Loading session...' : 'Session route unavailable.'}
		</p>
	{/if}
</div>
