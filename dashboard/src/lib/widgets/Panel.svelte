<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- Shared card shell: consistent header, collapsible body, hover hint. -->
<script lang="ts">
	import { ChevronDown, Info } from 'lucide-svelte';
	import type { Snippet } from 'svelte';

	interface Props {
		title: string;
		/** Muted text after the title, e.g. the scoped component or a count. */
		meta?: string;
		/** One-sentence explanation shown as a hover tooltip on the header. */
		hint: string;
		/** Start expanded (primary panels) or collapsed (secondary panels). */
		open?: boolean;
		/** Dark terminal-style variant. */
		dark?: boolean;
		/** Tailwind classes for the icon chip, e.g. "bg-red-50 text-red-600". */
		chip?: string;
		icon?: Snippet;
		/** Right-aligned header controls; clicks here do not toggle the panel. */
		actions?: Snippet;
		children: Snippet;
	}

	let {
		title,
		meta,
		hint,
		open = true,
		dark = false,
		chip = 'bg-indigo-50 text-indigo-600',
		icon,
		actions,
		children
	}: Props = $props();
</script>

<details
	{open}
	class="group rounded-lg border shadow-sm {dark
		? 'border-slate-800 bg-slate-900 text-slate-300'
		: 'border-border bg-card'}"
>
	<summary
		title={hint}
		class="flex cursor-pointer list-none items-center gap-2 p-5 [&::-webkit-details-marker]:hidden"
	>
		{#if icon}
			<span class="flex h-6 w-6 shrink-0 items-center justify-center rounded-md {chip}">
				{@render icon()}
			</span>
		{/if}
		<h3
			class="flex min-w-0 items-baseline gap-2 text-base font-semibold {dark ? 'text-white' : ''}"
		>
			<span class="truncate">{title}</span>
			{#if meta}
				<span
					class="truncate text-sm font-normal {dark ? 'text-slate-400' : 'text-muted-foreground'}"
				>
					{meta}
				</span>
			{/if}
		</h3>
		{#if actions}
			<!-- Container only stops header controls from toggling the panel. -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<span
				class="ml-auto flex shrink-0 items-center gap-2"
				onclick={(e) => e.stopPropagation()}
				onkeydown={(e) => e.stopPropagation()}
			>
				{@render actions()}
			</span>
		{/if}
		<span
			class="{actions ? '' : 'ml-auto'} flex shrink-0 items-center gap-2 {dark
				? 'text-slate-500'
				: 'text-muted-foreground/70'}"
		>
			<Info class="h-4 w-4" aria-hidden="true" />
			<ChevronDown class="h-4 w-4 transition-transform group-open:rotate-180" />
		</span>
	</summary>
	<div class="px-5 pb-5">
		{@render children()}
	</div>
</details>
