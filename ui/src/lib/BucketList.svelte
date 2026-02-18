<script lang="ts">
  import { onMount } from 'svelte'
  import * as Table from '$lib/components/ui/table'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import Database from 'lucide-svelte/icons/database'
  import Plus from 'lucide-svelte/icons/plus'
  import Trash2 from 'lucide-svelte/icons/trash-2'

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
  let error = $state<string | null>(null)
  let showCreate = $state(false)
  let newBucketName = $state('')
  let creating = $state(false)

  async function fetchBuckets() {
    loading = true
    error = null
    try {
      const res = await fetch('/api/buckets')
      if (res.ok) {
        const data = await res.json()
        buckets = data.buckets
      } else {
        error = `Failed to load buckets (${res.status})`
      }
    } catch {
      error = 'Failed to connect to server'
    } finally {
      loading = false
    }
  }

  async function createBucket() {
    if (!newBucketName.trim()) return
    creating = true
    error = null
    try {
      const res = await fetch('/api/buckets', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: newBucketName.trim() }),
      })
      if (res.ok) {
        newBucketName = ''
        showCreate = false
        await fetchBuckets()
      } else {
        const data = await res.json()
        error = data.error || `Failed to create bucket (${res.status})`
      }
    } catch {
      error = 'Failed to connect to server'
    } finally {
      creating = false
    }
  }

  async function deleteBucket(name: string, e: Event) {
    e.stopPropagation()
    if (!confirm(`Delete bucket "${name}"? This cannot be undone.`)) return
    error = null
    try {
      const res = await fetch(`/api/buckets/${encodeURIComponent(name)}`, { method: 'DELETE' })
      if (res.ok) {
        await fetchBuckets()
      } else {
        const data = await res.json()
        error = data.error || `Failed to delete bucket (${res.status})`
      }
    } catch {
      error = 'Failed to connect to server'
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
  {#if error}
    <div class="rounded-sm border border-destructive/50 bg-destructive/10 px-4 py-2 text-sm text-destructive">
      {error}
    </div>
  {/if}

  <div class="flex items-center gap-2">
    {#if showCreate}
      <form onsubmit={(e) => { e.preventDefault(); createBucket() }} class="flex items-center gap-2">
        <Input
          type="text"
          bind:value={newBucketName}
          placeholder="bucket-name"
          class="h-8 w-48"
          disabled={creating}
        />
        <Button type="submit" variant="brand" class="h-8" disabled={creating || !newBucketName.trim()}>
          {creating ? 'Creating...' : 'Create'}
        </Button>
        <Button type="button" variant="ghost" class="h-8" onclick={() => { showCreate = false; newBucketName = '' }}>
          Cancel
        </Button>
      </form>
    {:else}
      <Button variant="brand" class="h-8" onclick={() => (showCreate = true)}>
        <Plus class="size-4 mr-1" /> Create Bucket
      </Button>
    {/if}
  </div>

  {#if loading && buckets.length === 0}
    <p class="text-sm text-muted-foreground">Loading...</p>
  {:else if buckets.length === 0 && !error}
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
          <Table.Head class="w-10"></Table.Head>
        </Table.Row>
      </Table.Header>
      <Table.Body>
        {#each buckets as bucket}
          <Table.Row class="cursor-pointer" onclick={() => onSelect(bucket.name)}>
            <Table.Cell class="font-medium">{bucket.name}</Table.Cell>
            <Table.Cell class="text-muted-foreground">{formatDate(bucket.createdAt)}</Table.Cell>
            <Table.Cell class="w-10">
              <button
                class="text-muted-foreground hover:text-destructive transition-colors"
                onclick={(e) => deleteBucket(bucket.name, e)}
                title="Delete bucket"
              >
                <Trash2 class="size-4" />
              </button>
            </Table.Cell>
          </Table.Row>
        {/each}
      </Table.Body>
    </Table.Root>
  {/if}
</div>
