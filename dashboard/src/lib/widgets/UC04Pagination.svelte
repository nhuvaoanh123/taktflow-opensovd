<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- Pagination control. -->
<script lang="ts">
	interface Props {
		total: number;
		pageSize?: number;
		page: number;
		onPage: (p: number) => void;
	}

	let { total, pageSize = 5, page, onPage }: Props = $props();

	const pageCount = $derived(Math.max(1, Math.ceil(total / pageSize)));

	function pages(): number[] {
		const list: number[] = [];
		for (let i = 0; i < pageCount; i++) list.push(i);
		return list;
	}
</script>

{#if pageCount > 1}
	<nav class="flex items-center gap-1 text-xs" aria-label="Pagination">
		<button
			disabled={page === 0}
			onclick={() => onPage(0)}
			class="rounded border border-border px-2 py-0.5 disabled:opacity-30 hover:bg-muted"
		>
			First
		</button>
		<button
			disabled={page === 0}
			onclick={() => onPage(page - 1)}
			class="rounded border border-border px-2 py-0.5 disabled:opacity-30 hover:bg-muted"
		>
			Prev
		</button>

		{#each pages() as p (p)}
			<button
				onclick={() => onPage(p)}
				class="rounded border px-2 py-0.5 {p === page
					? 'border-slate-900 bg-slate-900 text-white'
					: 'border-border hover:bg-muted'}"
				aria-current={p === page ? 'page' : undefined}
			>
				{p + 1}
			</button>
		{/each}

		<button
			disabled={page >= pageCount - 1}
			onclick={() => onPage(page + 1)}
			class="rounded border border-border px-2 py-0.5 disabled:opacity-30 hover:bg-muted"
		>
			Next
		</button>
		<button
			disabled={page >= pageCount - 1}
			onclick={() => onPage(pageCount - 1)}
			class="rounded border border-border px-2 py-0.5 disabled:opacity-30 hover:bg-muted"
		>
			Last
		</button>

		<span class="ml-2 text-muted-foreground">
			{total} items / {pageCount} pages
		</span>
	</nav>
{/if}
