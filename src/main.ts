import { invoke } from "@tauri-apps/api/core";
import "./styles.css";

type Page = "protection" | "applications" | "websites" | "settings";

interface AppRule {
  id: string;
  platform_app_id: string;
  display_name: string;
  visibility_percent: number;
  enabled: boolean;
}

interface SiteRule {
  id: string;
  hostname: string;
  include_subdomains: boolean;
  visibility_percent: number;
  enabled: boolean;
}

interface AppConfig {
  config_version: number;
  enabled: boolean;
  launch_at_login: boolean;
  emergency_shortcut: string;
  hardware_brightness_enabled: boolean;
  privacy_brightness_percent: number;
  maximum_privacy: boolean;
  app_rules: AppRule[];
  site_rules: SiteRule[];
}

interface BrightnessStatus {
  supported: boolean;
  displays: Array<{
    id: string;
    name: string;
    brightness_percent: number;
    built_in: boolean;
  }>;
  message: string;
}

interface ForegroundApplication {
  platform_app_id: string;
  display_name: string;
}

interface ProtectionStatus {
  foreground_supported: boolean;
  foreground_app: ForegroundApplication | null;
  matched_rule_id: string | null;
  matched_visibility_percent: number | null;
  hardware_active: boolean;
  overlay_active: boolean;
  message: string;
}

const defaultConfig: AppConfig = {
  config_version: 2,
  enabled: true,
  launch_at_login: false,
  emergency_shortcut: "CommandOrControl+Shift+0",
  hardware_brightness_enabled: false,
  privacy_brightness_percent: 35,
  maximum_privacy: false,
  app_rules: [],
  site_rules: [],
};

const state = {
  page: "protection" as Page,
  config: structuredClone(defaultConfig),
  nativeAvailable: true,
  savedMessage: "",
  addKind: null as "app" | "site" | null,
  editingId: null as string | null,
  onboardingStep: localStorage.getItem("privacy-aperture-onboarded") ? 0 : 1,
  previewVisibility: 34,
  brightness: { supported: false, displays: [], message: "Checking hardware brightness support…" } as BrightnessStatus,
  hardwareActive: false,
  protection: {
    foreground_supported: false,
    foreground_app: null,
    matched_rule_id: null,
    matched_visibility_percent: null,
    hardware_active: false,
    overlay_active: false,
    message: "Starting foreground protection…",
  } as ProtectionStatus,
  runningApplications: [] as ForegroundApplication[],
  overlayPreviewActive: false,
};

const app = document.querySelector<HTMLDivElement>("#app");
if (!app) throw new Error("App mount point missing");
const mount = app;

const icons: Record<Page, string> = {
  protection: '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3 5 6v5c0 4.6 2.9 8.1 7 10 4.1-1.9 7-5.4 7-10V6l-7-3Z"/><path d="m9.5 12 1.6 1.6 3.5-3.8"/></svg>',
  applications: '<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="4" y="4" width="16" height="16" rx="3"/><path d="M8 9h8M8 13h5"/></svg>',
  websites: '<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="8"/><path d="M4.5 10h15M4.5 14h15M12 4c2 2.2 3 4.9 3 8s-1 5.8-3 8c-2-2.2-3-4.9-3-8s1-5.8 3-8Z"/></svg>',
  settings: '<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="3"/><path d="M19 13.5v-3l-2-.7-.6-1.4.9-2-2.1-2.1-2 .9-1.4-.6-.7-2h-3l-.7 2-1.4.6-2-.9-2.1 2.1.9 2-.6 1.4-2 .7v3l2 .7.6 1.4-.9 2 2.1 2.1 2-.9 1.4.6.7 2h3l.7-2 1.4-.6 2 .9 2.1-2.1-.9-2 .6-1.4 2-.7Z"/></svg>',
};

const escapeHtml = (value: string): string =>
  value.replace(/[&<>'"]/g, (character) => ({
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    "'": "&#39;",
    '"': "&quot;",
  })[character] ?? character);

function navItem(page: Page, label: string): string {
  const active = state.page === page;
  return `<button class="nav-item${active ? " active" : ""}" data-page="${page}" ${active ? 'aria-current="page"' : ""}>
    ${icons[page]}<span>${label}</span>
  </button>`;
}

function shell(content: string): string {
  const adapterReady = state.protection.foreground_supported;
  return `<div class="app-shell">
    <aside class="sidebar" aria-label="Main navigation">
      <div class="brand" aria-label="Privacy Aperture">
        <span class="brand-mark" aria-hidden="true"><i></i></span>
        <span class="brand-copy"><b>Privacy</b><small>Aperture</small></span>
      </div>
      <nav>
        ${navItem("protection", "Protection")}
        ${navItem("applications", "Applications")}
        ${navItem("websites", "Websites")}
        ${navItem("settings", "Settings")}
      </nav>
      <div class="rail-status">
        <span class="status-dot ${adapterReady ? "" : "warning"}" aria-hidden="true"></span>
        <span><b>${adapterReady ? "Foreground active" : "Setup mode"}</b><small>${adapterReady ? "App-window protection ready" : "Platform adapter pending"}</small></span>
      </div>
    </aside>
    <main class="main-panel">${content}</main>
    <div class="toast${state.savedMessage ? " show" : ""}" role="status" aria-live="polite">${escapeHtml(state.savedMessage)}</div>
    ${state.onboardingStep ? onboarding() : ""}
  </div>`;
}

