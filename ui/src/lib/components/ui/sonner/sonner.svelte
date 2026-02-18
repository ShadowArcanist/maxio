<script lang="ts">
  import { Toaster } from 'svelte-sonner'

  interface Props {
    theme?: 'light' | 'dark'
  }
  let { theme = 'light' }: Props = $props()
</script>

<Toaster
  {theme}
  position="bottom-right"
  richColors
  closeButton
  duration={4000}
  toastOptions={{
    classes: {
      toast: 'group !rounded-sm !shadow-lg !font-sans !text-sm !bg-[var(--cool-bg-card)] !text-[var(--cool-fg)] !border !border-[var(--cool-border)]',
      title: '!font-medium',
      description: '!text-[var(--cool-fg-muted)]',
      success: '!border-l-4 !border-l-[var(--cool-success)]',
      error: '!border-l-4 !border-l-[var(--cool-error)]',
      warning: '!border-l-4 !border-l-[#f59e0b]',
      info: '!border-l-4 !border-l-[var(--cool-accent)]',
      closeButton: '!bg-[var(--cool-bg-card)] !border-[var(--cool-border)] !text-[var(--cool-fg-muted)] hover:!text-[var(--cool-fg)]',
    },
  }}
/>
