<script lang="ts">
  import { onMount } from 'svelte'
  import * as Table from '$lib/components/ui/table'
  import { Button } from '$lib/components/ui/button'
  import RefreshCw from 'lucide-svelte/icons/refresh-cw'
  import Folder from 'lucide-svelte/icons/folder'
  import FileIcon from 'lucide-svelte/icons/file'
  import ChevronRight from 'lucide-svelte/icons/chevron-right'
  import ArrowLeft from 'lucide-svelte/icons/arrow-left'
  import Download from 'lucide-svelte/icons/download'

  interface Props {
    bucket: string
    onBack: () => void
    onPrefixChange?: (prefix: string, breadcrumbs: { label: string; prefix: string }[]) => void
  }
  let { bucket, onBack, onPrefixChange }: Props = $props()

  interface S3File {
    key: string
    size: number
    lastModified: string
    etag: string
  }

  let prefix = $state('')
  let files = $state<S3File[]>([])
  let prefixes = $state<string[]>([])
  let loading = $state(true)

  async function fetchObjects() {
    loading = true
    try {
      const params = new URLSearchParams({ prefix, delimiter: '/' })
      const res = await fetch(`/api/buckets/${encodeURIComponent(bucket)}/objects?${params}`)
      if (res.ok) {
        const data = await res.json()
        files = data.files
        prefixes = data.prefixes
      }
    } catch {
      // ignore
    } finally {
      loading = false
    }
  }

  function notifyPrefix() {
    onPrefixChange?.(prefix, breadcrumbs)
  }

  export function navigateTo(newPrefix: string) {
    prefix = newPrefix
    fetchObjects()
    notifyPrefix()
  }

  export function goUp() {
    if (!prefix) {
      onBack()
      return
    }
    const trimmed = prefix.slice(0, -1)
    const lastSlash = trimmed.lastIndexOf('/')
    prefix = lastSlash >= 0 ? trimmed.slice(0, lastSlash + 1) : ''
    fetchObjects()
    notifyPrefix()
  }

  // Extract display name from full key/prefix
  function displayName(fullPath: string): string {
    const trimmed = fullPath.endsWith('/') ? fullPath.slice(0, -1) : fullPath
    const lastSlash = trimmed.lastIndexOf('/')
    return lastSlash >= 0 ? trimmed.slice(lastSlash + 1) : trimmed
  }

  function formatSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`
  }

  function formatDate(iso: string): string {
    try {
      return new Date(iso).toLocaleString()
    } catch {
      return iso
    }
  }

  // Breadcrumb segments
  let breadcrumbs = $derived.by(() => {
    const parts = prefix.split('/').filter(Boolean)
    const crumbs: { label: string; prefix: string }[] = [
      { label: bucket, prefix: '' },
    ]
    let acc = ''
    for (const part of parts) {
      acc += part + '/'
      crumbs.push({ label: part, prefix: acc })
    }
    return crumbs
  })

  function downloadUrl(key: string): string {
    return `/api/buckets/${encodeURIComponent(bucket)}/download/${key}`
  }

  onMount(fetchObjects)
</script>

<div class="flex flex-col gap-4">
  {#if loading && files.length === 0 && prefixes.length === 0}
    <p class="text-sm text-muted-foreground">Loading...</p>
  {:else if files.length === 0 && prefixes.length === 0}
    <div class="flex flex-col items-center gap-2 py-12 text-muted-foreground">
      <Folder class="size-10 opacity-30" />
      <p class="text-sm">Empty</p>
    </div>
  {:else}
    <Table.Root>
      <Table.Header>
        <Table.Row>
          <Table.Head>Name</Table.Head>
          <Table.Head class="w-28 text-right">Size</Table.Head>
          <Table.Head class="w-48">Modified</Table.Head>
          <Table.Head class="w-10"></Table.Head>
        </Table.Row>
      </Table.Header>
      <Table.Body>
        {#each prefixes as p}
          <Table.Row class="cursor-pointer" onclick={() => navigateTo(p)}>
            <Table.Cell>
              <span class="flex items-center gap-2">
                <Folder class="size-4 shrink-0 text-muted-foreground" />
                <span class="font-medium">{displayName(p)}/</span>
              </span>
            </Table.Cell>
            <Table.Cell class="text-right text-muted-foreground">—</Table.Cell>
            <Table.Cell class="text-muted-foreground">—</Table.Cell>
            <Table.Cell></Table.Cell>
          </Table.Row>
        {/each}
        {#each files as file}
          <Table.Row class="cursor-pointer" onclick={() => window.location.href = downloadUrl(file.key)}>
            <Table.Cell>
              <span class="flex items-center gap-2">
                <FileIcon class="size-4 shrink-0 text-muted-foreground" />
                <span class="font-medium">{displayName(file.key)}</span>
              </span>
            </Table.Cell>
            <Table.Cell class="text-right text-muted-foreground">{formatSize(file.size)}</Table.Cell>
            <Table.Cell class="text-muted-foreground">{formatDate(file.lastModified)}</Table.Cell>
            <Table.Cell class="w-10">
              <a href={downloadUrl(file.key)} class="text-muted-foreground hover:text-foreground" onclick={(e) => e.stopPropagation()}>
                <Download class="size-4" />
              </a>
            </Table.Cell>
          </Table.Row>
        {/each}
      </Table.Body>
    </Table.Root>
  {/if}
</div>