function pageHeader(eyebrow: string, title: string, description: string, action = ""): string {
  return `<header class="page-header">
    <div><p class="eyebrow">${eyebrow}</p><h1>${title}</h1><p>${description}</p></div>
    ${action}
  </header>`;
}

function protectionPage(): string {
  const enabled = state.config.enabled;
  const foreground = state.protection.foreground_app;
  const matched = state.protection.matched_rule_id !== null;
  const visibility = state.protection.matched_visibility_percent ?? 100;
  const matchedRuleLabel = state.config.app_rules.find((rule) => rule.id === state.protection.matched_rule_id)?.display_name
    ?? state.config.site_rules.find((rule) => rule.id === state.protection.matched_rule_id)?.hostname
    ?? state.protection.matched_rule_id
    ?? "rule";
  return `${pageHeader(
    "Live protection",
    enabled ? matched ? "Sensitive context protected" : "Protection is watching" : "Protection is paused",
    enabled
      ? state.protection.message
      : "No rule can dim your displays while protection is paused.",
    `<button class="power-button ${enabled ? "enabled" : ""}" id="toggle-protection" aria-pressed="${enabled}">
      <span class="power-icon" aria-hidden="true"></span>${enabled ? "Pause" : "Enable"}
    </button>`,
  )}
  <section class="protection-grid">
    <article class="aperture-card">
      <div class="card-label"><span>Current display</span><span class="mono">LIVE</span></div>
      <div class="screen-preview ${matched ? "protected-preview" : "idle"}" aria-label="Display preview: ${matched ? `${visibility}% visibility` : "no protected context active"}">
        <div class="mock-window">
          <div class="mock-top"><i></i><i></i><i></i></div>
          <div class="mock-layout"><span></span><div><b></b><b></b><b></b></div></div>
        </div>
        <div class="aperture-shutter" style="opacity:${matched ? (100 - visibility) / 100 : 0}"><i></i></div>
        <span class="screen-state">${matched ? `${visibility}% visible` : "Clear"}</span>
      </div>
      <div class="context-row">
        <span class="context-icon" aria-hidden="true">${matched ? "✓" : "—"}</span>
        <div><small>Foreground context</small><strong>${foreground ? escapeHtml(foreground.display_name) : state.protection.foreground_supported ? "No identified application" : "Adapter unavailable"}</strong>${foreground ? `<code>${escapeHtml(foreground.platform_app_id)}</code>` : ""}</div>
        <span class="pill ${matched ? "" : "neutral"}">${matched ? escapeHtml("Matched: " + matchedRuleLabel) : "No match"}</span>
      </div>
    </article>
    <div class="status-stack">
      <article class="status-card">
        <div class="status-heading"><span class="status-symbol protected" aria-hidden="true">✓</span><div><small>Rules engine</small><strong>${state.protection.overlay_active ? state.protection.hardware_active ? "Overlay + hardware active" : "Overlay active" : state.protection.hardware_active ? "Hardware dim active" : "Ready"}</strong></div></div>
        <p>${state.config.app_rules.length + state.config.site_rules.length} local rule${state.config.app_rules.length + state.config.site_rules.length === 1 ? "" : "s"} configured</p>
      </article>
      <article class="status-card">
        <div class="status-heading"><span class="status-symbol warning" aria-hidden="true">↗</span><div><small>Browser extension</small><strong>Not connected</strong></div></div>
        <p>Website context stays unavailable until native host is installed.</p>
      </article>
      <article class="status-card compact">
        <div><small>Emergency shortcut</small><strong class="key-combo">${escapeHtml(state.config.emergency_shortcut)}</strong></div>
        <div class="compact-actions"><button class="text-button" id="preview-overlays">${state.overlayPreviewActive ? "Cancel preview" : "Preview current app"}</button><button class="text-button danger" id="remove-dim">Remove dim now</button></div>
      </article>
    </div>
  </section>
  <section class="privacy-note">
    <span aria-hidden="true">◌</span><div><strong>Nothing observed is stored</strong><p>Only current foreground context is evaluated. No activity history, page titles, full URLs, or content.</p></div>
  </section>`;
}

function ruleToggle(kind: "app" | "site", id: string, enabled: boolean, label: string): string {
  return `<label class="switch"><input type="checkbox" data-rule-toggle="${kind}" data-id="${escapeHtml(id)}" ${enabled ? "checked" : ""} aria-label="${enabled ? "Disable" : "Enable"} ${escapeHtml(label)}"><span></span></label>`;
}

