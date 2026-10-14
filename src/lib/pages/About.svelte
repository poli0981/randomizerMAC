<script lang="ts">
  import { FolderGit2, Tag, Bug, MessagesSquare, Scale, ClipboardCopy } from "@lucide/svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import Logo from "../components/Logo.svelte";
  import { app } from "../stores/app.svelte";
  import { api } from "../api";
  import { t, type MessageKey } from "../i18n/index.svelte";

  const REPO = "https://github.com/poli0981/randomizerMAC";

  const links: { key: MessageKey; url: string; icon: typeof FolderGit2 }[] = [
    { key: "about.repo", url: REPO, icon: FolderGit2 },
    { key: "about.releases", url: `${REPO}/releases`, icon: Tag },
    { key: "about.issues", url: `${REPO}/issues/new?template=bug_report.yml`, icon: Bug },
    { key: "about.discussions", url: `${REPO}/discussions`, icon: MessagesSquare },
    { key: "about.license", url: `${REPO}/blob/master/LICENSE`, icon: Scale },
  ];

  const components = [
    { name: "Tauri", license: "MIT / Apache-2.0", url: "https://tauri.app" },
    { name: "Rust", license: "MIT / Apache-2.0", url: "https://www.rust-lang.org" },
    { name: "Svelte", license: "MIT", url: "https://svelte.dev" },
    { name: "Lucide icons", license: "ISC", url: "https://lucide.dev" },
    {
      name: "windows-rs",
      license: "MIT / Apache-2.0",
      url: "https://github.com/microsoft/windows-rs",
    },
    { name: "zbus", license: "MIT", url: "https://github.com/dbus2/zbus" },
    { name: "serde", license: "MIT / Apache-2.0", url: "https://github.com/serde-rs/serde" },
    { name: "tracing", license: "MIT", url: "https://github.com/tokio-rs/tracing" },
    { name: "clap", license: "MIT / Apache-2.0", url: "https://github.com/clap-rs/clap" },
    { name: "marked", license: "MIT", url: "https://github.com/markedjs/marked" },
    {
      name: "DOMPurify",
      license: "Apache-2.0 / MPL-2.0",
      url: "https://github.com/cure53/DOMPurify",
    },
    { name: "IEEE OUI registry", license: "Public data", url: "https://standards-oui.ieee.org" },
  ];

  const info = $derived(app.info);
  const methods = $derived.by(() => {
    const c = info?.capabilities;
    if (!c) return "—";
    if (c.platform === "windows") return "Windows registry (NetworkAddress)";
    if (c.platform === "mock") return "mock";
    const m = [];
    if (c.networkManager) m.push("NetworkManager");
    if (c.iproute2) m.push(c.pkexec || c.elevated ? "iproute2 (pkexec)" : "iproute2");
    return m.length ? m.join(", ") : t("common.none");
  });

  async function copyDiagnostics() {
    try {
      const text = await api.diagnostics();
      await navigator.clipboard.writeText(text);
      app.toast("success", t("about.diagnosticsCopied"));
    } catch (e) {
      app.toastError(e);
    }
  }
</script>

<PageHeader title={t("about.title")} subtitle={t("about.subtitle")} />

<div class="stack">
  <section class="card hero">
    <Logo size={72} />
    <div>
      <h2 class="name">RandomMac <span class="muted">v{info?.version}</span></h2>
      <p class="muted">{t("about.description")}</p>
      <div class="row links">
        {#each links as l (l.key)}
          {@const Icon = l.icon}
          <button class="btn small" onclick={() => api.openUrl(l.url)}
            ><Icon size={14} aria-hidden="true" />{t(l.key)}</button
          >
        {/each}
      </div>
    </div>
  </section>

  <div class="two">
    <section class="card" aria-labelledby="sys-title">
      <div class="card-head">
        <h2 id="sys-title" class="card-title">{t("about.system")}</h2>
        <button class="btn small" onclick={copyDiagnostics}
          ><ClipboardCopy size={14} aria-hidden="true" />{t("about.diagnostics")}</button
        >
      </div>
      <dl class="facts">
        <dt>{t("about.os")}</dt>
        <dd>{info?.osVersion} ({info?.os})</dd>
        <dt>{t("about.arch")}</dt>
        <dd>{info?.arch}</dd>
        <dt>{t("about.package")}</dt>
        <dd>{info?.packageKind ?? "dev"}</dd>
        <dt>{t("about.privileges")}</dt>
        <dd>
          {info?.mock
            ? t("status.mock")
            : info?.elevated
              ? t("status.admin")
              : info?.os === "windows"
                ? t("status.readOnly")
                : t("status.user")}
        </dd>
        <dt>{t("about.methods")}</dt>
        <dd>{methods}</dd>
        <dt>{t("about.ouiData")}</dt>
        <dd>{t("about.ouiDataValue", { date: info?.ouiSourceDate ?? "" })}</dd>
        <dt>{t("about.dataFolder")}</dt>
        <dd class="mono path">{info?.dataDir}</dd>
        <dt>{t("about.logFolder")}</dt>
        <dd class="mono path">{info?.logDir}</dd>
      </dl>
    </section>

    <section class="card" aria-labelledby="oss-title">
      <h2 id="oss-title" class="card-title">{t("about.thirdParty")}</h2>
      <ul class="oss">
        {#each components as c (c.name)}
          <li>
            <button class="link" onclick={() => api.openUrl(c.url)}>{c.name}</button>
            <span class="subtle">{c.license}</span>
          </li>
        {/each}
      </ul>
    </section>
  </div>

  <section class="card" aria-labelledby="legal-title">
    <h2 id="legal-title" class="card-title">{t("about.legal")}</h2>
    <p class="muted legal">{t("about.legalText")}</p>
    <p class="subtle">© 2026 poli0981 · MIT · {t("about.ai")}</p>
  </section>
</div>

<style>
  .hero {
    display: flex;
    gap: 20px;
    align-items: center;
  }
  .name {
    font-size: 22px;
    margin-bottom: 4px;
  }
  .links {
    margin-top: 12px;
  }
  .two {
    display: grid;
    grid-template-columns: 1.2fr 1fr;
    gap: 16px;
    align-items: start;
  }
  @media (max-width: 1000px) {
    .two {
      grid-template-columns: 1fr;
    }
  }
  .facts {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 7px 16px;
    margin: 0;
  }
  .facts dt {
    color: var(--text-muted);
  }
  .facts dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .path {
    font-size: 12.5px;
  }
  .oss {
    list-style: none;
    margin: 8px 0 0;
    padding: 0;
    columns: 2;
  }
  .oss li {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    padding: 4px 0;
    break-inside: avoid;
  }
  .link {
    background: none;
    border: none;
    padding: 0;
    color: var(--accent);
    cursor: pointer;
    text-align: left;
  }
  .link:hover {
    text-decoration: underline;
  }
  .legal {
    margin: 8px 0;
  }
</style>
