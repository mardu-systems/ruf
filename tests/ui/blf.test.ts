import { expect, test } from "bun:test";
import { compile } from "svelte/compiler";
import { render } from "svelte/server";
import { locale } from "../../apps/desktop/src/lib/i18n";

Bun.plugin({
  name: "blf-svelte",
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
const { default: MonitoredExtensions } =
  await import("../../apps/desktop/src/lib/components/MonitoredExtensions.svelte");

test("BLF list labels every observed state without relying on color", () => {
  const body = render(MonitoredExtensions, {
    props: {
      extensions: [
        { extension: "1001", name: "Reception", state: "idle" },
        { extension: "1002", name: "", state: "ringing" },
        { extension: "1003", name: "Office", state: "busy" },
        { extension: "1004", name: "Other", state: "unknown" },
      ],
      canDial: true,
      oncall() {},
    },
  }).body;
  for (const state of ["Frei", "Klingelt", "Besetzt", "Unbekannt"])
    expect(body).toContain(state);
  expect(body).toContain("Reception");
  expect(body).toContain("Nebenstelle 1002");
  expect(body).not.toContain("disabled");
});

test("BLF calls follow the phone guard even when a target is idle", () => {
  locale.set("en");
  const body = render(MonitoredExtensions, {
    props: {
      extensions: [{ extension: "1001", name: "Reception", state: "idle" }],
      canDial: false,
      oncall() {},
    },
  }).body;
  expect(body).toContain("disabled");
  expect(body).toContain("Idle");
  expect(body).toContain("Call Reception, extension 1001, Idle");
});

test("accounts without monitored targets do not add an empty phone section", () => {
  const body = render(MonitoredExtensions, {
    props: { extensions: [], canDial: false, oncall() {} },
  }).body;
  expect(body).not.toContain("<section");
});