function applicationsPage(): string {
  const rules = state.config.app_rules;
  return `${pageHeader(
    "Rule library",
    "Applications",
    "Dim only protected application windows while they are foreground.",
    '<button class="primary-button" data-add="app"><span aria-hidden="true">＋</span>Add application</button>',
  )}
  ${state.addKind === "app" ? appForm() : ""}
  <section class="rule-panel" aria-label="Application rules">
    <div class="rule-head"><span>${rules.length} application${rules.length === 1 ? "" : "s"}</span><span>Visibility</span><span>Status</span><span></span></div>
    ${rules.length ? rules.map(appRuleRow).join("") : emptyState("app")}
  </section>
  <p class="footnote"><span aria-hidden="true">ⓘ</span> Stable platform identifiers are stored locally. macOS running applications are read only while this screen is open.</p>`;
}

function appRuleRow(rule: AppRule): string {
  return `<article class="rule-row">
    <span class="app-avatar" aria-hidden="true">${escapeHtml(rule.display_name.slice(0, 1).toUpperCase())}</span>
    <div class="rule-name"><strong>${escapeHtml(rule.display_name)}</strong><code>${escapeHtml(rule.platform_app_id)}</code></div>
    <span class="visibility-value">${rule.visibility_percent}<small>%</small></span>
    ${ruleToggle("app", rule.id, rule.enabled, rule.display_name)}
    <div class="row-actions"><button data-edit="app" data-id="${escapeHtml(rule.id)}" aria-label="Edit ${escapeHtml(rule.display_name)}">Edit</button><button data-delete="app" data-id="${escapeHtml(rule.id)}" aria-label="Delete ${escapeHtml(rule.display_name)}">Delete</button></div>
  </article>`;
}

function websitesPage(): string {
  const rules = state.config.site_rules;
  return `${pageHeader(
    "Rule library",
    "Websites",
    "Hostnames arrive from browser extension. Page content never leaves browser.",
    '<button class="primary-button" data-add="site"><span aria-hidden="true">＋</span>Add hostname</button>',
  )}
  <div class="extension-banner"><span class="extension-icon" aria-hidden="true">↗</span><div><strong>Extension not connected</strong><p>Install native host and Chromium extension to protect current site from popup.</p></div><span class="pill warning">Offline</span></div>
  ${state.addKind === "site" ? siteForm() : ""}
  <section class="rule-panel" aria-label="Website rules">
    <div class="rule-head"><span>${rules.length} hostname${rules.length === 1 ? "" : "s"}</span><span>Visibility</span><span>Status</span><span></span></div>
    ${rules.length ? rules.map(siteRuleRow).join("") : emptyState("site")}
  </section>`;
}

function siteRuleRow(rule: SiteRule): string {
  return `<article class="rule-row">
    <span class="app-avatar site" aria-hidden="true">●</span>
    <div class="rule-name"><strong class="mono">${escapeHtml(rule.hostname)}</strong><small>${rule.include_subdomains ? "Includes subdomains" : "Exact hostname only"}</small></div>
    <span class="visibility-value">${rule.visibility_percent}<small>%</small></span>
    ${ruleToggle("site", rule.id, rule.enabled, rule.hostname)}
    <div class="row-actions"><button data-edit="site" data-id="${escapeHtml(rule.id)}" aria-label="Edit ${escapeHtml(rule.hostname)}">Edit</button><button data-delete="site" data-id="${escapeHtml(rule.id)}" aria-label="Delete ${escapeHtml(rule.hostname)}">Delete</button></div>
  </article>`;
}

function emptyState(kind: "app" | "site"): string {
  const isApp = kind === "app";
  return `<div class="empty-state"><span class="empty-aperture" aria-hidden="true"><i></i></span><strong>No ${isApp ? "applications" : "websites"} protected yet</strong><p>${isApp ? "Add a stable application identifier to create your first rule." : "Add a hostname here, or use extension popup after connection."}</p><button class="secondary-button" data-add="${kind}">Add ${isApp ? "application" : "hostname"}</button></div>`;
}

function currentAppRule(): AppRule | undefined {
  return state.config.app_rules.find((rule) => rule.id === state.editingId);
}

function currentSiteRule(): SiteRule | undefined {
  return state.config.site_rules.find((rule) => rule.id === state.editingId);
}

function appForm(): string {
  const rule = currentAppRule();
  return `<form class="rule-form" id="app-form">
    <div class="form-title"><div><p class="eyebrow">${rule ? "Edit rule" : "New rule"}</p><h2>${rule ? "Update application" : "Add application"}</h2></div><button type="button" class="icon-button" data-cancel aria-label="Close form">×</button></div>
    ${!rule && state.runningApplications.length ? `<label><span>Running application</span><select id="running-application"><option value="">Choose current app…</option>${state.runningApplications.map((app) => `<option value="${escapeHtml(app.platform_app_id)}">${escapeHtml(app.display_name)} — ${escapeHtml(app.platform_app_id)}</option>`).join("")}</select></label>` : ""}
    <div class="field-grid">
      <label><span>Application name</span><input name="display_name" maxlength="160" required value="${escapeHtml(rule?.display_name ?? "")}" placeholder="Finance workspace"></label>
      <label><span>Platform identifier</span><input class="mono" name="platform_app_id" maxlength="512" required value="${escapeHtml(rule?.platform_app_id ?? "")}" placeholder="com.company.application"><small>Bundle ID on macOS; executable identity on Windows/Linux.</small></label>
    </div>
    ${visibilityField(rule?.visibility_percent ?? 35)}
    <div class="form-error" role="alert"></div><div class="form-actions"><button type="button" class="secondary-button" data-cancel>Cancel</button><button class="primary-button" type="submit">${rule ? "Save changes" : "Add application"}</button></div>
  </form>`;
}

