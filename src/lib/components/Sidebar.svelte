<script lang="ts">
  import {
    LayoutDashboard,
    History,
    Settings,
    ScrollText,
    CloudDownload,
    Info,
    Heart,
    PanelLeftClose,
    PanelLeftOpen,
    ShieldCheck,
    ShieldAlert,
    User,
    FlaskConical,
  } from "@lucide/svelte";
  import Logo from "./Logo.svelte";
  import { app } from "../stores/app.svelte";
  import { t, type MessageKey } from "../i18n/index.svelte";
  import type { Page } from "../types";

  let { collapsed = $bindable(false) }: { collapsed?: boolean } = $props();

  const items: { page: Page; key: MessageKey; icon: typeof LayoutDashboard }[] = [
    { page: "dashboard", key: "nav.dashboard", icon: LayoutDashboard },
    { page: "history", key: "nav.history", icon: History },
    { page: "settings", key: "nav.settings", icon: Settings },
    { page: "logs", key: "nav.logs", icon: ScrollText },
    { page: "update", key: "nav.update", icon: CloudDownload },
    { page: "about", key: "nav.about", icon: Info },
    { page: "social", key: "nav.social", icon: Heart },
  ];

  const status = $derived.by(() => {
    const info = app.info;
    if (!info) return null;
    if (info.mock) return { icon: FlaskConical, key: "status.mock" as MessageKey, tone: "accent" };
    if (info.os === "windows") {
      return info.elevated
        ? { icon: ShieldCheck, key: "status.admin" as MessageKey, tone: "success" }
        : { icon: ShieldAlert, key: "status.readOnly" as MessageKey, tone: "warning" };
    }
    return { icon: User, key: "status.user" as MessageKey, tone: "accent" };
  });
</script>

<nav class="sidebar" class:collapsed aria-label={t("nav.label")}>
  <div class="brand">
    <Logo size={30} />
    {#if !collapsed}
      <div class="brand-text">
        <span class="name">RandomMac</span>
        <span class="version subtle">v{app.info?.version ?? ""}</span>
      </div>
    {/if}
  </div>

  <ul>
    {#each items as item, i (item.page)}
      {@const Icon = item.icon}
      <li>
        <button
          class="nav-item"
          class:active={app.page === item.page}
          aria-current={app.page === item.page ? "page" : undefined}
          title={collapsed ? `${t(item.key)} (Ctrl+${i + 1})` : `Ctrl+${i + 1}`}
          onclick={() => app.navigate(item.page)}
        >
          <Icon size={18} aria-hidden="true" />
          <span class:sr-only={collapsed}>{t(item.key)}</span>
          {#if item.page === "update" && app.update?.available}
            <span class="dot" aria-hidden="true"></span>
          {/if}
        </button>
      </li>
    {/each}
  </ul>

  <div class="bottom">
    {#if status}
      {@const StatusIcon = status.icon}
      <div class="status {status.tone}" title={t(status.key)}>
        <StatusIcon size={16} aria-hidden="true" />
        <span class:sr-only={collapsed}>{t(status.key)}</span>
      </div>
    {/if}
    <button
      class="btn ghost icon collapse"
      onclick={() => (collapsed = !collapsed)}
      aria-label={collapsed ? t("nav.expand") : t("nav.collapse")}
      title={collapsed ? t("nav.expand") : t("nav.collapse")}
    >
      {#if collapsed}<PanelLeftOpen size={18} aria-hidden="true" />{:else}<PanelLeftClose
          size={18}
          aria-hidden="true"
        />{/if}
    </button>
  </div>
</nav>

<style>
  .sidebar {
    width: var(--sidebar-w);
    background: var(--sidebar);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    padding: 14px 10px;
    gap: 14px;
    transition: width 0.18s var(--ease);
    overflow: hidden;
  }
  .sidebar.collapsed {
    width: var(--sidebar-w-collapsed);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 8px 8px;
    min-height: 44px;
  }
  .brand-text {
    display: flex;
    flex-direction: column;
    line-height: 1.2;
  }
  .name {
    font-weight: 700;
    font-size: 15px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
  }
  .nav-item {
    position: relative;
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 12px;
    border-radius: var(--radius-sm);
    border: none;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    text-align: left;
    font-weight: 500;
  }
  .collapsed .nav-item {
    justify-content: center;
    padding: 10px 0;
  }
  .nav-item:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .nav-item.active {
    background: var(--accent-soft);
    color: var(--accent);
  }
  .nav-item.active::before {
    content: "";
    position: absolute;
    left: 0;
    top: 8px;
    bottom: 8px;
    width: 3px;
    border-radius: 3px;
    background: var(--accent);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--danger);
    margin-left: auto;
  }
  .collapsed .dot {
    position: absolute;
    top: 6px;
    right: 14px;
  }
  .bottom {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
  }
  .collapsed .bottom {
    flex-direction: column;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    font-weight: 600;
    padding: 5px 10px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--text-muted);
    overflow: hidden;
    white-space: nowrap;
  }
  .status.success {
    color: var(--success);
    background: var(--success-soft);
  }
  .status.warning {
    color: var(--warning);
    background: var(--warning-soft);
  }
  .status.accent {
    color: var(--accent);
    background: var(--accent-soft);
  }
</style>
