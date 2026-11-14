import type { Page } from "playwright-core";

// Uses the shared synthetic desktop fixture; never starts native SIP or audio.
export default async function stability(page: Page, origin: string) {
  page.setDefaultTimeout(10000);
  await page.goto(origin);
  const language = await page.evaluate(() => window.__rufTestLanguage);
  const en = language === "en";
  const inputName = en ? "Number or extension" : "Nummer oder Nebenstelle";
  const callName = en ? "Call" : "Anrufen";
  const assert = (condition: unknown, description: string) => {
    if (!condition) throw new Error(description);
  };
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));

  for (const viewport of [
    { width: 340, height: 500 },
    { width: 320, height: 440 },
  ]) {
    await page.setViewportSize(viewport);
    await page.goto(origin);
    const input = page.getByRole("textbox", { name: inputName, exact: true });
    await input.waitFor();
    await page.waitForFunction(() =>
      window.__appStoreFixture.commands.some(
        (c) => c.command === "get_contacts",
      ),
    );
    await page.evaluate(() => document.fonts.ready);
    const baseline = await page.locator(".keypad").boundingBox();
    for (const value of ["120", "1201", "999", "++", ""]) {
      await input.fill(value);
      if (value === "120")
        await page.getByRole("button", { name: /Anna Beispiel/ }).waitFor();
      const keypad = await page.locator(".keypad").boundingBox();
      assert(
        baseline &&
          keypad &&
          Math.abs(baseline.y - keypad.y) < 1 &&
          Math.abs(baseline.height - keypad.height) < 1,
        `Dialpad moved while entering ${value} at ${viewport.width}px`,
      );
      const visible = await page
        .getByRole("button", { name: callName, exact: true })
        .evaluate((el) => {
          const rect = el.getBoundingClientRect();
          const atCenter = document.elementFromPoint(
            rect.x + rect.width / 2,
            rect.y + rect.height / 2,
          );
          return (
            rect.y >= 0 &&
            rect.bottom <= innerHeight &&
            (el === atCenter || el.contains(atCenter))
          );
        });
      assert(visible, "Call button must stay visible without scrolling");
    }
    await input.fill("120");
    // Every suggestion stays keyboard-reachable without moving the number keys.
    await input.focus();
    for (let index = 0; index < 5; index++) await page.keyboard.press("Tab");
    const focused = await page.evaluate(() => {
      const el = document.activeElement!;
      const rect = el.getBoundingClientRect();
      const hit = document.elementFromPoint(
        rect.x + rect.width / 2,
        rect.y + rect.height / 2,
      );
      return {
        visible: hit === el || el.contains(hit),
        rect: rect.toJSON(),
        hit: hit?.outerHTML.slice(0, 200),
        scroll: document.querySelector(".dial-suggestions")?.scrollLeft,
      };
    });
    assert(
      focused.visible,
      `Last contact suggestion must be visible on keyboard focus: ${JSON.stringify(focused)}`,
    );
    await page.screenshot({
      path: `output/playwright/stability-${language}-dial-${viewport.width}.png`,
    });
  }

  const beforePermission = (await page.locator(".keypad").boundingBox())!.y;
  await page.evaluate(() => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = (command, args) =>
      command === "get_notification_permission"
        ? Promise.resolve("denied")
        : invoke(command, args);
    window.dispatchEvent(new Event("focus"));
  });
  await page
    .getByRole("button", {
      name: en ? "Check notifications" : "Mitteilungen prüfen",
      exact: true,
    })
    .waitFor();
  assert(
    Math.abs(
      (await page.locator(".keypad").boundingBox())!.y - beforePermission,
    ) < 1,
    "Notification permission changes move the dialpad",
  );

  await page.goto(origin);
  await page.getByRole("textbox", { name: inputName }).waitFor();
  await page.evaluate(() => window.__appStoreFixture.setCall("active"));
  const hangup = page.getByRole("button", {
    name: en ? "Hang up" : "Auflegen",
    exact: true,
  });
  await hangup.waitFor();
  const actionsY = (await page.locator(".call-actions").boundingBox())!.y;
  for (const state of ["pending", "held", "active"]) {
    await page.evaluate((state) => {
      const fixture = window.__appStoreFixture;
      const call = fixture.state.calls![0]!;
      call.holdPending = state === "pending";
      call.localHeld = state === "held";
      fixture.emit();
    }, state);
    await page.waitForTimeout(50);
    assert(
      Math.abs(
        (await page.locator(".call-actions").boundingBox())!.y - actionsY,
      ) < 1,
      "Call actions move with hold status",
    );
  }
  await page.locator(".call-security summary").click();
  assert(
    Math.abs(
      (await page.locator(".call-actions").boundingBox())!.y - actionsY,
    ) < 1,
    "Security details move call actions",
  );
  await page.locator(".call-security summary").click();
  await page.locator(".tone-toggle").click();
  await page
    .getByRole("button", { name: en ? "Tone #" : "Ton #", exact: true })
    .scrollIntoViewIfNeeded();
  for (const name of [
    en ? "Mute" : "Stummschalten",
    en ? "Hang up" : "Auflegen",
  ]) {
    const available = await page
      .getByRole("button", { name, exact: true })
      .evaluate((el) => {
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
    assert(available, `${name} must remain reachable while using DTMF`);
  }
  await page.screenshot({
    path: `output/playwright/stability-${language}-call.png`,
  });
  await page
    .getByRole("button", { name: en ? "Contacts" : "Kontakte", exact: true })
    .click();
  await page
    .getByRole("heading", { name: en ? "Contacts" : "Kontakte", exact: true })
    .waitFor();
  const collection = await page.locator(".contacts-panel").boundingBox();
  const navigation = await page.locator(".bottom-nav").boundingBox();
  assert(
    collection &&
      navigation &&
      collection.height >= 80 &&
      collection.y + 80 <= navigation.y,
    "Expanded call tools leave no room for contacts",
  );

  await page.setViewportSize({ width: 900, height: 660 });
  await page.goto(`${origin}/?window=settings&section=audio`);
  await page.locator("#inputDevice:enabled").waitFor();
  await page.evaluate(() => document.fonts.ready);
  const titleY = (await page.locator("h1").boundingBox())!.y;
  await page.evaluate(() => window.__appStoreFixture.setCall("incoming"));
  await page
    .getByText(
      en
        ? "A call is in progress. End it in the phone window before changing settings."
        : "Ein Gespräch läuft. Beende es im Telefonfenster, bevor du Einstellungen änderst.",
      { exact: true },
    )
    .waitFor();
  assert(
    Math.abs((await page.locator("h1").boundingBox())!.y - titleY) < 1,
    "Incoming call moves the settings form",
  );
  await page.screenshot({
    path: `output/playwright/stability-${language}-settings-call.png`,
  });
  assert(errors.length === 0, errors.join("\n"));
  return { passed: true };
}