function siteForm(): string {
  const rule = currentSiteRule();
  return `<form class="rule-form" id="site-form">
    <div class="form-title"><div><p class="eyebrow">${rule ? "Edit rule" : "New rule"}</p><h2>${rule ? "Update hostname" : "Add hostname"}</h2></div><button type="button" class="icon-button" data-cancel aria-label="Close form">×</button></div>
    <label><span>Hostname only</span><input class="mono" name="hostname" maxlength="253" required value="${escapeHtml(rule?.hostname ?? "")}" placeholder="web.example.com"><small>No protocol, path, query, or page title is accepted.</small></label>
    <label class="check-row"><input type="checkbox" name="include_subdomains" ${rule?.include_subdomains ? "checked" : ""}><span><strong>Include subdomains</strong><small>Also match names such as secure.web.example.com</small></span></label>
    ${visibilityField(rule?.visibility_percent ?? 35)}
    <div class="form-error" role="alert"></div><div class="form-actions"><button type="button" class="secondary-button" data-cancel>Cancel</button><button class="primary-button" type="submit">${rule ? "Save changes" : "Add hostname"}</button></div>
  </form>`;
}

function visibilityField(value: number): string {
  return `<fieldset class="visibility-field"><legend>Protected visibility</legend><div class="range-wrap"><input type="range" name="visibility_range" min="10" max="100" value="${value}" aria-label="Protected visibility percentage"><div class="number-suffix"><input class="mono" type="number" name="visibility_percent" min="10" max="100" value="${value}" aria-label="Protected visibility percentage"><span>%</span></div></div><small>Lower visibility means darker overlay. 30% visibility uses about 70% black opacity.</small></fieldset>`;
}

