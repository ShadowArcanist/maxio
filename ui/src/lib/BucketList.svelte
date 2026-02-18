<script lang="ts">
  import { onMount } from 'svelte'
  import * as Table from '$lib/components/ui/table'
  import { Button } from '$lib/components/ui/button'
  import RefreshCw from 'lucide-svelte/icons/refresh-cw'
  import Database from 'lucide-svelte/icons/database'

  interface Props {
    onSelect: (bucket: string) => void
  }
  let { onSelect }: Props = $props()

  interface Bucket {
    name: string
    createdAt: string
  }

  let buckets = $state<Bucket[]>([])
  let loading = $state(true)

  async function fetchBuckets() {
    loading = true
    try {
      const res = await fetch('/api/buckets')
      if (res.ok) {
        const data = await res.json()
        buckets = data.buckets
      }
    } catch {
      // ignore
    } finally {
      loading = false
    }
  }

  function formatDate(iso: string): string {
    try {
      return new Date(iso).toLocaleString()
    } catch {
      return iso
    }
  }

  onMount(fetchBuckets)
</script>

<div class="flex flex-col gap-4">
  {#if loading && buckets.length === 0}
    <p class="text-sm text-muted-foreground">Loading...</p>
  {:else if buckets.length === 0}
    <div class="flex flex-col items-center gap-2 py-12 text-muted-foreground">
      <Database class="size-10 opacity-30" />
      <p class="text-sm">No buckets yet</p>
    </div>
  {:else}
    <Table.Root>
      <Table.Header>
        <Table.Row>
          <Table.Head>Name</Table.Head>
          <Table.Head>Created</Table.Head>
        </Table.Row>
      </Table.Header>
      <Table.Body>
        {#each buckets as bucket}
          <Table.Row class="cursor-pointer" onclick={() => onSelect(bucket.name)}>
            <Table.Cell class="font-medium">{bucket.name}</Table.Cell>
            <Table.Cell class="text-muted-foreground">{formatDate(bucket.createdAt)}</Table.Cell>
          </Table.Row>
        {/each}
      </Table.Body>
    </Table.Root>
  {/if}
</div>
