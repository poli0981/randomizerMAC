<script lang="ts">
  import {
    Coffee,
    Heart,
    HandCoins,
    Bug,
    Mail,
    Gamepad2,
    MessageCircle,
    Send,
    Video,
    AtSign,
    Cloud,
    Hash,
    Wallet,
  } from "@lucide/svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import { api } from "../api";
  import { app } from "../stores/app.svelte";
  import { t, type MessageKey } from "../i18n/index.svelte";

  type Link = { label: string; key?: MessageKey; url: string; icon: typeof Coffee };

  const donate: Link[] = [
    { label: "Ko-fi", url: "https://ko-fi.com/skullmute", icon: Coffee },
    { label: "Buy Me a Coffee", url: "https://www.buymeacoffee.com/skullmute", icon: Coffee },
    { label: "Patreon", url: "https://www.patreon.com/skullmute", icon: Heart },
    { label: "PayPal", url: "https://paypal.me/DungDang212", icon: Wallet },
  ];

  const networks: Link[] = [
    { label: "X", url: "https://x.com/SkullMute0011", icon: AtSign },
    { label: "YouTube", url: "https://youtube.com/@SkullMute", icon: Video },
    {
      label: "Discord",
      key: "social.discordRepo",
      url: "https://discord.gg/2aNR3aVt",
      icon: MessageCircle,
    },
    {
      label: "Discord",
      key: "social.discordGame",
      url: "https://discord.gg/kDM9GMu5vm",
      icon: MessageCircle,
    },
    { label: "Patreon", url: "https://www.patreon.com/skullmute", icon: Heart },
    { label: "Ko-fi", url: "https://ko-fi.com/skullmute", icon: HandCoins },
    {
      label: "Steam",
      url: "https://steamcommunity.com/profiles/76561199544666292/",
      icon: Gamepad2,
    },
    { label: "Bluesky", url: "https://bsky.app/profile/skullmute0011.bsky.social", icon: Cloud },
    { label: "Mastodon", url: "https://mastodon.social/@skullmute1122", icon: Hash },
    { label: "Telegram", key: "social.telegramBot", url: "https://t.me/my_skull_bot", icon: Send },
    { label: "Telegram", url: "https://t.me/SkullMute0011", icon: Send },
  ];

  function open(url: string) {
    api.openUrl(url).catch((e) => app.toastError(e));
  }
</script>

<PageHeader title={t("social.title")} subtitle={t("social.subtitle")} />

<div class="grid">
  <section class="card" aria-labelledby="donate-title">
    <h2 id="donate-title" class="card-title">{t("social.donate")}</h2>
    <p class="subtle">{t("social.donate.desc")}</p>
    <div class="buttons">
      {#each donate as d (d.url)}
        {@const Icon = d.icon}
        <button class="btn" onclick={() => open(d.url)}
          ><Icon size={16} aria-hidden="true" />{d.label}</button
        >
      {/each}
    </div>
  </section>

  <section class="card" aria-labelledby="bug-title">
    <h2 id="bug-title" class="card-title">{t("social.reportBug")}</h2>
    <p class="subtle">{t("social.reportBug.desc")}</p>
    <div class="buttons">
      <button
        class="btn primary"
        onclick={() =>
          open("https://github.com/poli0981/randomizerMAC/issues/new?template=bug_report.yml")}
      >
        <Bug size={16} aria-hidden="true" />{t("social.reportBug")}
      </button>
    </div>
  </section>

  <section class="card wide" aria-labelledby="net-title">
    <h2 id="net-title" class="card-title">{t("social.networks")}</h2>
    <div class="networks">
      {#each networks as n (n.url)}
        {@const Icon = n.icon}
        <button class="network" onclick={() => open(n.url)}>
          <Icon size={18} aria-hidden="true" />
          <span>{n.key ? t(n.key) : n.label}</span>
        </button>
      {/each}
    </div>
  </section>

  <section class="card wide" aria-labelledby="contact-title">
    <h2 id="contact-title" class="card-title">{t("social.contact")}</h2>
    <button class="btn" onclick={() => open("mailto:lopop05905@proton.me")}
      ><Mail size={16} aria-hidden="true" />lopop05905@proton.me</button
    >
  </section>
</div>

<style>
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }
  .wide {
    grid-column: 1 / -1;
  }
  @media (max-width: 900px) {
    .grid {
      grid-template-columns: 1fr;
    }
  }
  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 12px;
  }
  .networks {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    gap: 8px;
    margin-top: 12px;
  }
  .network {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--surface-2);
    cursor: pointer;
    text-align: left;
  }
  .network:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
</style>