function settingsPage(): string {
  const brightness = state.brightness;
  const brightnessControlsDisabled = brightness.supported ? "" : "disabled";
  const displays = brightness.displays.length
    ? brightness.displays.map((display) => `<div class="detected-display"><span><strong>${escapeHtml(display.name)}</strong><small>${display.built_in ? "Built-in panel" : "External / DDC panel"}</small></span><code>${display.brightness_percent}%</code></div>`).join("")
    : `<p class="hardware-message">${escapeHtml(brightness.message)}</p>`;
  return `${pageHeader("Preferences", "Settings", "System behavior and recovery controls. Theme follows your operating system.")}
  <section class="settings-section hardware-section">
    <div class="section-heading"><div><p class="eyebrow">Display hardware</p><h2>Global hardware brightness</h2></div><span class="pill ${brightness.supported ? "warning" : "neutral"}">${brightness.supported ? "Panel-wide" : "Unsupported"}</span></div>
    <p class="section-copy">Same physical level as macOS F1/F2 display keys. Hardware cannot target one app: enabling automatic hardware brightness dims entire display. Leave it off for window-only privacy.</p>
    <div class="hardware-displays">${displays}</div>
    <div class="setting-row inset"><div><strong>Also dim entire display for protected rules</strong><p>Optional global layer. Window overlay remains app-only.</p></div><label class="switch"><input id="hardware-enabled" type="checkbox" ${state.config.hardware_brightness_enabled ? "checked" : ""} ${brightnessControlsDisabled} aria-label="Also dim entire display using hardware brightness"><span></span></label></div>
    <fieldset class="hardware-level" ${brightnessControlsDisabled}><legend>Privacy brightness</legend><div class="range-wrap"><input id="hardware-range" type="range" min="10" max="100" value="${state.config.privacy_brightness_percent}" aria-label="Hardware privacy brightness percentage"><div class="number-suffix"><input class="mono" id="hardware-number" type="number" min="10" max="100" value="${state.config.privacy_brightness_percent}" aria-label="Hardware privacy brightness percentage"><span>%</span></div></div></fieldset>
    <div class="hardware-actions"><button class="secondary-button" id="preview-hardware" ${brightnessControlsDisabled}>Test for 3 seconds</button><button class="primary-button" id="apply-hardware" ${brightnessControlsDisabled}>${state.hardwareActive ? "Update brightness" : "Apply now"}</button>${state.hardwareActive ? '<button class="text-button danger" id="restore-hardware">Restore original</button>' : ""}</div>
  </section>
  <section class="settings-section privacy-display-section">
    <div class="section-heading"><div><p class="eyebrow">Privacy display</p><h2>Automatic conditions</h2></div><span class="pill neutral">Laptop mode</span></div>
    <p class="section-copy">Inspired by Galaxy S26 Ultra controls. S26 viewing-angle restriction uses Flex Magic Pixel hardware; normal laptop panels cannot reproduce it.</p>
    <div class="condition-list">
      <div><span class="status-symbol protected" aria-hidden="true">✓</span><span><strong>Selected applications</strong><small>${state.config.app_rules.length} configured rule${state.config.app_rules.length === 1 ? "" : "s"}</small></span><span class="pill">Supported</span></div>
      <div><span class="status-symbol" aria-hidden="true">—</span><span><strong>PIN, pattern, or password fields</strong><small>Disabled: would require invasive cross-app content inspection.</small></span><span class="pill neutral">Not available</span></div>
      <div><span class="status-symbol" aria-hidden="true">—</span><span><strong>Notification pop-ups only</strong><small>Desktop platforms do not expose safe cross-app partial-screen control.</small></span><span class="pill neutral">Not available</span></div>
    </div>
    <div class="setting-row inset"><div><strong>Maximum privacy</strong><p>Use 10% app-window visibility and, only when global hardware mode is enabled, 10% entire-display brightness. Does not narrow panel viewing angle.</p></div><label class="switch"><input id="maximum-privacy" type="checkbox" ${state.config.maximum_privacy ? "checked" : ""} aria-label="Maximum privacy"><span></span></label></div>
  </section>
  <section class="settings-list">
    <div class="setting-row"><div><strong>Launch at login</strong><p>Start protection after you sign in. Registration activates with platform adapter.</p></div><label class="switch"><input id="launch-at-login" type="checkbox" ${state.config.launch_at_login ? "checked" : ""} aria-label="Launch at login"><span></span></label></div>
    <div class="setting-row shortcut-setting"><div><strong>Emergency shortcut</strong><p>Immediately removes all dim overlays and pauses protection.</p></div><label><span class="sr-only">Emergency shortcut</span><input class="mono" id="shortcut" maxlength="80" value="${escapeHtml(state.config.emergency_shortcut)}"></label></div>
    <div class="setting-row"><div><strong>Theme</strong><p>Uses system light or dark appearance.</p></div><span class="setting-value">System</span></div>
  </section>
  <section class="settings-section"><h2>Connection</h2><div class="connection-card"><span class="status-symbol warning" aria-hidden="true">↗</span><div><strong>Chromium extension disconnected</strong><p>Native host registration ships with browser-integration milestone.</p></div><span class="pill warning">Offline</span></div></section>
  <section class="settings-section"><h2>Platform support</h2><div class="support-grid"><div><span>macOS 13+</span><strong>App-window overlay; optional global hardware dim</strong></div><div><span>Windows 10/11</span><strong>Brightness code; hardware QA pending</strong></div><div><span>Linux X11</span><strong>Backlight code; hardware QA pending</strong></div><div><span>Linux Wayland</span><strong>Brightness only</strong></div></div></section>
  <section class="settings-section privacy-copy"><h2>Privacy</h2><p>Privacy Aperture stores rules and preferences on this device. It has no account, analytics, telemetry, or network service. It never stores activity history, page content, titles, or full URLs.</p><p>It reduces casual shoulder-surfing; it cannot stop cameras, screenshots, or close viewing.</p></section>`;
}

function onboarding(): string {
  const step = state.onboardingStep;
  const copy = [
    ["Sensitive apps dim when you open them.", "Privacy Aperture watches current foreground identity locally, matches your rules, then covers only that app's visible windows."],
    ["Choose how much stays visible.", "Move control to close aperture. Lower visibility creates darker protection."],
    ["Add your first application.", "Choose a running macOS app. Privacy Aperture stores only its stable bundle identifier and rule."],
    ["Protect browser hostnames.", "Chromium extension sends active hostname only. It never reads page content, titles, paths, or history."],
    ["Keep recovery close.", "Emergency shortcut removes all overlays immediately. You can change it later in Settings."],
  ][step - 1];
  const preview = step === 2 ? `<div class="onboarding-preview"><div class="mini-screen"><span>Private workspace</span><i style="opacity:${(100 - state.previewVisibility) / 100}"></i></div>${visibilityField(state.previewVisibility)}</div>` : `<div class="onboarding-art step-${step}"><span class="onboard-aperture"><i></i></span><small>${step === 1 ? "LOCAL ONLY" : step === 3 ? "APPLICATION ID" : step === 4 ? "HOSTNAME ONLY" : escapeHtml(state.config.emergency_shortcut)}</small></div>`;
  return `<div class="onboarding-backdrop"><section class="onboarding" role="dialog" aria-modal="true" aria-labelledby="onboarding-title">
    <div class="onboarding-progress"><span>Setup</span><div>${[1, 2, 3, 4, 5].map((number) => `<i class="${number <= step ? "done" : ""}"></i>`).join("")}</div><span class="mono">${step}/5</span></div>
    ${preview}<div class="onboarding-copy"><p class="eyebrow">${step === 1 ? "Welcome" : `Step ${step}`}</p><h2 id="onboarding-title">${copy?.[0]}</h2><p>${copy?.[1]}</p></div>
    <div class="onboarding-actions">${step > 1 ? '<button class="secondary-button" id="onboarding-back">Back</button>' : '<button class="text-button" id="onboarding-skip">Skip setup</button>'}<button class="primary-button" id="onboarding-next">${step === 5 ? "Finish setup" : "Continue"}</button></div>
  </section></div>`;
}

