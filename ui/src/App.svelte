<script lang="ts">
  import { onMount } from "svelte";
  import Login from "./lib/Login.svelte";
  import BucketList from "./lib/BucketList.svelte";
  import ObjectBrowser from "./lib/ObjectBrowser.svelte";
  import Home from "lucide-svelte/icons/home";
  import LogOut from "lucide-svelte/icons/log-out";
  import PanelLeftClose from "lucide-svelte/icons/panel-left-close";
  import PanelLeftOpen from "lucide-svelte/icons/panel-left-open";
  import FlaskConical from "lucide-svelte/icons/flask-conical";
  import ArrowLeft from "lucide-svelte/icons/arrow-left";
  import ChevronRight from "lucide-svelte/icons/chevron-right";

  let authenticated = $state<boolean | null>(null);
  let collapsed = $state(false);
  let selectedBucket = $state<string | null>(null);
  let objectBrowserRef = $state<ObjectBrowser | null>(null);
  let currentPrefix = $state("");
  let currentBreadcrumbs = $state<{ label: string; prefix: string }[]>([]);

  onMount(async () => {
    try {
      const res = await fetch("/api/auth/check");
      authenticated = res.ok;
    } catch {
      authenticated = false;
    }
  });

  function handleLogin() {
    authenticated = true;
  }

  async function handleLogout() {
    await fetch("/api/auth/logout", { method: "POST" });
    authenticated = false;
  }
</script>

{#if authenticated === null}
  <!-- loading -->
{:else if !authenticated}
  <Login onLogin={handleLogin} />
{:else}
  <div class="relative flex h-screen bg-background">
    <nav
      class="relative flex flex-col border-r bg-sidebar-background transition-[width] duration-200"
      class:w-56={!collapsed}
      class:w-14={collapsed}
      style="border-color: var(--cool-sidebar-border);"
    >
      <!-- Collapse/expand toggle — positioned outside the nav edge -->
      <button
        onclick={() => (collapsed = !collapsed)}
        class="absolute top-4 -right-3 z-10 flex size-6 items-center justify-center rounded-full border bg-card text-muted-foreground transition-colors hover:text-foreground"
        style="border-color: var(--cool-sidebar-border);"
        title={collapsed ? "Expand sidebar" : "Collapse sidebar"}
      >
        {#if collapsed}
          <PanelLeftOpen class="size-3" />
        {:else}
          <PanelLeftClose class="size-3" />
        {/if}
      </button>

      <!-- Logo -->
      <div
        class="flex h-14 items-center overflow-hidden"
        class:px-4={!collapsed}
        class:justify-center={collapsed}
        style="border-bottom: 1px solid var(--cool-sidebar-border);"
      >
        {#if collapsed}
          <FlaskConical class="size-5 text-primary" />
        {:else}
          <span
            class="text-lg font-bold tracking-tight text-foreground whitespace-nowrap"
            >MaxIO</span
          >
        {/if}
      </div>

      <!-- Nav items -->
      <div class="flex flex-1 flex-col gap-0.5 p-2">
        <button
          onclick={() => { selectedBucket = null; currentPrefix = ""; currentBreadcrumbs = []; }}
          class="flex h-9 w-full items-center rounded-sm text-left text-sm font-medium transition-colors overflow-hidden"
          class:gap-3={!collapsed}
          class:px-3={!collapsed}
          class:justify-center={collapsed}
          style="background: var(--cool-sidebar-active-bg); color: var(--cool-sidebar-active-fg);"
          title="Buckets"
        >
          <Home class="size-4 shrink-0" />
          {#if !collapsed}<span class="whitespace-nowrap">Buckets</span>{/if}
        </button>
      </div>

      <!-- Bottom: logout -->
      <div
        class="p-2"
        style="border-top: 1px solid var(--cool-sidebar-border);"
      >
        <button
          onclick={handleLogout}
          class="flex h-9 w-full items-center rounded-sm text-left text-sm font-medium text-muted-foreground transition-colors hover:text-foreground overflow-hidden"
          class:gap-3={!collapsed}
          class:px-3={!collapsed}
          class:justify-center={collapsed}
          title="Sign out"
          style="background: transparent;"
          onmouseenter={(e) =>
            (e.currentTarget.style.background = "var(--cool-sidebar-hover)")}
          onmouseleave={(e) =>
            (e.currentTarget.style.background = "transparent")}
        >
          <LogOut class="size-4 shrink-0" />
          {#if !collapsed}<span class="whitespace-nowrap">Sign out</span>{/if}
        </button>
      </div>
    </nav>

    <main class="flex flex-1 flex-col overflow-hidden">
      <!-- Header bar — aligned with sidebar h-14 -->
      <div
        class="flex h-14 shrink-0 items-center gap-2 px-6"
        style="border-bottom: 1px solid var(--cool-sidebar-border);"
      >
        {#if selectedBucket}
          <button
            onclick={() => objectBrowserRef?.goUp()}
            class="shrink-0 rounded-sm p-1 text-muted-foreground transition-colors hover:text-foreground"
          >
            <ArrowLeft class="size-4" />
          </button>
          <nav class="flex items-center gap-1 text-sm overflow-x-auto">
            <button
              class="text-muted-foreground hover:text-foreground transition-colors shrink-0"
              onclick={() => { selectedBucket = null; currentPrefix = ""; currentBreadcrumbs = []; }}>Buckets</button
            >
            <ChevronRight class="size-3 shrink-0 text-muted-foreground" />
            {#if currentBreadcrumbs.length > 1}
              {#each currentBreadcrumbs as crumb, i}
                {#if i < currentBreadcrumbs.length - 1}
                  <button
                    class="text-muted-foreground hover:text-foreground transition-colors shrink-0"
                    onclick={() => objectBrowserRef?.navigateTo(crumb.prefix)}
                  >{crumb.label}</button>
                  <ChevronRight class="size-3 shrink-0 text-muted-foreground" />
                {:else}
                  <span class="font-semibold shrink-0">{crumb.label}</span>
                {/if}
              {/each}
            {:else}
              <span class="font-semibold shrink-0">{selectedBucket}</span>
            {/if}
          </nav>
        {:else}
          <h2 class="text-lg font-semibold">Buckets</h2>
        {/if}
      </div>
      <!-- Scrollable content -->
      <div class="flex-1 overflow-auto p-6">
        {#if selectedBucket}
          <ObjectBrowser
            bind:this={objectBrowserRef}
            bucket={selectedBucket}
            onBack={() => (selectedBucket = null)}
            onPrefixChange={(p, crumbs) => { currentPrefix = p; currentBreadcrumbs = crumbs; }}
          />
        {:else}
          <BucketList onSelect={(name) => (selectedBucket = name)} />
        {/if}
      </div>
    </main>
  </div>
{/if}
