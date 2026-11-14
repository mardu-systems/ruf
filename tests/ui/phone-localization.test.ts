import { afterEach, expect, test } from "bun:test";
import { compile } from "svelte/compiler";
import { render } from "svelte/server";
import { get } from "svelte/store";
import { locale, message } from "../../apps/desktop/src/lib/i18n";
import type {
  AppLocale,
  CallSnapshot,
  Contact,
} from "../../apps/desktop/src/lib/generated/AppInfo";

Bun.plugin({
  name: "phone-localization-svelte",
  setup(build) {
    build.onLoad({ filter: /\.svelte$/ }, async ({ path }) => ({
      contents: compile(await Bun.file(path).text(), {
        filename: path,
        generate: "server",
      }).js.code,
      loader: "js",
    }));
  },
});
const { default: Dialpad } =
  await import("../../apps/desktop/src/lib/components/Dialpad.svelte");
const { default: CallPanel } =
  await import("../../apps/desktop/src/lib/components/CallPanel.svelte");
const { default: ContactRow } =
  await import("../../apps/desktop/src/lib/components/ContactRow.svelte");
const { default: GettingStarted } =
  await import("../../apps/desktop/src/lib/components/GettingStarted.svelte");
const initialLocale = get(locale);
afterEach(() => locale.set(initialLocale));

test("dialpad renders the same stored hint in the selected language", () => {
  locale.set("de");
  const hint = message("phone.start.ready_hint");
  const props = {
    value: "+4930123456",
    canCall: true,
    busy: false,
    registered: true,
    connectionHint: hint,
    setupLabel: message("phone.start.setup"),
    setupEnabled: true,
    oncall() {},
    onsetup() {},
  };
  const de = render(Dialpad, { props }).body;
  expect(de).toContain('aria-label="Nummer oder Nebenstelle"');
  expect(de).toContain("Wähle eine Nummer oder Nebenstelle.");
  locale.set("en");
  const en = render(Dialpad, { props }).body;
  expect(en).toContain('aria-label="Number or extension"');
  expect(en).toContain("Choose a number or extension.");
  expect(en).toContain("+4930123456");
  expect(en).not.toContain("Wähle eine Nummer");
});

const current: CallSnapshot = {
  id: "call-localization",
  accountId: 1,
  contact: null,
  state: "active",
  destination: "+4930123456",
  caller: null,
  connectedAtMs: 1000,
  muted: false,
  localHeld: true,
  remoteHeld: false,
  holdPending: false,
  transfer: "idle",
  consultation: false,
  events: [],
  eventsDropped: 0,
  lastError: null,
  security: {
    signalingTransport: "tls",
    tlsPeerVerified: true,
    media: "unprotected",
  },
};

test("call controls translate held state, security and controls while preserving caller data", () => {
  const props = {
    call: current,
    callerContact: {
      id: "raw-contact",
      source: "macos" as const,
      name: "Ada <Büro>",
    },
    elapsed: "02:00",
    busy: false,
    compact: false,
    onanswer() {},
    onhangup() {},
    async onmute() {
      return true;
    },
    onhold() {},
    ontransfer() {},
    onconsult() {},
    onswitch() {},
    oncancelconsult() {},
    oncompletetransfer() {},
  };
  locale.set("de");
  const de = render(CallPanel, { props }).body;
  expect(de).toContain("Von dir gehalten");
  expect(de).toContain("TLS geprüft · Audio unverschlüsselt");
  expect(de).toContain("Fortsetzen");
  locale.set("en");
  const en = render(CallPanel, { props }).body;
  expect(en).toContain("On hold by you");
  expect(en).toContain("TLS verified · Audio unencrypted");
  expect(en).toContain("Resume");
  expect(en).toContain('aria-label="Call duration"');
  expect(en).toContain("Ada &lt;Büro>");
  expect(en).toContain("+4930123456");
});

const contact: Contact = {
  id: "localized-contact",
  source: "macos",
  name: "Zoë <Büro>",
  organization: "Original GmbH",
  favorite: true,
  phones: [
    { label: "Privat", number: "+49 30 123456", destination: "+4930123456" },
    { label: "Desk", number: "987", destination: "987" },
  ],
};

test.each([
  ["de", "2 Nummern", "Favorit entfernen", "macOS-Kontakte öffnen"],
  ["en", "2 numbers", "Remove favorite", "Open macOS Contacts"],
] satisfies [AppLocale, string, string, string][])(
  "contact rows localize actions and accessible counts in %s without translating contact fields",
  (language, count, favorite, source) => {
    locale.set(language);
    const body = render(ContactRow, {
      props: {
        contact,
        expanded: true,
        canDial: true,
        favoriteBusy: false,
        onexpand() {},
        onfavorite() {},
        oncall() {},
        onhistory() {},
      },
    }).body;
    expect(body).toContain(count);
    expect(body).toContain(favorite);
    expect(body).toContain(source);
    expect(body).toContain("Zoë &lt;Büro>");
    expect(body).toContain("Original GmbH");
    expect(body).toContain("Privat");
    expect(body).toContain("+49 30 123456");
  },
);

test("getting started renders the browser explanation in both languages", () => {
  const props = {
    runtime: "browser" as const,
    registration: "unconfigured" as const,
    audioUnknown: false,
    onretry() {},
    onsetup() {},
    onexplore() {},
  };
  locale.set("de");
  expect(render(GettingStarted, { props }).body).toContain("Browser-Vorschau");
  locale.set("en");
  expect(render(GettingStarted, { props }).body).toContain("Browser preview");
});

test("getting started distinguishes disabled accounts from missing configuration", () => {
  locale.set("de");
  const body = render(GettingStarted, {
    props: {
      runtime: "desktop",
      registration: "unconfigured",
      hasAccount: true,
      accountEnabled: false,
      audioUnknown: false,
      onsetup() {},
      onexplore() {},
      onretry() {},
    },
  }).body;
  expect(body).toContain("Dein SIP-Konto ist deaktiviert");
  expect(body).toContain("Kontoeinstellungen öffnen");
  expect(body).not.toContain("speichere sie");
});

test("unavailable runtime offers reload without claiming a stale account is ready", () => {
  locale.set("en");
  const body = render(GettingStarted, {
    props: {
      runtime: "unavailable",
      registration: "registered",
      hasAccount: true,
      audioUnknown: false,
      onsetup() {},
      onexplore() {},
      onretry() {},
    },
  }).body;
  expect(body).toContain("Reload status");
  expect(body).not.toContain("For an international number");
  expect(body).not.toContain("Open account settings");
});

test("help during a call returns to the call instead of offering a new number", () => {
  locale.set("de");
  const body = render(GettingStarted, {
    props: {
      runtime: "desktop",
      registration: "registered",
      hasAccount: true,
      callInProgress: true,
      audioUnknown: false,
      onsetup() {},
      onexplore() {},
      onretry() {},
    },
  }).body;
  expect(body).toContain("Zum Gespräch");
  expect(body).not.toContain("Nummer wählen");
});

test("loading status does not offer to configure an account", () => {
  locale.set("en");
  const body = render(GettingStarted, {
    props: {
      runtime: "loading",
      registration: "unconfigured",
      audioUnknown: false,
      onsetup() {},
      onexplore() {},
      onretry() {},
    },
  }).body;
  expect(body).toContain("Loading phone status");
  expect(body).not.toContain("Set up SIP account");
});