function render(): void {
  const page = state.page === "protection" ? protectionPage() : state.page === "applications" ? applicationsPage() : state.page === "websites" ? websitesPage() : settingsPage();
  mount.innerHTML = shell(page);
  bindEvents();
}

async function persist(message = "Saved locally"): Promise<void> {
  try {
    await invoke("save_config", { config: state.config });
  } catch {
    state.nativeAvailable = false;
    localStorage.setItem("privacy-aperture-preview-config", JSON.stringify(state.config));
  }
  state.savedMessage = message;
  render();
  clearMessageLater();
}

function clearMessageLater(): void {
  window.setTimeout(() => {
    state.savedMessage = "";
    const toast = document.querySelector(".toast");
    toast?.classList.remove("show");
  }, 1800);
}

function setPage(page: Page): void {
  state.page = page;
  state.addKind = null;
  state.editingId = null;
  render();
}

async function openAddForm(kind: "app" | "site"): Promise<void> {
  state.editingId = null;
  if (kind === "app") {
    try {
      state.runningApplications = await invoke<ForegroundApplication[]>("list_running_applications");
    } catch {
      state.runningApplications = [];
    }
  }
  state.addKind = kind;
  render();
  document.querySelector<HTMLElement>(kind === "app" && state.runningApplications.length ? "#running-application" : ".rule-form input")?.focus();
}

function bindEvents(): void {
  document.querySelectorAll<HTMLButtonElement>("[data-page]").forEach((button) => button.addEventListener("click", () => setPage(button.dataset.page as Page)));
  document.querySelectorAll<HTMLButtonElement>("[data-add]").forEach((button) => button.addEventListener("click", () => void openAddForm(button.dataset.add as "app" | "site")));
  document.querySelectorAll<HTMLButtonElement>("[data-cancel]").forEach((button) => button.addEventListener("click", () => {
    state.addKind = null;
    state.editingId = null;
    render();
  }));
  document.querySelectorAll<HTMLButtonElement>("[data-edit]").forEach((button) => button.addEventListener("click", () => {
    state.addKind = button.dataset.edit as "app" | "site";
    state.editingId = button.dataset.id ?? null;
    render();
    document.querySelector<HTMLElement>(".rule-form input")?.focus();
  }));
  document.querySelectorAll<HTMLButtonElement>("[data-delete]").forEach((button) => button.addEventListener("click", () => {
    const kind = button.dataset.delete;
    const id = button.dataset.id;
    if (kind === "app") state.config.app_rules = state.config.app_rules.filter((rule) => rule.id !== id);
    if (kind === "site") state.config.site_rules = state.config.site_rules.filter((rule) => rule.id !== id);
    void persist("Rule deleted");
  }));
  document.querySelectorAll<HTMLInputElement>("[data-rule-toggle]").forEach((toggle) => toggle.addEventListener("change", () => {
    const rules = toggle.dataset.ruleToggle === "app" ? state.config.app_rules : state.config.site_rules;
    const rule = rules.find((candidate) => candidate.id === toggle.dataset.id);
    if (rule) rule.enabled = toggle.checked;
    void persist(toggle.checked ? "Rule enabled" : "Rule disabled");
  }));
  bindRangePairs();
  document.querySelector<HTMLFormElement>("#app-form")?.addEventListener("submit", saveAppRule);
  document.querySelector<HTMLFormElement>("#site-form")?.addEventListener("submit", saveSiteRule);
  document.querySelector<HTMLSelectElement>("#running-application")?.addEventListener("change", (event) => {
    const selected = state.runningApplications.find((app) => app.platform_app_id === (event.currentTarget as HTMLSelectElement).value);
    const form = document.querySelector<HTMLFormElement>("#app-form");
    if (!selected || !form) return;
    const name = form.elements.namedItem("display_name") as HTMLInputElement | null;
    const identifier = form.elements.namedItem("platform_app_id") as HTMLInputElement | null;
    if (name) name.value = selected.display_name;
    if (identifier) identifier.value = selected.platform_app_id;
  });
  document.querySelector<HTMLButtonElement>("#toggle-protection")?.addEventListener("click", () => {
    state.config.enabled = !state.config.enabled;
    void persist(state.config.enabled ? "Protection enabled" : "Protection paused");
  });
  document.querySelector<HTMLButtonElement>("#remove-dim")?.addEventListener("click", () => {
    state.config.enabled = false;
    state.hardwareActive = false;
    state.overlayPreviewActive = false;
    void invoke("remove_all_dimming").catch(() => undefined);
    void persist("All dimming removed; protection paused");
  });
  document.querySelector<HTMLButtonElement>("#preview-overlays")?.addEventListener("click", () => void toggleOverlayPreview());
  document.querySelector<HTMLInputElement>("#hardware-enabled")?.addEventListener("change", (event) => {
    state.config.hardware_brightness_enabled = (event.currentTarget as HTMLInputElement).checked;
    void persist("Hardware brightness preference saved");
  });
  document.querySelector<HTMLInputElement>("#maximum-privacy")?.addEventListener("change", (event) => {
    state.config.maximum_privacy = (event.currentTarget as HTMLInputElement).checked;
    void persist("Maximum privacy preference saved");
  });
  bindHardwareLevel();
  document.querySelector<HTMLButtonElement>("#preview-hardware")?.addEventListener("click", () => void previewHardware());
  document.querySelector<HTMLButtonElement>("#apply-hardware")?.addEventListener("click", () => void applyHardware());
  document.querySelector<HTMLButtonElement>("#restore-hardware")?.addEventListener("click", () => void restoreHardware());
  document.querySelector<HTMLInputElement>("#launch-at-login")?.addEventListener("change", (event) => {
    state.config.launch_at_login = (event.currentTarget as HTMLInputElement).checked;
    void persist("Launch preference saved");
  });
  document.querySelector<HTMLInputElement>("#shortcut")?.addEventListener("change", (event) => {
    const value = (event.currentTarget as HTMLInputElement).value.trim();
    if (value) state.config.emergency_shortcut = value;
    void persist("Shortcut saved");
  });
  document.querySelector<HTMLButtonElement>("#onboarding-next")?.addEventListener("click", () => {
    if (state.onboardingStep === 5) finishOnboarding();
    else {
      state.onboardingStep += 1;
      render();
    }
  });
  document.querySelector<HTMLButtonElement>("#onboarding-back")?.addEventListener("click", () => {
    state.onboardingStep -= 1;
    render();
  });
  document.querySelector<HTMLButtonElement>("#onboarding-skip")?.addEventListener("click", finishOnboarding);
}

