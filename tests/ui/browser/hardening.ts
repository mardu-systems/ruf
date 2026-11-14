import type { Page } from "playwright-core";
import type { AppLocale } from "../../../apps/desktop/src/lib/generated/AppInfo";

// Synthetic IPC and local module failures only; no native calls or personal data.
export default async function hardening(
  page: Page,
  origin: string,
  capture = false,
) {
  page.setDefaultTimeout(8000);
  await page.goto(origin);
  const en = await page.evaluate(() => window.__rufTestLanguage === "en");
  const failures: string[] = [];
  const check = async (name: string, run: () => Promise<void>) => {
    try {
      await run();
    } catch (error) {
      failures.push(
        `${name}: ${error instanceof Error ? error.message : String(error)}`,
      );
    }
  };
  const assert = (condition: unknown, description: string) => {
    if (!condition) throw Error(description);
  };
  const number = () =>
    page.getByRole("textbox", {
      name: en ? "Number or extension" : "Nummer oder Nebenstelle",
      exact: true,
    });
  await page.setViewportSize({ width: 320, height: 440 });

  for (const settings of [false, true])
    await check(
      `${settings ? "settings" : "phone"} module failure has a retry`,
      async () => {
        const pattern = settings
          ? "**/src/Settings.svelte*"
          : "**/src/App.svelte*";
        await page.route(pattern, (route) => route.abort("failed"));
        await page.goto(
          `${origin}/?${settings ? "window=settings&section=audio" : ""}`,
        );
        try {
          await page
            .getByRole("heading", {
              name: en
                ? "Window could not be loaded"
                : "Fenster konnte nicht geladen werden",
              exact: true,
            })
            .waitFor({ timeout: 1500 });
          if (capture)
            await page.screenshot({
              path: `output/playwright/harden-${en ? "en" : "de"}-${settings ? "settings" : "phone"}-load-error.png`,
            });
        } finally {
          await page.unroute(pattern);
        }
        await page.evaluate<void, AppLocale>(
          async (language: AppLocale) => {
            const { applyLocalization } = (await import(
              /* @vite-ignore */ "/src/lib/i18n.ts" as string
            )) as typeof import("../../../apps/desktop/src/lib/i18n");
            applyLocalization({
              preference: language,
              locale: language,
              revision: 1,
            });
          },
          en ? "de" : "en",
        );
        await page
          .getByRole("heading", {
            name: en
              ? "Fenster konnte nicht geladen werden"
              : "Window could not be loaded",
            exact: true,
          })
          .waitFor({ timeout: 1500 });
        await page
          .getByRole("button", {
            name: en ? "Fenster neu laden" : "Reload window",
            exact: true,
          })
          .click();
        if (settings) {
          await page.locator("#inputDevice").waitFor();
          assert(
            new URL(page.url()).searchParams.get("section") === "audio",
            "Reload lost settings section",
          );
        } else await number().waitFor();
      },
    );

  await check(
    "long account names keep every header action in view",
    async () => {
      await page.goto(origin);
      await number().waitFor();
      await page.evaluate(() => {
        const f = window.__appStoreFixture;
        f.state.accounts[0]!.name = "W".repeat(32) + "🎧日本語مكتب".repeat(4);
        f.emit();
        const invoke = window.__TAURI_INTERNALS__.invoke;
        window.__TAURI_INTERNALS__.invoke = (command, args) =>
          command === "get_notification_permission"
            ? Promise.resolve("denied")
            : invoke(command, args);
        window.dispatchEvent(new Event("focus"));
      });
      await page.locator(".notification-hint").waitFor();
      const outside = await page
        .locator(".app-header button")
        .evaluateAll((buttons) =>
          buttons
            .filter((button) => {
              const r = button.getBoundingClientRect();
              const hit = document.elementFromPoint(
                r.x + r.width / 2,
                r.y + r.height / 2,
              );
              return (
                r.x < 0 ||
                r.right > innerWidth ||
                !(hit === button || button.contains(hit))
              );
            })
            .map((button) => button.getAttribute("aria-label")),
        );
      assert(!outside.length, `Header actions clipped: ${outside.join(", ")}`);
    },
  );

  await check("long caller names do not push hangup out of view", async () => {
    await page.goto(origin);
    await number().waitFor();
    await page.evaluate(() => {
      const f = window.__appStoreFixture;
      f.contacts[0]!.name = "مكتب 🎧 日本語 ".repeat(60);
      f.setCall("active");
    });
    const hangup = page.getByRole("button", {
      name: en ? "Hang up" : "Auflegen",
      exact: true,
    });
    await hangup.waitFor();
    assert(
      (await page.locator(".call-peer").textContent()) ===
        "مكتب 🎧 日本語 ".repeat(60),
      "Caller data was truncated instead of its visual presentation",
    );
    assert(
      await page
        .locator(".call-peer")
        .evaluate((el) => getComputedStyle(el).direction === "rtl"),
      "Caller script direction was lost",
    );
    const visible = await hangup.evaluate((el) => {
      const r = el.getBoundingClientRect();
      const hit = document.elementFromPoint(
        r.x + r.width / 2,
        r.y + r.height / 2,
      );
      return (
        r.top >= 0 &&
        r.bottom <= innerHeight &&
        (hit === el || el.contains(hit))
      );
    });
    assert(visible, "Hangup is hidden by a long caller name");
    if (capture)
      await page.screenshot({
        path: `output/playwright/harden-${en ? "en" : "de"}-long-caller.png`,
      });
  });

  await check(
    "keyboard focus stays clear of sticky call controls",
    async () => {
      await page.goto(origin);
      await number().waitFor();
      await page.evaluate(() => window.__appStoreFixture.setCall("active"));
      await page.locator(".tone-toggle").click();
      await page
        .locator(".call-panel")
        .evaluate((el) => (el.scrollTop = el.scrollHeight));
      for (let index = 0; index < 12; index++) {
        await page.keyboard.press("Tab");
        const focus = await page.evaluate(() => {
          const el = document.activeElement!;
          const r = el.getBoundingClientRect();
          const top = document
            .querySelector(".call-controls")!
            .getBoundingClientRect().bottom;
          const bottom = document
            .querySelector(".call-panel")!
            .getBoundingClientRect().bottom;
          return {
            label: el.getAttribute("aria-label"),
            y: r.y,
            bottom: r.bottom,
            top,
            limit: bottom,
          };
        });
        assert(
          focus.label?.startsWith(en ? "Tone " : "Ton "),
          "Tab left the DTMF keypad unexpectedly",
        );
        assert(
          focus.y >= focus.top && focus.bottom <= focus.limit,
          `Focused tone is covered: ${JSON.stringify(focus)}`,
        );
      }
      await page.locator(".tone-toggle").click();
      await page
        .getByRole("button", {
          name: en ? "Transfer …" : "Vermitteln …",
          exact: true,
        })
        .click();
      const target = page.locator("#transfer-target");
      await target.waitFor();
      const covered = await target.evaluate((el) => {
        const r = el.getBoundingClientRect();
        return (
          r.top <
            document.querySelector(".call-controls")!.getBoundingClientRect()
              .bottom ||
          r.bottom >
            document.querySelector(".call-panel")!.getBoundingClientRect()
              .bottom
        );
      });
      assert(!covered, "Focused transfer destination is covered");
      if (capture)
        await page.screenshot({
          path: `output/playwright/harden-${en ? "en" : "de"}-transfer-focus.png`,
        });
    },
  );

  await check(
    "large contact sets stay searchable without requesting permission",
    async () => {
      await page.goto(origin);
      await number().waitFor();
      await page.evaluate(() => {
        const f = window.__appStoreFixture;
        const contact = f.contacts[0]!;
        f.contacts.splice(
          0,
          f.contacts.length,
          ...Array.from({ length: 1000 }, (_, index) => ({
            ...contact,
            id: `stress-${index}`,
            name: `日本語 🎧 Contact ${index}`,
            favorite: false,
          })),
        );
      });
      await page
        .getByRole("button", {
          name: en ? "Contacts" : "Kontakte",
          exact: true,
        })
        .click();
      await page.locator(".contact-name").first().waitFor();
      await page
        .locator(".contacts-panel input[type=search]")
        .fill("Contact 999");
      await page.getByText("日本語 🎧 Contact 999", { exact: true }).waitFor();
      assert(
        (await page.locator(".contact-entry").count()) === 1,
        "Contact filtering returned the wrong rows",
      );
      assert(
        !(await page.evaluate(() =>
          window.__appStoreFixture.commands.some((c) =>
            c.command.startsWith("request_"),
          ),
        )),
        "Stress test prompted for permission",
      );
    },
  );
  return { passed: failures.length === 0, failures };
}