function bindHardwareLevel(): void {
  const range = document.querySelector<HTMLInputElement>("#hardware-range");
  const number = document.querySelector<HTMLInputElement>("#hardware-number");
  if (!range || !number) return;
  const sync = (source: HTMLInputElement, target: HTMLInputElement): void => {
    const value = Math.min(100, Math.max(10, Number(source.value) || 10));
    source.value = String(value);
    target.value = String(value);
    state.config.privacy_brightness_percent = value;
  };
  range.addEventListener("input", () => sync(range, number));
  number.addEventListener("input", () => sync(number, range));
  range.addEventListener("change", () => void persist("Privacy brightness saved"));
  number.addEventListener("change", () => void persist("Privacy brightness saved"));
}

async function previewHardware(): Promise<void> {
  state.hardwareActive = false;
  try {
    state.brightness = await invoke<BrightnessStatus>("preview_hardware_brightness", { percent: state.config.privacy_brightness_percent });
    state.savedMessage = "Brightness preview active for 3 seconds";
  } catch (error) {
    state.savedMessage = String(error);
  }
  render();
  clearMessageLater();
  window.setTimeout(() => void refreshBrightness(), 3200);
}

async function applyHardware(): Promise<void> {
  try {
    state.brightness = await invoke<BrightnessStatus>("apply_hardware_brightness", { percent: state.config.privacy_brightness_percent });
    state.hardwareActive = true;
    state.savedMessage = "Hardware brightness applied";
  } catch (error) {
    state.savedMessage = String(error);
  }
  render();
  clearMessageLater();
}

async function restoreHardware(): Promise<void> {
  try {
    await invoke("cancel_hardware_brightness_preview");
    state.hardwareActive = false;
    state.savedMessage = "Original brightness restored";
    await refreshBrightness();
    clearMessageLater();
  } catch (error) {
    state.savedMessage = String(error);
    render();
  }
}

async function refreshBrightness(renderAfter = true): Promise<void> {
  try {
    state.brightness = await invoke<BrightnessStatus>("get_hardware_brightness");
  } catch {
    state.brightness = { supported: false, displays: [], message: "Hardware brightness requires desktop app" };
  }
  if (renderAfter) render();
}

async function refreshProtection(renderAfter = true): Promise<void> {
  try {
    const next = await invoke<ProtectionStatus>("get_protection_status");
    const changed = JSON.stringify(next) !== JSON.stringify(state.protection);
    state.protection = next;
    if (renderAfter && changed && state.page === "protection" && !state.onboardingStep) render();
  } catch {
    state.protection = {
      foreground_supported: false,
      foreground_app: null,
      matched_rule_id: null,
      matched_visibility_percent: null,
      hardware_active: false,
      overlay_active: false,
      message: "Foreground protection requires desktop app",
    };
  }
}

async function toggleOverlayPreview(): Promise<void> {
  try {
    if (state.overlayPreviewActive) {
      await invoke("cancel_privacy_overlay_preview");
      state.overlayPreviewActive = false;
      state.savedMessage = "Overlay preview cancelled";
    } else {
      await invoke("preview_privacy_overlay", { visibilityPercent: 30 });
      state.overlayPreviewActive = true;
      state.savedMessage = "Current app preview active for 3 seconds";
      window.setTimeout(() => {
        state.overlayPreviewActive = false;
        if (state.page === "protection") render();
      }, 3100);
    }
  } catch (error) {
    state.overlayPreviewActive = false;
    state.savedMessage = String(error);
  }
  render();
  clearMessageLater();
}

function bindRangePairs(): void {
  document.querySelectorAll<HTMLElement>(".visibility-field").forEach((field) => {
    const range = field.querySelector<HTMLInputElement>('[name="visibility_range"]');
    const number = field.querySelector<HTMLInputElement>('[name="visibility_percent"]');
    if (!range || !number) return;
    const sync = (source: HTMLInputElement, target: HTMLInputElement): void => {
      target.value = source.value;
      if (state.onboardingStep === 2) {
        state.previewVisibility = Number(source.value);
        const shutter = document.querySelector<HTMLElement>(".mini-screen i");
        if (shutter) shutter.style.opacity = String((100 - state.previewVisibility) / 100);
      }
    };
    range.addEventListener("input", () => sync(range, number));
    number.addEventListener("input", () => sync(number, range));
  });
}

function formError(form: HTMLFormElement, message: string): void {
  const target = form.querySelector<HTMLElement>(".form-error");
  if (target) target.textContent = message;
}

function saveAppRule(event: SubmitEvent): void {
  event.preventDefault();
  const form = event.currentTarget as HTMLFormElement;
  const data = new FormData(form);
  const displayName = String(data.get("display_name") ?? "").trim();
  const platformId = String(data.get("platform_app_id") ?? "").trim();
  const visibility = Number(data.get("visibility_percent"));
  if (!displayName || !platformId || !Number.isInteger(visibility) || visibility < 10 || visibility > 100) {
    formError(form, "Enter application name, platform identifier, and visibility from 10 to 100.");
    return;
  }
  const existing = currentAppRule();
  const rule: AppRule = { id: existing?.id ?? crypto.randomUUID(), display_name: displayName, platform_app_id: platformId, visibility_percent: visibility, enabled: existing?.enabled ?? true };
  if (existing) Object.assign(existing, rule);
  else state.config.app_rules.push(rule);
  state.addKind = null;
  state.editingId = null;
  void persist(existing ? "Application rule updated" : "Application rule added");
}

function validHostname(hostname: string): boolean {
  return hostname.length > 0 && hostname.length <= 253 && !hostname.startsWith(".") && !hostname.endsWith(".") && hostname.split(".").every((label) => label.length > 0 && label.length <= 63 && !label.startsWith("-") && !label.endsWith("-") && /^[a-z0-9-]+$/.test(label));
}

function saveSiteRule(event: SubmitEvent): void {
  event.preventDefault();
  const form = event.currentTarget as HTMLFormElement;
  const data = new FormData(form);
  const hostname = String(data.get("hostname") ?? "").trim().toLowerCase();
  const visibility = Number(data.get("visibility_percent"));
  if (!validHostname(hostname)) {
    formError(form, "Enter a hostname only, such as web.example.com. Do not include protocol or path.");
    return;
  }
  if (!Number.isInteger(visibility) || visibility < 10 || visibility > 100) {
    formError(form, "Visibility must be from 10 to 100.");
    return;
  }
  const existing = currentSiteRule();
  const duplicate = state.config.site_rules.some((rule) => rule.hostname === hostname && rule.id !== existing?.id);
  if (duplicate) {
    formError(form, "A rule for this hostname already exists.");
    return;
  }
  const rule: SiteRule = { id: existing?.id ?? crypto.randomUUID(), hostname, include_subdomains: data.get("include_subdomains") === "on", visibility_percent: visibility, enabled: existing?.enabled ?? true };
  if (existing) Object.assign(existing, rule);
  else state.config.site_rules.push(rule);
  state.addKind = null;
  state.editingId = null;
  void persist(existing ? "Website rule updated" : "Website rule added");
}

function finishOnboarding(): void {
  localStorage.setItem("privacy-aperture-onboarded", "1");
  state.onboardingStep = 0;
  render();
}

async function boot(): Promise<void> {
  try {
    const loaded = await invoke<AppConfig>("load_config");
    state.config = { ...structuredClone(defaultConfig), ...loaded };
  } catch {
    state.nativeAvailable = false;
    const preview = localStorage.getItem("privacy-aperture-preview-config");
    if (preview) {
      try {
        state.config = { ...structuredClone(defaultConfig), ...(JSON.parse(preview) as Partial<AppConfig>) };
      } catch {
        state.config = structuredClone(defaultConfig);
      }
    }
  }
  await refreshBrightness(false);
  await refreshProtection(false);
  render();
  window.setInterval(() => void refreshProtection(), 500);
}

void boot();
