import type { EnglishSmoke } from "./english-smoke";
import type { Page } from "playwright-core";
import type {
  FixtureSnapshot,
  FixtureAccount,
  FixtureCallback,
  FixtureCommand,
} from "./types";
// Run with the repository browser runner; the IPC fixture never contacts SIP.
export default async function settingsRegressions(
  page: Page,
  baseUrl = new URL(page.url()).origin,
  _capture = false,
  englishSmoke?: EnglishSmoke,
) {
  await page.addInitScript(() => {
    window.isTauri = true;
    const scenario = new URLSearchParams(location.search).get("scenario");
    // Permission scenarios model macOS independently of the browser host OS.
    Object.defineProperty(navigator, "platform", {
      value:
        scenario === "windows-audio"
          ? "Win32"
          : scenario === "linux-notifications"
            ? "Linux aarch64"
            : "MacIntel",
    });
    const soundSettings: FixtureSnapshot["soundSettings"] = {
      inputDevice: "",
      outputDevice: "",
      ringDevice: scenario === "devices-failure" ? "saved-speaker" : "",
      ringtoneEnabled: true,
      ringtone: "classic",
      ringtoneVolume: 100,
    };
    const saved: FixtureAccount = {
      id: 1,
      name: "Büro",
      transport: "udp",
      mediaSecurity: "disabled",
      authUsername: null,
      outboundProxy: null,
      registrar: "pbx.example.test",
      username: "alice",
      registration: "registered",
      lastError: null,
      removing: false,
      enabled: true,
      enabledChanging: false,
      registeredAtMs: 1730000000000,
    };
    const snapshot: FixtureSnapshot = {
      revision: 1,
      callConnectedAtMs: null,
      callEvents: [],
      callEventsDropped: 0,
      registration: "registered",
      call: "idle",
      callId: null,
      caller: null,
      callerContact: null,
      historyError: null,
      lastError: null,
      accounts: [saved],
      selectedAccountId: 1,
      callAccountId: null,
      audioMode: "system",
      soundSettings,
      ringtonePlaying: false,
      audioTest: null,
      audioTestError: null,
    };
    const credentialCodes: Partial<
      Record<string, NonNullable<FixtureAccount["lastError"]>["code"]>
    > = {
      "credential-access": "credentialAccessRequired",
      "credential-cancel": "credentialAccessRequired",
      "credential-missing": "credentialMissing",
      "credential-failure": "credentialFailure",
      "credential-storage": "storageFailure",
    };
    const credentialCode = credentialCodes[scenario ?? ""];
    if (credentialCode) {
      saved.registration = "failed";
      saved.lastError = {
        code: credentialCode,
        message: "Synthetic local credential failure",
        sipStatus: null,
      };
      snapshot.registration = "failed";
      snapshot.lastError = saved.lastError;
    }
    const notificationPermission =
      scenario === "linux-notifications" ||
      scenario === "notification-authorized"
        ? "authorized"
        : scenario === "notification-permission"
          ? "notDetermined"
          : scenario === "notification-denied"
            ? "denied"
            : "unsupported";
    const desktop: Window["__settingsTest"]["desktop"] = {
      autostartEnabled: false,
      autostartAvailable: true,
      notificationsEnabled:
        scenario === "notification-permission" ||
        scenario === "notification-denied",
      notificationsGranted:
        notificationPermission === "unsupported"
          ? null
          : notificationPermission === "authorized",
      notificationPermission,
    };
    const callbacks = new Map<number, FixtureCallback>();
    const listeners = new Map<string, number>();
    let callbackId = 0;
    window.__settingsTest = {
      snapshot,
      desktop,
      preferences: {
        language: "system",
        startHidden: true,
        showCallerInNotifications: true,
        contacts: {
          favoritesFirst: true,
          showOrganization: true,
          matchGermanNumbers: true,
          numberRegion: "DE",
        },
        history: { recordingEnabled: true, retentionDays: null },
      },
      requestPermissionResult: "authorized",
      commands: [] as FixtureCommand[],
      closed: false,
      failDesktopSave: true,
      failTestNotification: false,
      failOpenNotificationSettings: false,
      microphonePermission:
        scenario === "windows-audio" ? "unsupported" : "notDetermined",
      failMicrophoneRead: false,
      failMicrophoneRequest: false,
      failOpenMicrophoneSettings: false,
      contactsPermission: "authorized",
      failContactsRead: false,
      failContactsRequest: false,
      failOpenContactsSettings: false,
      failPreferences: scenario === "preferences-failure",
      deviceAttempts: 0,
      holdSnapshot: false,
      resolveSnapshot: null,
      rejectSnapshot: null,
      holdSave: false,
      resolveSave: null,
      emit(payload) {
        payload = { ...payload, revision: this.snapshot.revision + 1 };
        this.snapshot = structuredClone(window.__rufTestSyncDialogs(payload));
        this.event("phone-state-changed", payload);
      },
      event(event, payload = null) {
        const id = listeners.get(event);
        if (id) callbacks.get(id)!({ event, payload });
      },
    };
    window.__TAURI_INTERNALS__ = {
      metadata: {
        currentWindow: { label: "settings" },
        currentWebview: { label: "settings" },
      },
      transformCallback(callback) {
        const id = ++callbackId;
        callbacks.set(id, callback);
        return id;
      },
      invoke: async (command, args) => {
        if (command === "get_localization")
          return {
            preference: "system",
            locale: window.__rufTestLanguage,
            revision: 0,
          };
        const state = window.__settingsTest;
        state.commands.push({ command, args });
        if (command === "plugin:event|listen") {
          listeners.set(args.event!, args.handler!);
          return callbackId;
        }
        if (command === "plugin:event|unlisten") return;
        if (command === "plugin:window|destroy") {
          state.closed = true;
          return;
        }
        if (command === "settings_ready") return;
        if (command === "open_about_link") {
          if (scenario === "about-link-failure")
            throw { code: "nativeFailure", message: "Browser unavailable" };
          return;
        }
        if (command === "quit_app") {
          state.quit = true;
          return;
        }
        if (command === "get_app_info")
          return { version: "0.1.0", telephony: "linked" };
        if (command === "get_engine_snapshot") {
          state.snapshot.revision++;
          const captured = structuredClone(
            window.__rufTestSyncDialogs(state.snapshot),
          );
          if (state.holdSnapshot)
            return new Promise((resolve, reject) => {
              state.resolveSnapshot = () => resolve(captured);
              state.rejectSnapshot = reject;
            });
          return captured;
        }
        if (command === "get_desktop_settings") {
          if (scenario === "desktop-failure")
            throw {
              code: "storageFailure",
              message: "Preferences unavailable",
            };
          return structuredClone(state.desktop);
        }
        if (command === "get_preferences") {
          if (state.failPreferences)
            throw {
              code: "storageFailure",
              message: "Synthetic preference read failure",
            };
          return structuredClone(state.preferences);
        }
        if (command === "save_preferences") {
          if (state.failPreferenceSave)
            throw { code: "storageFailure", message: "Preference save failed" };
          if (
            args.preferences!.history.retentionDays !== null &&
            !args.confirmHistoryDeletion
          )
            throw {
              code: "invalidState",
              message: "Retention requires confirmation",
            };
          state.preferences = structuredClone(args.preferences!);
          return state.preferences;
        }
        if (command === "get_notification_permission") {
          if (scenario === "permission-failure")
            throw {
              code: "nativeFailure",
              message: "Synthetic permission read failure",
            };
          return state.desktop.notificationPermission;
        }
        if (command === "get_microphone_permission") {
          if (state.failMicrophoneRead)
            throw new Error("Synthetic microphone read failure");
          if (state.holdMicrophonePermission)
            return new Promise((resolve) => {
              state.resolveMicrophonePermission = () => resolve("authorized");
            });
          return state.microphonePermission;
        }
        if (command === "request_microphone_permission") {
          if (state.failMicrophoneRequest)
            throw new Error("Synthetic microphone request failure");
          state.microphonePermission = "authorized";
          return state.microphonePermission;
        }
        if (command === "get_contacts_permission") {
          if (state.failContactsRead)
            throw new Error("Synthetic contacts read failure");
          return state.contactsPermission;
        }
        if (command === "request_contacts_permission") {
          if (state.failContactsRequest)
            throw new Error("Synthetic contacts request failure");
          state.contactsPermission = "authorized";
          return state.contactsPermission;
        }
        if (command === "open_contacts_settings") {
          if (state.failOpenContactsSettings)
            throw new Error("Synthetic contacts settings open failure");
          return;
        }
        if (
          command === "open_microphone_settings" &&
          state.failOpenMicrophoneSettings
        )
          throw new Error("Synthetic microphone settings open failure");
        if (command === "start_audio_test") {
          state.snapshot.audioTest = {
            id: "owned-test",
            kind: args.kind!,
            level: 0.2,
          };
          return "owned-test";
        }
        if (command === "stop_audio_test") {
          if (state.snapshot.audioTest?.id === args.id)
            state.snapshot.audioTest = null;
          return;
        }
        if (command === "test_notification" && state.failTestNotification)
          throw {
            code: "nativeFailure",
            message: "Synthetic failed test delivery",
          };
        if (
          command === "open_notification_settings" &&
          state.failOpenNotificationSettings
        )
          throw {
            code: "nativeFailure",
            message: "Synthetic failed system settings open",
          };
        if (
          command === "open_notification_settings" ||
          command === "open_microphone_settings" ||
          command === "test_notification"
        )
          return;
        if (
          command === "get_diagnostics" &&
          scenario === "diagnostics-refresh-failure" &&
          state.commands.filter((entry) => entry.command === command).length ===
            2
        )
          throw { code: "nativeFailure" };
        if (command === "get_diagnostics")
          return {
            version: "0.1.0",
            platform: "macos",
            telephony: "linked",
            accountsTotal: 1,
            accountsRegistered: 1,
            callActive: false,
            notificationPermission: state.desktop.notificationPermission,
            historyAvailable: true,
            lastError: null,
            microphonePermission: "notDetermined",
            audioDevicesAvailable: true,
            credentialError: null,
          };
        if (command === "clear_history" && state.failHistoryClear)
          throw { code: "storageFailure", message: "History deletion failed" };
        if (command === "copy_diagnostics" || command === "clear_history")
          return;
        if (command === "set_account_enabled") {
          state.snapshot.accounts[0]!.enabled = args.enabled!;
          state.snapshot.accounts[0]!.registration = args.enabled
            ? "registered"
            : "unconfigured";
          state.snapshot.revision++;
          return structuredClone(window.__rufTestSyncDialogs(state.snapshot));
        }
        if (command === "select_account") {
          state.snapshot.selectedAccountId = args.id as number;
          state.snapshot.revision++;
          return structuredClone(window.__rufTestSyncDialogs(state.snapshot));
        }
        if (command === "request_notification_permission") {
          state.desktop.notificationPermission = state.requestPermissionResult;
          state.desktop.notificationsGranted =
            state.requestPermissionResult === "unsupported"
              ? null
              : state.requestPermissionResult === "authorized";
          return structuredClone(state.desktop);
        }
        if (command === "get_audio_devices") {
          if (scenario === "devices-failure" && state.deviceAttempts++ === 0)
            throw {
              code: "nativeFailure",
              message: "Device enumeration failed",
            };
          return {
            inputs: [],
            outputs: [
              {
                id: "draft-output",
                name: "Synthetic speaker",
                selectable: true,
              },
            ],
          };
        }
        if (command === "save_desktop_settings") {
          if (state.failDesktopSave)
            throw { code: "storageFailure", message: "Desktop save failed" };
          Object.assign(state.desktop, args);
          return structuredClone(state.desktop);
        }
        if (command === "save_sound_settings") {
          state.snapshot.revision++;
          state.snapshot.soundSettings = { ...args.sound! };
          return structuredClone(window.__rufTestSyncDialogs(state.snapshot));
        }
        if (command === "reconnect_account") {
          if (scenario === "credential-cancel")
            throw {
              code: "credentialAccessRequired",
              message: "Synthetic cancelled permission",
              sipStatus: null,
            };
          if (args.id !== 1 || state.snapshot.call !== "idle")
            throw {
              code: "invalidState",
              message: "Account cannot reconnect now",
              sipStatus: null,
            };
          state.snapshot.revision++;
          state.snapshot.registration = "registering";
          state.snapshot.accounts[0]!.registration = "registering";
          state.snapshot.accounts[0]!.lastError = null;
          state.snapshot.lastError = null;
          return structuredClone(window.__rufTestSyncDialogs(state.snapshot));
        }
        if (command === "save_account") {
          if (state.holdSave) {
            state.snapshot.revision++;
            Object.assign(state.snapshot.accounts[0]!, {
              name: args.name,
              registrar: args.account!.registrar,
              username: args.account!.username,
              transport: args.account!.transport,
              mediaSecurity: args.account!.mediaSecurity,
              authUsername: args.account!.authUsername,
              outboundProxy: args.account!.outboundProxy,
              monitoredExtensions: args.account!.monitoredExtensions.map(
                (target) => ({
                  ...target,
                  state: "unknown" as const,
                }),
              ),
            });
            const captured = structuredClone(
              window.__rufTestSyncDialogs(state.snapshot),
            );
            return new Promise((resolve) => {
              state.resolveSave = () => resolve(captured);
            });
          }
          if (scenario === "busy-save")
            await new Promise<void>((resolve) => {
              state.releaseSave = resolve;
            });
          throw { code: "storageFailure", message: "Save rejected" };
        }
        if (command === "stop_ringtone_preview") {
          if (scenario === "slow-preview-stop")
            await new Promise<void>((resolve) => {
              state.releaseStop = resolve;
            });
          state.snapshot.revision++;
          state.snapshot.ringtonePlaying = false;
          return structuredClone(window.__rufTestSyncDialogs(state.snapshot));
        }
        if (command === "preview_ringtone") {
          state.snapshot.ringtonePlaying = true;
          state.emit(state.snapshot);
          return structuredClone(window.__rufTestSyncDialogs(state.snapshot));
        }
        throw Error(`Unexpected synthetic IPC command: ${command}`);
      },
    };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
  });
  if (englishSmoke) return englishSmoke(page, "settings", baseUrl);
  const failures: string[] = [];
  const check = async (
    name: string,
    scenario: string,
    run: () => Promise<void>,
  ) => {
    try {
      await page.goto(`${baseUrl}/?window=settings&scenario=${scenario}`);
      await page.getByLabel("Name", { exact: true }).waitFor();
      await page.waitForFunction(
        () =>
          !document.body.textContent.includes("Einstellungen werden geladen"),
      );
      await run();
    } catch (error) {
      failures.push(
        `${name}: ${error instanceof Error ? error.message : String(error)}`,
      );
    }
  };
  await check(
    "BLF drafts validate duplicates, survive navigation and submit only configuration",
    "",
    async () => {
      await page
        .getByRole("button", { name: "Nebenstelle hinzufügen", exact: true })
        .click();
      await page.getByLabel("Nebenstelle 1", { exact: true }).fill("1001");
      await page
        .getByLabel("Anzeigename 1 (optional)", { exact: true })
        .fill(" Empfang ");
      await page.getByRole("button", { name: "Ton", exact: true }).click();
      await page
        .getByRole("group", { name: "Ungespeicherte Änderungen" })
        .waitFor();
      await page
        .getByRole("button", { name: "Weiter bearbeiten", exact: true })
        .click();
      await page
        .getByRole("button", { name: "Nebenstelle hinzufügen", exact: true })
        .click();
      await page.getByLabel("Nebenstelle 2", { exact: true }).fill("1001");
      if (_capture) {
        await page.locator(".blf-editor-row").first().scrollIntoViewIfNeeded();
        await page.screenshot({ path: "output/playwright/blf-settings.png" });
      }
      await page
        .getByRole("button", { name: "Speichern & verbinden", exact: true })
        .click();
      await page.locator("#blf-error").waitFor();
      if (
        await page.evaluate(() =>
          window.__settingsTest.commands.some(
            (item) => item.command === "save_account",
          ),
        )
      )
        throw new Error("Duplicate BLF extension reached IPC");
      await page.getByLabel("Nebenstelle 2", { exact: true }).fill("1002");
      await page
        .getByRole("button", { name: "Speichern & verbinden", exact: true })
        .click();
      await page.locator(".settings-feedback.error").first().waitFor();
      const targets = await page.evaluate(
        () =>
          window.__settingsTest.commands.find(
            (item) => item.command === "save_account",
          )?.args.account?.monitoredExtensions,
      );
      if (
        JSON.stringify(targets) !==
        JSON.stringify([
          { extension: "1001", name: "Empfang" },
          { extension: "1002", name: "" },
        ])
      )
        throw new Error(
          "BLF draft configuration was lost: " + JSON.stringify(targets),
        );
      if (
        (await page
          .getByLabel("Nebenstelle 1", { exact: true })
          .inputValue()) !== "1001"
      )
        throw new Error("Failed save lost the BLF draft");
      await page
        .getByRole("button", { name: "Nebenstelle 2 entfernen", exact: true })
        .click();
      if (await page.getByLabel("Nebenstelle 2", { exact: true }).count())
        throw new Error("Removed BLF target is still in draft");
    },
  );
  await check(
    "BLF status refresh preserves saved target configuration without dirtying settings",
    "",
    async () => {
      await page.evaluate(() => {
        const fixture = window.__settingsTest;
        fixture.snapshot.accounts[0]!.monitoredExtensions = [
          { extension: "1001", name: "Empfang", state: "idle" },
        ];
        fixture.emit(fixture.snapshot);
      });
      await page
        .getByLabel("Anzeigename 1 (optional)", { exact: true })
        .waitFor();
      if (
        (await page
          .getByLabel("Anzeigename 1 (optional)", { exact: true })
          .inputValue()) !== "Empfang"
      )
        throw new Error(
          "Saved BLF label was not restored into the account draft",
        );
      await page.evaluate(() => {
        const fixture = window.__settingsTest;
        fixture.snapshot.accounts[0]!.monitoredExtensions![0]!.state = "busy";
        fixture.emit(fixture.snapshot);
      });
      await page.getByRole("button", { name: "Ton", exact: true }).click();
      await page.getByRole("heading", { name: "Ton", exact: true }).waitFor();
      if (
        await page
          .getByRole("group", { name: "Ungespeicherte Änderungen" })
          .count()
      )
        throw new Error(
          "A BLF status update dirtied saved account configuration",
        );
    },
  );
  await check(
    "Windows unsupported permission query preserves manual capture and OS settings guidance",
    "windows-audio",
    async () => {
      await page
        .getByRole("button", { name: "Datenschutz", exact: true })
        .click();
      await page
        .locator(".permission-state")
        .filter({ hasText: "Freigabe kann hier nicht abgefragt werden" })
        .waitFor();
      await page.getByText("Desktop-Apps", { exact: false }).waitFor();
      if (
        await page.evaluate(() =>
          window.__settingsTest.commands.some(({ command }) =>
            ["request_microphone_permission", "start_audio_test"].includes(
              command,
            ),
          ),
        )
      )
        throw Error(
          "Opening Windows privacy settings prompted or captured audio",
        );
      await page
        .locator(".permission-row")
        .getByRole("button", {
          name: "Systemeinstellungen öffnen",
          exact: true,
        })
        .click();
      if (
        !(await page.evaluate(() =>
          window.__settingsTest.commands.some(
            ({ command }) => command === "open_microphone_settings",
          ),
        ))
      )
        throw Error(
          "Windows microphone settings action did not reach native IPC",
        );
      await page.getByRole("button", { name: "Ton", exact: true }).click();
      await page
        .getByText("Standardkommunikationsgerät", { exact: false })
        .waitFor();
      await page
        .getByRole("button", { name: "Mikrofon testen", exact: true })
        .click();
      await page.getByRole("meter").waitFor();
      if (
        !(await page.evaluate(() =>
          window.__settingsTest.commands.some(
            ({ command }) => command === "start_audio_test",
          ),
        ))
      )
        throw Error(
          "Unsupported query incorrectly prevented explicit microphone test",
        );
    },
  );
  await check(
    "microphone settings open failure preserves authorization and retry clears the action error",
    "",
    async () => {
      await page.evaluate(() => {
        window.__settingsTest.microphonePermission = "authorized";
        window.__settingsTest.failOpenMicrophoneSettings = true;
      });
      await page
        .getByRole("button", { name: "Datenschutz", exact: true })
        .click();
      await page
        .locator(".permission-state")
        .getByText("Erlaubt", { exact: true })
        .waitFor();
      await page
        .locator(".permission-row")
        .getByRole("button", {
          name: "Systemeinstellungen öffnen",
          exact: true,
        })
        .click();
      await page.getByRole("alert").waitFor();
      if ((await page.locator(".permission-state").innerText()) !== "Erlaubt")
        throw Error(
          "Opening system settings replaced known microphone authorization",
        );
      await page
        .getByRole("alert")
        .getByText("Die Systemeinstellungen konnten nicht geöffnet werden.", {
          exact: false,
        })
        .waitFor();
      await page.evaluate(() => {
        window.__settingsTest.failOpenMicrophoneSettings = false;
      });
      await page
        .locator(".permission-row")
        .getByRole("button", {
          name: "Systemeinstellungen öffnen",
          exact: true,
        })
        .click();
      if (await page.getByRole("alert").count())
        throw Error("Successful opening retained action error");
    },
  );
  await check(
    "microphone request error is separate from reading permission and allows retry",
    "",
    async () => {
      await page.evaluate(() => {
        window.__settingsTest.failMicrophoneRequest = true;
      });
      await page
        .getByRole("button", { name: "Datenschutz", exact: true })
        .click();
      await page
        .getByRole("button", { name: "Mikrofon erlauben", exact: true })
        .click();
      await page.getByRole("alert").waitFor();
      if (
        (await page.locator(".permission-state").innerText()) !==
        "Noch nicht angefragt"
      )
        throw Error("Request error became a permission read error");
      await page
        .getByRole("alert")
        .getByText("angefordert", { exact: false })
        .waitFor();
      await page.evaluate(() => {
        window.__settingsTest.failMicrophoneRequest = false;
      });
      await page
        .getByRole("button", { name: "Mikrofon erlauben", exact: true })
        .click();
      await page
        .locator(".permission-state")
        .getByText("Erlaubt", { exact: true })
        .waitFor();
      if (await page.getByRole("alert").count())
        throw Error("Successful request retained action error");
    },
  );
  await check(
    "failed microphone read clears stale authorization and recovers without prompting",
    "",
    async () => {
      await page.evaluate(() => {
        window.__settingsTest.microphonePermission = "authorized";
      });
      await page
        .getByRole("button", { name: "Datenschutz", exact: true })
        .click();
      await page
        .locator(".permission-state")
        .getByText("Erlaubt", { exact: true })
        .waitFor();
      await page.evaluate(() => {
        window.__settingsTest.failMicrophoneRead = true;
      });
      await page
        .locator(".permission-row")
        .getByRole("button", { name: "Freigabe erneut prüfen", exact: true })
        .click();
      await page
        .locator(".permission-state")
        .getByText("Prüfung fehlgeschlagen", { exact: true })
        .waitFor();
      await page.evaluate(() => {
        window.__settingsTest.failMicrophoneRead = false;
      });
      await page
        .locator(".permission-row")
        .getByRole("button", { name: "Freigabe erneut prüfen", exact: true })
        .click();
      await page
        .locator(".permission-state")
        .getByText("Erlaubt", { exact: true })
        .waitFor();
      if (
        await page.evaluate(() =>
          window.__settingsTest.commands.some(
            (c) => c.command === "request_microphone_permission",
          ),
        )
      )
        throw Error("A read prompted for microphone permission");
    },
  );
  await check(
    "failed first contacts read stops loading and recovers without requesting access or reading contacts",
    "",
    async () => {
      await page.evaluate(() => {
        window.__settingsTest.failContactsRead = true;
      });
      await page
        .getByRole("button", { name: "Datenschutz", exact: true })
        .click();
      await page.locator(".contacts-permission-error").waitFor();
      if (
        (await page.locator(".permission-status").innerText()) !==
        "Prüfung fehlgeschlagen"
      )
        throw Error("Contacts read failure still announces loading");
      await page.evaluate(() => {
        window.__settingsTest.failContactsRead = false;
      });
      await page
        .getByRole("button", { name: "Freigabe erneut prüfen", exact: true })
        .last()
        .click();
      await page
        .locator(".permission-status")
        .getByText("Freigegeben", { exact: true })
        .waitFor();
      if (await page.getByRole("alert").count())
        throw Error("Retry retained contacts query error");
      if (
        await page.evaluate(() =>
          window.__settingsTest.commands.some((c) =>
            [
              "request_contacts_permission",
              "get_contacts",
              "refresh_contacts",
            ].includes(c.command),
          ),
        )
      )
        throw Error(
          "A contacts permission read prompted or enumerated personal data",
        );
    },
  );
  await check(
    "contacts settings open failure preserves authorization and failed recheck clears stale permission",
    "",
    async () => {
      await page.evaluate(() => {
        window.__settingsTest.failOpenContactsSettings = true;
      });
      await page
        .getByRole("button", { name: "Datenschutz", exact: true })
        .click();
      await page
        .locator(".permission-status")
        .getByText("Freigegeben", { exact: true })
        .waitFor();
      await page
        .getByRole("button", {
          name: "Systemeinstellungen öffnen",
          exact: true,
        })
        .last()
        .click();
      await page.locator(".contacts-permission-error").waitFor();
      if (
        (await page.locator(".permission-status").innerText()) !== "Freigegeben"
      )
        throw Error("Opening contacts settings changed authorization");
      await page.evaluate(() => {
        window.__settingsTest.failOpenContactsSettings = false;
        window.__settingsTest.failContactsRead = true;
      });
      await page
        .getByRole("button", { name: "Freigabe erneut prüfen", exact: true })
        .last()
        .click();
      if (
        (await page.locator(".permission-status").innerText()) !==
        "Prüfung fehlgeschlagen"
      )
        throw Error("Failed contacts recheck retained stale authorization");
    },
  );
  await check(
    "contacts request failure preserves the known state and retry can grant access",
    "",
    async () => {
      await page.evaluate(() => {
        window.__settingsTest.contactsPermission = "notDetermined";
        window.__settingsTest.failContactsRequest = true;
      });
      await page
        .getByRole("button", { name: "Datenschutz", exact: true })
        .click();
      await page
        .getByRole("button", { name: "macOS-Kontakte freigeben", exact: true })
        .click();
      await page.locator(".contacts-permission-error").waitFor();
      if (
        (await page.locator(".permission-status").innerText()) !==
        "Noch nicht angefragt"
      )
        throw Error("Contacts request error became a permission read error");
      await page.evaluate(() => {
        window.__settingsTest.failContactsRequest = false;
      });
      await page
        .getByRole("button", { name: "macOS-Kontakte freigeben", exact: true })
        .click();
      await page
        .locator(".permission-status")
        .getByText("Freigegeben", { exact: true })
        .waitFor();
      if (await page.locator(".contacts-permission-error").count())
        throw Error("Successful contacts request retained its action error");
    },
  );
  await check(
    "test notification failure preserves known authorization and caller draft",
    "notification-permission",
    async () => {
      await page.evaluate(() => {
        window.__settingsTest.desktop.notificationPermission = "authorized";
        window.__settingsTest.failTestNotification = true;
      });
      await page
        .getByRole("button", { name: "Mitteilungen", exact: true })
        .click();
      await page.getByText("Erlaubt", { exact: true }).waitFor();
      await page
        .getByLabel("Anruferdetails anzeigen", { exact: true })
        .uncheck();
      await page
        .getByRole("button", { name: "Testmitteilung senden", exact: true })
        .click();
      await page
        .getByRole("alert")
        .filter({ hasText: "Die Testmitteilung konnte nicht" })
        .waitFor();
      if (
        (await page.locator(".permission-state").textContent())!.trim() !==
        "Erlaubt"
      )
        throw Error("Test delivery failure replaced known OS authorization");
      if (
        await page
          .getByLabel("Anruferdetails anzeigen", { exact: true })
          .isChecked()
      )
        throw Error("Delivery failure reset caller draft");
      if (
        await page
          .getByText("Die Testmitteilung wird in fünf Sekunden angefordert.", {
            exact: false,
          })
          .count()
      )
        throw Error("Failed delivery still claims to be pending");
      await page.evaluate(() => {
        window.__settingsTest.failTestNotification = false;
      });
      await page
        .getByRole("button", { name: "Testmitteilung senden", exact: true })
        .click();
      await page
        .getByText("Testmitteilung angefordert. Fokus und Systemregeln", {
          exact: false,
        })
        .waitFor();
      if (await page.getByRole("alert").count())
        throw Error("Successful retry retained old delivery error");
    },
  );
  await check(
    "opening system settings failure does not masquerade as permission query failure",
    "notification-denied",
    async () => {
      await page.evaluate(() => {
        window.__settingsTest.failOpenNotificationSettings = true;
      });
      await page
        .getByRole("button", { name: "Mitteilungen", exact: true })
        .click();
      await page.getByText("Im System blockiert", { exact: true }).waitFor();
      await page
        .getByRole("button", {
          name: "Systemeinstellungen öffnen",
          exact: true,
        })
        .click();
      await page
        .getByRole("alert")
        .filter({ hasText: "konnten nicht geöffnet" })
        .waitFor();
      if (
        (await page.locator(".permission-state").textContent())!.trim() !==
        "Im System blockiert"
      )
        throw Error("Open failure erased denied authorization state");
    },
  );
  await check(
    "version search focuses the actual About heading",
    "",
    async () => {
      await page
        .getByLabel("Konten und Bereiche suchen", { exact: true })
        .fill("Version");
      await page
        .getByRole("button", {
          name: "Version und Software Über Ruf",
          exact: true,
        })
        .click();
      await page.waitForFunction(
        () => document.activeElement?.id === "about-heading",
        {},
        { timeout: 1500 },
      );
    },
  );
  await check(
    "selected account actions identify their account and headings follow the section",
    "",
    async () => {
      await page
        .getByRole("button", { name: "Deaktivieren · Büro", exact: true })
        .waitFor({ timeout: 1500 });
      if (await page.locator(".account-overview").count())
        throw Error(
          "Selected account editor must not repeat the whole account list",
        );
      await page
        .getByRole("heading", { name: "SIP-Konten", level: 1, exact: true })
        .waitFor({ timeout: 1500 });
      await page
        .getByRole("heading", { name: "Büro", level: 2, exact: true })
        .waitFor({ timeout: 1500 });
      await page
        .getByRole("heading", {
          name: "Kontoinformationen",
          level: 3,
          exact: true,
        })
        .waitFor({ timeout: 1500 });
    },
  );
  await check(
    "long valid account names cannot overflow the inner content or overlap actions",
    "",
    async () => {
      await page.setViewportSize({ width: 740, height: 540 });
      await page.evaluate(() => {
        const state = window.__settingsTest;
        state.snapshot.accounts = Array.from({ length: 8 }, (_, i) => ({
          ...state.snapshot.accounts[0]!,
          id: i + 1,
          name: i === 0 ? "W".repeat(64) : "Auditkonto " + (i + 1),
        }));
        state.emit(state.snapshot);
      });
      const layout = await page
        .locator(".settings-scroll")
        .evaluate((el) => ({ scroll: el.scrollWidth, client: el.clientWidth }));
      if (layout.scroll > layout.client + 1)
        throw Error(
          `Inner settings overflow: ${layout.scroll}/${layout.client}`,
        );
      const name = await page
        .getByRole("heading", { name: "W".repeat(64), level: 2, exact: true })
        .boundingBox();
      const actions = await page
        .getByRole("button", {
          name: `Deaktivieren · ${"W".repeat(64)}`,
          exact: true,
        })
        .boundingBox();
      if (
        !name ||
        !actions ||
        (name.x < actions.x + actions.width &&
          name.x + name.width > actions.x &&
          name.y < actions.y + actions.height &&
          name.y + name.height > actions.y)
      )
        throw Error("Account name overlaps actions");
      await page.setViewportSize({ width: 900, height: 660 });
    },
  );
  await check(
    "every normal visible setting label resolves to its actual control",
    "",
    async () => {
      for (const section of [
        "general",
        "accounts",
        "audio",
        "notifications",
        "contacts",
        "privacy",
        "diagnostics",
        "about",
      ]) {
        await page.goto(`${baseUrl}/?window=settings&section=${section}`);
        await page.waitForFunction(
          () =>
            !document.body.textContent.includes("Einstellungen werden geladen"),
        );
        const missing = await page.evaluate(async () => {
          const { searchSettings } = (await import(
            /* @vite-ignore */ "/src/lib/settings-navigation.ts" as string
          )) as typeof import("../../../apps/desktop/src/lib/settings-navigation");
          return [
            ...document.querySelectorAll<HTMLLabelElement>("main label[for]"),
          ]
            .filter(
              (el) =>
                el.getClientRects().length &&
                !el.closest("details:not([open])"),
            )
            .map((el) => ({ label: el.textContent.trim(), target: el.htmlFor }))
            .filter(
              ({ label, target }) =>
                !searchSettings(label).some(
                  (result) => result.target === target,
                ),
            );
        });
        if (missing.length)
          throw Error(
            `${section}: visible labels missing from search ${JSON.stringify(missing)}`,
          );
      }
    },
  );
  await check(
    "search opens the existing parent disclosure before focusing SBOM",
    "",
    async () => {
      await page
        .getByLabel("Konten und Bereiche suchen", { exact: true })
        .fill("SBOM");
      await page
        .getByRole("button", {
          name: "Projektinventar (SBOM) Über Ruf",
          exact: true,
        })
        .click();
      await page.waitForFunction(
        () => document.activeElement?.id === "software-inventory",
      );
      if (!(await page.locator("#software-inventory").isVisible()))
        throw Error("SBOM search target remains inside closed disclosure");
    },
  );
  await check(
    "About shows authorship, company links and offline licenses",
    "",
    async () => {
      await page.getByRole("button", { name: "Über Ruf", exact: true }).click();
      await page
        .locator("footer")
        .getByText("Autor: Mardu GmbH", { exact: true })
        .waitFor();
      if (
        await page
          .getByRole("list", { name: "Zentrale Softwarekomponenten" })
          .isVisible()
      )
        throw Error("Software details should start collapsed");
      for (const [label, link, href] of [
        ["Website", "company", "https://www.mardu.de/"],
        ["Impressum", "imprint", "https://www.mardu.de/publisher"],
        ["Datenschutz", "privacy", "https://www.mardu.de/privacy"],
      ]) {
        const anchor = page.getByRole("link", {
          name: new RegExp(`^${label}`),
        });
        if ((await anchor.getAttribute("href")) !== href)
          throw Error(`Incorrect ${label} URL`);
        await anchor.click();
        await page.waitForFunction(
          (link) =>
            window.__settingsTest.commands.some(
              (item) =>
                item.command === "open_about_link" && item.args.link === link,
            ),
          link,
        );
      }
      await page.getByText("Software & Lizenzen", { exact: true }).click();
      const overview = page.getByRole("list", {
        name: "Zentrale Softwarekomponenten",
      });
      for (const name of ["Svelte", "Tauri", "rusqlite", "IBM Plex Sans"])
        if (!(await overview.textContent())!.includes(name))
          throw Error(`Missing central component: ${name}`);
      if ((await overview.locator("li").count()) > 8)
        throw Error(
          "The compact software overview should show only central components",
        );
      await page
        .getByText("Lizenztexte & Herkunftshinweise", { exact: true })
        .click();
      await page.getByText("Ruf · MIT-Lizenz", { exact: true }).click();
      const licenseText = page.getByRole("textbox", {
        name: "Ruf MIT-Lizenz",
        exact: true,
      });
      await licenseText.waitFor();
      if (
        !(await licenseText.inputValue()).includes(
          "Permission is hereby granted",
        )
      )
        throw Error("Bundled MIT license missing");
      await page
        .getByText("Lizenztexte & Herkunftshinweise", { exact: true })
        .click();
      await page.getByText("Projektinventar (SBOM)", { exact: true }).click();
      const packages: {
        bomFormat: string;
        components: Array<{ name: string }>;
      } = JSON.parse(
        await page
          .getByRole("textbox", {
            name: "SBOM als CycloneDX-JSON",
            exact: true,
          })
          .inputValue(),
      );
      if (
        packages.bomFormat !== "CycloneDX" ||
        !packages.components.some((item) => item.name === "tauri") ||
        packages.components.some((item) => item.name === "baresip")
      )
        throw Error("Portable preview must expose its own software inventory");
    },
  );
  await check(
    "consumer changelog loads GitHub text without rendering HTML",
    "",
    async () => {
      const route = "https://raw.githubusercontent.com/**";
      await page.route(route, (request) =>
        request.fulfill({
          status: 200,
          contentType: "text/plain",
          body: "# Was ist neu?\n\n## 0.2.0 · Beispiel\n\n- Kontakte schneller finden.\n- <img src=x onerror=alert(1)>\n",
        }),
      );
      try {
        await page
          .getByRole("button", { name: "Über Ruf", exact: true })
          .click();
        await page.getByText("Was ist neu?", { exact: true }).click();
        await page
          .getByRole("heading", { name: "0.2.0 · Beispiel", exact: true })
          .waitFor();
        await page
          .getByText("Kontakte schneller finden.", { exact: true })
          .waitFor();
        if (await page.locator(".changelog img").count())
          throw Error("Remote HTML was rendered");
      } finally {
        await page.unroute(route);
      }
    },
  );
  await check(
    "consumer changelog keeps bundled notes when GitHub is unavailable",
    "",
    async () => {
      const route = "https://raw.githubusercontent.com/**";
      await page.route(route, (request) => request.abort());
      try {
        await page
          .getByRole("button", { name: "Über Ruf", exact: true })
          .click();
        await page.getByText("Was ist neu?", { exact: true }).click();
        await page
          .getByRole("heading", { name: "0.1.0 · Vorschau", exact: true })
          .waitFor();
        await page
          .getByText("Online-Änderungen konnten nicht geladen werden.", {
            exact: false,
          })
          .waitFor();
      } finally {
        await page.unroute(route);
      }
    },
  );
  await check(
    "About reports browser failures without navigating away",
    "about-link-failure",
    async () => {
      await page.getByRole("button", { name: "Über Ruf", exact: true }).click();
      await page.getByRole("link", { name: /^Impressum/ }).click();
      await page
        .getByRole("alert")
        .filter({ hasText: "Die Seite konnte nicht geöffnet werden" })
        .waitFor();
      await page
        .getByRole("heading", { name: "Über Ruf", exact: true })
        .waitFor();
    },
  );
  await check(
    "credential reconnect is explicit, sends only id and preserves drafts",
    "credential-access",
    async () => {
      if (
        await page.evaluate(() =>
          window.__settingsTest.commands.some(
            ({ command }) => command === "reconnect_account",
          ),
        )
      )
        throw new Error(
          "Loading settings prompted for credential access automatically",
        );
      await page
        .getByLabel("Name", { exact: true })
        .fill("Nicht gespeicherter Name");
      await page
        .getByLabel("Passwort", { exact: true })
        .fill("unsaved-secret-draft");
      await page
        .getByRole("button", { name: "Erneut verbinden", exact: true })
        .click();
      await page.getByText("Wird angemeldet …", { exact: true }).waitFor();
      const commands = await page.evaluate(() =>
        window.__settingsTest.commands.filter(
          ({ command }) => command === "reconnect_account",
        ),
      );
      if (
        commands.length !== 1 ||
        JSON.stringify(commands[0]!.args) !== '{"id":1}'
      )
        throw new Error(
          "Reconnect did not use exactly the existing account id",
        );
      if (
        await page.evaluate(() =>
          window.__settingsTest.commands.some(
            ({ command }) => command === "save_account",
          ),
        )
      )
        throw new Error("Reconnect saved unsaved metadata or password");
      if (
        (await page.getByLabel("Name", { exact: true }).inputValue()) !==
          "Nicht gespeicherter Name" ||
        (await page.getByLabel("Passwort", { exact: true }).inputValue()) !==
          "unsaved-secret-draft"
      )
        throw new Error("Successful reconnect discarded the unsaved draft");
    },
  );
  await check(
    "cancelled credential access keeps drafts and allows an explicit retry",
    "credential-cancel",
    async () => {
      await page
        .getByLabel("Name", { exact: true })
        .fill("Entwurf nach Abbruch");
      await page
        .getByLabel("Passwort", { exact: true })
        .fill("unsaved-cancelled-draft");
      await page
        .getByRole("button", { name: "Erneut verbinden", exact: true })
        .click();
      await page.getByRole("alert").waitFor();
      if (
        (await page.getByLabel("Name", { exact: true }).inputValue()) !==
          "Entwurf nach Abbruch" ||
        (await page.getByLabel("Passwort", { exact: true }).inputValue()) !==
          "unsaved-cancelled-draft"
      )
        throw new Error("Cancelled credential access discarded the draft");
      if (
        await page
          .getByRole("button", { name: "Erneut verbinden", exact: true })
          .isDisabled()
      )
        throw new Error("Cancelled access left explicit retry disabled");
      const commands = await page.evaluate(() =>
        window.__settingsTest.commands.filter(
          ({ command }) => command === "reconnect_account",
        ),
      );
      if (
        commands.length !== 1 ||
        JSON.stringify(commands[0]!.args) !== '{"id":1}'
      )
        throw new Error(
          "Cancellation retried automatically or sent the draft password",
        );
    },
  );
  await check(
    "credential reconnect is blocked while a call is present",
    "credential-access",
    async () => {
      await page.evaluate(() => {
        const f = window.__settingsTest;
        Object.assign(f.snapshot!, {
          call: "incoming",
          callId: "credential-call",
          callAccountId: 1,
        });
        f.emit(f.snapshot);
      });
      const reconnect = page.getByRole("button", {
        name: "Erneut verbinden",
        exact: true,
      });
      await reconnect.waitFor();
      if (!(await reconnect.isDisabled()))
        throw new Error("Reconnect remained enabled during a call");
      await reconnect.dispatchEvent("click");
      if (
        await page.evaluate(() =>
          window.__settingsTest.commands.some(
            ({ command }) => command === "reconnect_account",
          ),
        )
      )
        throw new Error(
          "A disabled reconnect still invoked the credential command",
        );
    },
  );
  await check(
    "missing stored credential requires password entry before save",
    "credential-missing",
    async () => {
      if (
        await page
          .getByRole("button", { name: "Erneut verbinden", exact: true })
          .count()
      )
        throw new Error(
          "Missing password offered a futile credential-access retry",
        );
      await page
        .getByRole("button", { name: "Speichern & verbinden", exact: true })
        .click();
      await page
        .getByText("Gib dein SIP-Passwort ein", { exact: false })
        .waitFor();
      if (
        await page.evaluate(() =>
          window.__settingsTest.commands.some(
            ({ command }) =>
              command === "save_account" || command === "reconnect_account",
          ),
        )
      )
        throw new Error(
          "Missing credential initiated a save or prompt without a password",
        );
    },
  );
  await check(
    "continue editing restores focus to the changed field",
    "",
    async () => {
      const input = page.getByLabel("Name", { exact: true });
      await input.fill("Geänderter Entwurf");
      await page.getByRole("button", { name: "Ton", exact: true }).click();
      const keep = page.getByRole("button", {
        name: "Weiter bearbeiten",
        exact: true,
      });
      await keep.waitFor();
      await keep.press("Enter");
      if (!(await input.evaluate((e) => e === document.activeElement)))
        throw new Error(
          "Continue editing did not return focus to the edited field",
        );
      if ((await input.inputValue()) !== "Geänderter Entwurf")
        throw new Error("Continue editing lost the draft");
      await page
        .getByRole("textbox", { name: "Konten und Bereiche suchen" })
        .fill("Ton");
      await page.getByRole("button", { name: "Ton", exact: true }).click();
      await keep.waitFor();
      await keep.press("Escape");
      if (!(await input.evaluate((e) => e === document.activeElement)))
        throw new Error("Escape did not restore the edited field");
    },
  );
  await check(
    "older event cannot re-enable account edits during a newer call",
    "",
    async () => {
      await page
        .getByLabel("Name", { exact: true })
        .fill("Unveränderter Entwurf");
      await page.evaluate(() => {
        const f = window.__settingsTest;
        const older = structuredClone(f.snapshot);
        f.holdSnapshot = true;
        Object.assign(f.snapshot!, {
          call: "incoming",
          callId: "new-call",
          callAccountId: 1,
        });
        f.emit(f.snapshot);
        f.event("phone-state-changed", older);
      });
      if (!(await page.getByLabel("Name", { exact: true }).isDisabled()))
        throw new Error(
          "Older idle event re-enabled edits during the incoming call",
        );
      if (
        (await page.getByLabel("Name", { exact: true }).inputValue()) !==
        "Unveränderter Entwurf"
      )
        throw new Error("State ordering discarded the account draft");
    },
  );
  await check(
    "obsolete read failure preserves a newer call without an error",
    "",
    async () => {
      await page.evaluate(() => {
        const f = window.__settingsTest;
        f.holdSnapshot = true;
        document.dispatchEvent(new Event("visibilitychange"));
      });
      await page.waitForFunction(
        () => window.__settingsTest.rejectSnapshot !== null,
      );
      await page.evaluate(() => {
        const f = window.__settingsTest;
        Object.assign(f.snapshot!, {
          call: "incoming",
          callId: "failure-call",
          callAccountId: 1,
        });
        f.emit(f.snapshot);
        f.rejectSnapshot!({
          code: "nativeFailure",
          message: "Obsolete status failure",
        });
      });
      await page.evaluate(
        () => new Promise((resolve) => setTimeout(resolve, 0)),
      );
      if (!(await page.getByLabel("Name", { exact: true }).isDisabled()))
        throw new Error("Read failure removed the valid incoming call");
      if (await page.getByRole("alert").count())
        throw new Error(
          "Obsolete read failure replaced the newer status with an error",
        );
    },
  );
  await check(
    "newer read survives an older event delivered while pending",
    "",
    async () => {
      await page.evaluate(() => {
        const f = window.__settingsTest;
        f.older = structuredClone(f.snapshot);
        f.holdSnapshot = true;
        Object.assign(f.snapshot!, {
          call: "incoming",
          callId: "read-call",
          callAccountId: 1,
        });
        document.dispatchEvent(new Event("visibilitychange"));
      });
      await page.waitForFunction(
        () => window.__settingsTest.resolveSnapshot !== null,
      );
      await page.evaluate(() => {
        const f = window.__settingsTest;
        f.event("phone-state-changed", f.older);
        f.resolveSnapshot!();
      });
      await page.waitForFunction(() =>
        document.getElementById("name")!.matches(":disabled"),
      );
    },
  );
  await check(
    "newer account save survives an older event delivered while pending",
    "",
    async () => {
      await page.evaluate(() => {
        const f = window.__settingsTest;
        f.older = structuredClone(f.snapshot);
        f.holdSave = true;
        f.holdSnapshot = true;
      });
      await page.getByLabel("Name", { exact: true }).fill("Neuer Kontoname");
      await page
        .getByRole("button", { name: "Speichern & verbinden", exact: true })
        .click();
      await page.waitForFunction(
        () => window.__settingsTest.resolveSave !== null,
      );
      await page.evaluate(() => {
        const f = window.__settingsTest;
        f.event("phone-state-changed", f.older);
        f.resolveSave!();
      });
      await page
        .getByText("Kontoeinstellungen gespeichert.", {
          exact: true,
        })
        .waitFor();
      if (
        (await page.getByLabel("Name", { exact: true }).inputValue()) !==
        "Neuer Kontoname"
      )
        throw new Error("Older event discarded the newer saved account name");
      await page
        .getByRole("navigation", { name: "SIP-Konten", exact: true })
        .getByRole("button", { name: /Neuer Kontoname/ })
        .waitFor();
    },
  );
  await check("failed account save keeps retry draft", "", async () => {
    await page
      .getByLabel("Passwort", { exact: true })
      .fill("synthetic-retry-draft");
    await page
      .getByRole("button", { name: "Speichern & verbinden", exact: true })
      .click();
    await page.getByRole("alert").waitFor();
    if (
      (await page.getByLabel("Passwort", { exact: true }).inputValue()) !==
      "synthetic-retry-draft"
    )
      throw Error("Password draft was lost");
  });
  await check(
    "desktop failure does not discard loaded accounts",
    "desktop-failure",
    async () => {
      if (
        (await page.getByLabel("Name", { exact: true }).inputValue()) !== "Büro"
      )
        throw Error("Engine state was discarded");
      if (await page.getByLabel("Name", { exact: true }).isDisabled())
        throw Error("Account recovery was disabled by preference failure");
    },
  );
  await check("clean account form follows external metadata", "", async () => {
    await page.evaluate(() => {
      const state = window.__settingsTest;
      state.snapshot.accounts[0]!.name = "Aktualisiert";
      state.emit(state.snapshot);
    });
    await page.waitForFunction(
      () =>
        document.querySelector<HTMLInputElement>("#name")!.value ===
        "Aktualisiert",
      null,
      { timeout: 2000 },
    );
  });
  await check(
    "removed account cannot silently become a new account",
    "",
    async () => {
      await page.getByLabel("Name", { exact: true }).fill("Mein Entwurf");
      await page.evaluate(() => {
        const state = window.__settingsTest;
        state.snapshot.accounts = [];
        state.emit(state.snapshot);
      });
      await page
        .getByText("Dieses Konto wurde entfernt.", { exact: false })
        .waitFor({ timeout: 2000 });
      if (
        !(await page
          .getByRole("heading", { name: "Konto entfernt", exact: true })
          .count())
      )
        throw Error("Removed form is labelled as new-account creation");
      if (
        !(await page
          .getByRole("button", { name: "Konto entfernt", exact: true })
          .isDisabled())
      )
        throw Error("Removed draft can be submitted as a new account");
      if (
        (await page.getByLabel("Name", { exact: true }).inputValue()) !==
        "Mein Entwurf"
      )
        throw Error("Removed account draft was lost");
    },
  );
  await check(
    "Linux reports notification service capability without a macOS permission pane",
    "linux-notifications",
    async () => {
      await page
        .getByRole("button", { name: "Mitteilungen", exact: true })
        .click();
      await page
        .getByText("Desktop-Benachrichtigungen verfügbar", { exact: true })
        .waitFor();
      await page
        .getByText(
          "Dein Desktop unterstützt Benachrichtigungen mit Anrufaktionen.",
          { exact: false },
        )
        .waitFor();
      if (
        await page
          .getByRole("button", {
            name: "Systemeinstellungen öffnen",
            exact: true,
          })
          .count()
      )
        throw new Error(
          "Linux offers an unsupported notification settings pane",
        );
      if (
        await page.evaluate(() =>
          window.__settingsTest.commands.some(
            ({ command }) => command === "request_notification_permission",
          ),
        )
      )
        throw new Error(
          "Reading Linux notification capability requested permission",
        );
    },
  );
  await check(
    "enabled defaults retain an explicit first authorization action",
    "notification-permission",
    async () => {
      await page
        .getByRole("button", { name: "Mitteilungen", exact: true })
        .click();
      if (
        !(await page
          .getByLabel("Eingehende Anrufe anzeigen", { exact: true })
          .isChecked())
      )
        throw new Error("Saved enabled notification preference was lost");
      if (
        !(await page
          .getByRole("button", { name: "Mitteilungen speichern", exact: true })
          .isDisabled())
      )
        throw new Error("Unchanged defaults were marked dirty");
      await page
        .getByRole("button", { name: "Mitteilungen erlauben", exact: true })
        .waitFor();
      if (
        await page.evaluate(() =>
          window.__settingsTest.commands.some(
            ({ command }) => command === "request_notification_permission",
          ),
        )
      )
        throw new Error(
          "Settings requested permission without an explicit action",
        );
    },
  );
  await check(
    "denied notifications explain OS recovery without promising another prompt",
    "notification-denied",
    async () => {
      await page
        .getByRole("button", { name: "Mitteilungen", exact: true })
        .click();
      if (
        await page
          .getByRole("button", { name: "Mitteilungen erlauben", exact: true })
          .count()
      )
        throw new Error(
          "Denied permission still offers an ineffective permission prompt",
        );
      await page
        .getByText(
          "Mitteilungen sind in den Systemeinstellungen für Ruf deaktiviert.",
          { exact: false },
        )
        .waitFor();
    },
  );
  await check(
    "unsupported authorization response stays distinct from denial",
    "notification-permission",
    async () => {
      await page
        .getByRole("button", { name: "Mitteilungen", exact: true })
        .click();
      await page.evaluate(
        () => (window.__settingsTest.requestPermissionResult = "unsupported"),
      );
      await page
        .getByRole("button", { name: "Mitteilungen erlauben", exact: true })
        .click();
      await page
        .getByText(
          "Systemmitteilungen sind in diesem Startmodus nicht verfügbar.",
          { exact: false },
        )
        .waitFor();
      if (
        await page
          .getByText(
            "Erlaube Mitteilungen für Ruf in den Systemeinstellungen.",
            { exact: false },
          )
          .count()
      )
        throw new Error(
          "Unsupported context was described as a denied OS preference",
        );
      if (
        await page
          .getByRole("button", { name: "Mitteilungen erlauben", exact: true })
          .count()
      )
        throw new Error("Unsupported context still offers a permission prompt");
    },
  );
  await check(
    "returning from OS permission settings refreshes status and preserves drafts",
    "notification-denied",
    async () => {
      await page
        .getByRole("button", { name: "Mitteilungen", exact: true })
        .click();
      await page
        .getByLabel("Anruferdetails anzeigen", { exact: true })
        .uncheck();
      await page.evaluate(() => {
        const f = window.__settingsTest;
        f.desktop.notificationPermission = "authorized";
        f.desktop.notificationsGranted = true;
        f.commands = [];
        window.dispatchEvent(new Event("focus"));
      });
      await page.waitForFunction(() =>
        window.__settingsTest.commands.some(
          ({ command }) => command === "get_notification_permission",
        ),
      );
      await page
        .getByText("Mitteilungen sind erlaubt. Fokus, Banner-Einstellungen", {
          exact: false,
        })
        .waitFor();
      if (
        await page
          .getByLabel("Anruferdetails anzeigen", { exact: true })
          .isChecked()
      )
        throw new Error("Authorization refresh overwrote the autostart draft");
      if (
        await page
          .getByRole("button", { name: "Mitteilungen speichern", exact: true })
          .isDisabled()
      )
        throw new Error(
          "Authorization refresh reset the dirty preference baseline",
        );
      if (
        await page.evaluate(() =>
          window.__settingsTest.commands.some(
            ({ command }) => command === "request_notification_permission",
          ),
        )
      )
        throw new Error("A focus refresh requested permission automatically");
    },
  );
  await check(
    "notification permission preserves preference drafts",
    "notification-permission",
    async () => {
      await page
        .getByRole("button", { name: "Mitteilungen", exact: true })
        .click();
      await page
        .getByLabel("Anruferdetails anzeigen", { exact: true })
        .uncheck();
      await page
        .getByRole("button", { name: "Mitteilungen erlauben", exact: true })
        .click();
      await page
        .getByText("Mitteilungen sind erlaubt. Fokus, Banner-Einstellungen", {
          exact: false,
        })
        .waitFor();
      if (
        await page
          .getByLabel("Anruferdetails anzeigen", { exact: true })
          .isChecked()
      )
        throw Error(
          "Permission result overwrote unsaved login-start preference",
        );
      if (
        await page
          .getByRole("button", { name: "Mitteilungen speichern", exact: true })
          .isDisabled()
      )
        throw Error("Permission result discarded dirty preference baseline");
      if (
        await page
          .getByRole("button", { name: "Mitteilungen erlauben", exact: true })
          .count()
      )
        throw Error("Granted permission still shown as unavailable");
      const calls = await page.evaluate(
        () =>
          window.__settingsTest.commands.filter(
            (c) => c.command === "request_notification_permission",
          ).length,
      );
      if (calls !== 1)
        throw Error(
          "Permission was requested without an explicit single action",
        );
    },
  );
  await check(
    "OS rights remain readable despite account preference failure",
    "desktop-failure",
    async () => {
      await page
        .getByRole("button", { name: "Mitteilungen", exact: true })
        .click();
      await page.getByText("Nicht verfügbar", { exact: true }).waitFor();
      if (
        !(await page
          .getByRole("button", { name: "Freigabe erneut prüfen", exact: true })
          .isEnabled())
      )
        throw Error("Independent OS check blocked by storage");
      await page
        .getByText("Gespeicherte Werte sind nicht verfügbar.", { exact: true })
        .waitFor();
      if ((await page.locator("#notifications-enabled").count()) !== 0)
        throw Error("Failed preferences looked configured");
      if (
        await page.evaluate(() =>
          window.__settingsTest.commands.some(
            (c) => c.command === "request_notification_permission",
          ),
        )
      )
        throw Error("Read-only permission query prompted");
    },
  );
  await check(
    "failed OS query stays unknown and retryable",
    "permission-failure",
    async () => {
      await page
        .getByRole("button", { name: "Mitteilungen", exact: true })
        .click();
      await page.getByText("Prüfung fehlgeschlagen", { exact: true }).waitFor();
      if (await page.getByText("Im System blockiert", { exact: true }).count())
        throw Error("Failure invented denial");
      if (
        !(await page
          .getByRole("button", { name: "Freigabe erneut prüfen", exact: true })
          .isEnabled())
      )
        throw Error("Retry unavailable");
    },
  );
  await check(
    "explicit test notification works with app switch off and preserves call state",
    "",
    async () => {
      await page.evaluate(() => {
        window.__settingsTest.desktop.notificationPermission = "authorized";
      });
      await page
        .getByRole("button", { name: "Mitteilungen", exact: true })
        .click();
      await page.getByText("Erlaubt", { exact: true }).waitFor();
      if (
        await page
          .getByLabel("Eingehende Anrufe anzeigen", { exact: true })
          .isChecked()
      )
        throw Error("App switch unexpectedly enabled");
      await page
        .getByRole("button", { name: "Testmitteilung senden", exact: true })
        .click();
      await page
        .getByText("Testmitteilung angefordert. Fokus und Systemregeln", {
          exact: false,
        })
        .waitFor();
      const state = await page.evaluate(() => ({
        call: window.__settingsTest.snapshot.call,
        commands: window.__settingsTest.commands.map((c) => c.command),
      }));
      if (
        state.call !== "idle" ||
        state.commands.filter((c) => c === "test_notification").length !== 1 ||
        state.commands.includes("request_notification_permission")
      )
        throw Error("Test notice changed call or authorization");
    },
  );
  await check(
    "field search focuses without discarding edits in the same section",
    "",
    async () => {
      await page.getByLabel("Name", { exact: true }).fill("Mein Entwurf");
      await page
        .getByLabel("Konten und Bereiche suchen", { exact: true })
        .fill("SIP-Konto");
      await page
        .getByRole("button", {
          name: "SIP-Konto bearbeiten SIP-Konten",
          exact: true,
        })
        .click();
      if (
        (await page.getByLabel("Name", { exact: true }).inputValue()) !==
        "Mein Entwurf"
      )
        throw Error("Search reset same-section draft");
      if (
        !(await page
          .getByLabel("Name", { exact: true })
          .evaluate((e) => e === document.activeElement))
      )
        throw Error("Search failed to focus field");
      await page
        .getByRole("button", { name: "Datenschutz", exact: true })
        .click();
      await page
        .getByRole("group", { name: "Ungespeicherte Änderungen" })
        .waitFor();
    },
  );
  await check(
    "account disable preserves selection and exposes native state",
    "",
    async () => {
      await page
        .getByRole("button", { name: "Deaktivieren · Büro", exact: true })
        .click();
      await page
        .getByRole("button", { name: "Aktivieren · Büro", exact: true })
        .waitFor();
      if (
        (await page.evaluate(
          () => window.__settingsTest.snapshot.selectedAccountId,
        )) !== 1
      )
        throw Error("Disable switched caller identity");
      await page
        .getByRole("button", { name: "Aktivieren · Büro", exact: true })
        .click();
      await page
        .getByRole("button", { name: "Deaktivieren · Büro", exact: true })
        .waitFor();
    },
  );
  await check(
    "notification privacy is saved before enabling alerts and partial saves can be retried",
    "notification-authorized",
    async () => {
      await page
        .getByRole("button", { name: "Mitteilungen", exact: true })
        .click();
      await page.evaluate(() => {
        window.__settingsTest.desktop.notificationPermission = "authorized";
        window.__settingsTest.failPreferenceSave = true;
      });
      const enabled = page.locator("#notifications-enabled");
      const caller = page.locator("#notification-caller");
      const save = page.getByRole("button", {
        name: "Mitteilungen speichern",
        exact: true,
      });
      const alerts = page.locator(".settings-notices [role=alert]");
      await enabled.check();
      await caller.uncheck();
      await save.click();
      await alerts.waitFor();
      if (_capture)
        await page.screenshot({
          path: "output/playwright/settings-notifications-save-error.png",
        });
      const firstWrites = await page.evaluate(() =>
        window.__settingsTest.commands
          .filter(({ command }) => command.startsWith("save_"))
          .map(({ command }) => command),
      );
      if (firstWrites.join(",") !== "save_preferences")
        throw Error(
          "Alerts were enabled or attempted before saving caller privacy",
        );
      if (await caller.isChecked())
        throw Error("Failed save discarded caller privacy draft");
      await page.evaluate(() => {
        window.__settingsTest.failPreferenceSave = false;
      });
      await save.click();
      await page
        .getByText(
          "Anruferdetails gespeichert. Die Aktivierung der Mitteilungen ist nicht bestätigt.",
          { exact: true },
        )
        .waitFor();
      if (_capture)
        await page.screenshot({
          path: "output/playwright/settings-notifications-partial-save.png",
        });
      const partial = await page.evaluate(() => ({
        enabled: window.__settingsTest.desktop.notificationsEnabled,
        details: window.__settingsTest.preferences.showCallerInNotifications,
      }));
      if (partial.enabled || partial.details)
        throw Error("Teilerfolg wurde falsch persistiert");
      await page.evaluate(() => {
        window.__settingsTest.failDesktopSave = false;
      });
      await save.click();
      await page
        .getByText("Einstellungen gespeichert.", { exact: true })
        .waitFor();
      if (
        !(await page.evaluate(
          () => window.__settingsTest.desktop.notificationsEnabled,
        ))
      )
        throw Error("Retry did not enable anonymous alerts");
      if (_capture)
        await page.screenshot({
          path: "output/playwright/settings-notifications-saved.png",
        });
    },
  );
  await check(
    "saving a disabled account never claims a registration",
    "",
    async () => {
      await page
        .getByRole("button", { name: "Deaktivieren · Büro", exact: true })
        .click();
      await page
        .getByRole("button", { name: "Aktivieren · Büro", exact: true })
        .waitFor();
      await page.evaluate(() => {
        window.__settingsTest.holdSave = true;
      });
      await page.getByLabel("Name", { exact: true }).fill("Empfang");
      await page
        .getByRole("button", { name: "Speichern", exact: true })
        .click({ timeout: 1500 });
      await page.waitForFunction(() => !!window.__settingsTest.resolveSave);
      await page.evaluate(() => window.__settingsTest.resolveSave!());
      await page
        .getByText("Gespeichert. Das Konto bleibt deaktiviert.", {
          exact: true,
        })
        .waitFor();
      await page
        .getByRole("button", { name: "Aktivieren · Empfang", exact: true })
        .waitFor();
      if (_capture)
        await page.screenshot({
          path: "output/playwright/settings-account-disabled-saved.png",
        });
    },
  );
  await check(
    "during a call only recording is locked and history deletion returns focus",
    "",
    async () => {
      await page
        .getByRole("button", { name: "Kontakte & Verlauf", exact: true })
        .click();
      const recording = page.locator("#history-recording");
      const save = page.getByRole("button", {
        name: "Anzeige & Verlauf speichern",
        exact: true,
      });
      await recording.uncheck();
      await page.locator("#favorites-first").uncheck();
      await page.evaluate(() => {
        const f = window.__settingsTest;
        f.emit({
          ...f.snapshot,
          call: "active",
          callId: "history-call",
          callAccountId: 1,
          callConnectedAtMs: Date.now(),
        });
      });
      await page
        .getByText(/Nimm die Änderung an der Verlaufserfassung zurück/)
        .waitFor();
      if (await save.isEnabled())
        throw Error("Recording draft can be sent during a call");
      if (_capture)
        await page.screenshot({
          path: "output/playwright/settings-history-recording-draft-locked.png",
        });
      await recording.check();
      await page.waitForFunction(() =>
        document.querySelector("#history-recording")?.matches(":disabled"),
      );
      if (!(await page.locator("#history-retention").isEnabled()))
        throw Error("Retention is locked during a call");
      await save.click();
      await page
        .getByText("Einstellungen gespeichert.", { exact: true })
        .waitFor();
      if (
        await page.evaluate(
          () => window.__settingsTest.preferences.contacts.favoritesFirst,
        )
      )
        throw Error("Contact preference was not saved during a call");
      const clear = page.getByRole("button", {
        name: "Verlauf löschen …",
        exact: true,
      });
      await clear.click();
      await page.waitForFunction(
        () => document.activeElement?.textContent?.trim() === "Abbrechen",
      );
      await page
        .getByRole("button", { name: "Abbrechen", exact: true })
        .click();
      await page.waitForFunction(
        () => document.activeElement?.id === "clear-history",
      );
      await page.evaluate(() => {
        window.__settingsTest.failHistoryClear = true;
      });
      await clear.click();
      const confirm = page.getByRole("button", {
        name: "Verlauf endgültig löschen",
        exact: true,
      });
      await confirm.click();
      await page
        .getByRole("group", { name: /Alle abgeschlossenen Anrufe/ })
        .getByRole("alert")
        .waitFor();
      await page.evaluate(() => {
        window.__settingsTest.failHistoryClear = false;
      });
      await confirm.click();
      await page
        .getByText("Abgeschlossene Anrufe wurden aus dem Verlauf gelöscht.", {
          exact: true,
        })
        .waitFor();
      await page.waitForFunction(
        () => document.activeElement?.id === "clear-history",
      );
    },
  );
  await check(
    "switching sections resets scroll and field search still navigates to the target",
    "",
    async () => {
      const content = page.locator(".settings-scroll");
      await content.evaluate((el) => (el.scrollTop = el.scrollHeight));
      await page
        .getByRole("button", { name: "Diagnose & Hilfe", exact: true })
        .click();
      await page.getByText("Aktueller Zustand", { exact: true }).waitFor();
      if ((await content.evaluate((el) => el.scrollTop)) !== 0)
        throw Error("New section retained the previous scroll position");
      await page.locator("#settings-search").fill("Aufbewahrung");
      await page
        .getByRole("navigation", { name: "Gefundene Einstellungen" })
        .getByRole("button", { name: /Aufbewahrung/ })
        .click();
      await page.waitForFunction(
        () => document.activeElement?.id === "history-retention",
      );
    },
  );
  await check(
    "regional number matching can be selected, disabled and restored",
    "",
    async () => {
      await page
        .getByRole("button", { name: "Kontakte & Verlauf", exact: true })
        .click();
      const region = page.getByLabel(
        "Land oder Region für nationale Rufnummern",
        { exact: true },
      );
      const on = page.getByLabel(
        "Nationale und internationale Rufnummern abgleichen",
        { exact: true },
      );
      const save = page.getByRole("button", {
        name: "Anzeige & Verlauf speichern",
        exact: true,
      });
      if ((await region.inputValue()) !== "DE")
        throw Error("Existing German region changed");
      await region.selectOption("GB");
      await save.click();
      await page.waitForFunction(
        () => window.__settingsTest.preferences.contacts.numberRegion === "GB",
      );
      if (_capture) {
        await region.scrollIntoViewIfNeeded();
        await page.screenshot({ path: "output/playwright/number-region.png" });
      }
      await on.uncheck();
      if (!(await region.isDisabled()))
        throw Error(
          "Disabled regional matching offers an active region selector",
        );
      await save.click();
      await page.waitForFunction(
        () =>
          window.__settingsTest.preferences.contacts.matchGermanNumbers ===
          false,
      );
      await on.check();
      if ((await region.inputValue()) !== "GB")
        throw Error("Toggling regional matching discarded the saved region");
      await save.click();
      await page.waitForFunction(() => {
        const { matchGermanNumbers, numberRegion } =
          window.__settingsTest.preferences.contacts;
        return matchGermanNumbers && numberRegion === "GB";
      });
    },
  );
  await check(
    "history retention requires confirmation and settings preserve draft",
    "",
    async () => {
      await page
        .getByRole("button", { name: "Kontakte & Verlauf", exact: true })
        .click();
      await page
        .getByLabel("Aufbewahrung", { exact: true })
        .selectOption({ label: "30 Tage" });
      await page
        .getByRole("button", {
          name: "Anzeige & Verlauf speichern",
          exact: true,
        })
        .click();
      await page.getByRole("alert").waitFor();
      await page
        .getByLabel("Löschung älterer Einträge bestätigen", { exact: true })
        .check();
      await page
        .getByRole("button", {
          name: "Anzeige & Verlauf speichern",
          exact: true,
        })
        .click();
      await page
        .getByText("Einstellungen gespeichert.", { exact: true })
        .waitFor();
      if (
        (await page.evaluate(
          () => window.__settingsTest.preferences.history.retentionDays,
        )) !== 30
      )
        throw Error("Retention not saved");
      await page
        .getByRole("button", { name: "Verlauf löschen …", exact: true })
        .click();
      if (
        await page.evaluate(() =>
          window.__settingsTest.commands.some(
            (c) => c.command === "clear_history",
          ),
        )
      )
        throw Error("History deleted before confirmation");
      await page
        .getByRole("button", { name: "Abbrechen", exact: true })
        .click();
    },
  );
  await check(
    "late microphone permission cannot start capture after navigation",
    "",
    async () => {
      await page.getByRole("button", { name: "Ton", exact: true }).click();
      await page.evaluate(() => {
        window.__settingsTest.holdMicrophonePermission = true;
      });
      await page
        .getByRole("button", { name: "Mikrofon testen", exact: true })
        .click();
      await page.waitForFunction(
        () =>
          typeof window.__settingsTest.resolveMicrophonePermission ===
          "function",
      );
      await page.getByRole("button", { name: "Über Ruf", exact: true }).click();
      await page.evaluate(() =>
        window.__settingsTest.resolveMicrophonePermission!(),
      );
      await page.waitForTimeout(150);
      if (
        await page.evaluate(() =>
          window.__settingsTest.commands.some(
            (c) => c.command === "start_audio_test",
          ),
        )
      )
        throw Error("Late permission started microphone after leaving audio");
    },
  );
  await check(
    "speaker test uses draft call output and owned test stops on navigation",
    "",
    async () => {
      await page.getByRole("button", { name: "Ton", exact: true }).click();
      await page
        .getByLabel("Lautsprecher", { exact: true })
        .selectOption("draft-output");
      await page
        .getByRole("button", { name: "Lautsprecher testen", exact: true })
        .click();
      await page
        .getByRole("button", { name: "Audiotest stoppen", exact: true })
        .waitFor();
      await page.getByRole("button", { name: "Über Ruf", exact: true }).click();
      await page
        .getByRole("button", { name: "Änderungen verwerfen", exact: true })
        .click();
      await page.waitForFunction(() =>
        window.__settingsTest.commands.some(
          (c) => c.command === "stop_audio_test" && c.args.id === "owned-test",
        ),
      );
      const test = await page.evaluate(() =>
        window.__settingsTest.commands.find(
          (c) => c.command === "start_audio_test",
        ),
      );
      if (test!.args.kind !== "speaker" || test!.args.device !== "draft-output")
        throw Error("Speaker test used wrong output");
    },
  );
  await check(
    "unknown preferences are not shown as saved defaults",
    "preferences-failure",
    async () => {
      await page
        .getByRole("button", { name: "Allgemein", exact: true })
        .click();
      await page
        .getByText("Gespeicherte Werte sind nicht verfügbar.", { exact: true })
        .waitFor({ timeout: 1500 });
      if ((await page.locator("#language-preference").count()) !== 0)
        throw Error("Default language is shown as a saved value");
      await page
        .getByRole("button", { name: "Kontakte & Verlauf", exact: true })
        .click();
      await page
        .getByText("Gespeicherte Werte sind nicht verfügbar.", { exact: true })
        .waitFor();
      if ((await page.locator("#history-recording").count()) !== 0)
        throw Error("Default recording preference is shown as saved");
      if (!(await page.locator("#clear-history").isEnabled()))
        throw Error(
          "Reading preferences incorrectly controls history deletion",
        );
    },
  );
  await check(
    "failed preference load can recover without reopening",
    "preferences-failure",
    async () => {
      await page
        .getByRole("button", { name: "Mitteilungen", exact: true })
        .click();
      await page
        .getByText("Gespeicherte Werte sind nicht verfügbar.", { exact: true })
        .waitFor();
      if ((await page.locator("#notification-caller").count()) !== 0)
        throw Error("Unknown caller privacy preference looked configured");
      await page.evaluate(() => {
        window.__settingsTest.failPreferences = false;
      });
      await page
        .getByRole("button", {
          name: "Einstellungen erneut laden",
          exact: true,
        })
        .click();
      await page
        .getByLabel("Anruferdetails anzeigen", { exact: true })
        .waitFor({ state: "visible" });
      await page.waitForFunction(
        () =>
          !document.querySelector<HTMLInputElement>("#notification-caller")!
            .disabled &&
          !document.getElementById("notification-caller")!.closest("fieldset")!
            .disabled,
      );
      if (
        !(await page
          .getByLabel("Anruferdetails anzeigen", { exact: true })
          .isChecked())
      )
        throw Error("Recovery lost saved preference");
    },
  );
  await check("window close protects an account draft", "", async () => {
    await page.getByLabel("Name", { exact: true }).fill("Nicht verlieren");
    await page.evaluate(() =>
      window.__settingsTest.event("tauri://close-requested"),
    );
    await page
      .getByRole("group", { name: "Ungespeicherte Änderungen" })
      .waitFor({ timeout: 2000 });
    if (await page.evaluate(() => window.__settingsTest.closed))
      throw Error("Dirty window closed");
    await page
      .getByRole("button", { name: "Weiter bearbeiten", exact: true })
      .click();
    if (
      (await page.getByLabel("Name", { exact: true }).inputValue()) !==
      "Nicht verlieren"
    )
      throw Error("Kept draft was lost");
    await page.evaluate(() =>
      window.__settingsTest.event("tauri://close-requested"),
    );
    await page
      .getByRole("button", { name: "Änderungen verwerfen", exact: true })
      .click();
    await page.waitForFunction(() => window.__settingsTest.closed, null, {
      timeout: 2000,
    });
  });
  await check("application quit uses the draft guard", "", async () => {
    await page.getByLabel("Name", { exact: true }).fill("Nicht beenden");
    await page.evaluate(() =>
      window.__settingsTest.event("settings-quit-requested"),
    );
    await page
      .getByRole("group", { name: "Ungespeicherte Änderungen" })
      .waitFor({ timeout: 2000 });
    if (await page.evaluate(() => window.__settingsTest.quit))
      throw Error("Dirty application quit");
    await page
      .getByRole("button", { name: "Änderungen verwerfen", exact: true })
      .click();
    await page.waitForFunction(() => window.__settingsTest.quit, null, {
      timeout: 2000,
    });
  });
  await check(
    "ringtone preview plays the unsaved sound draft",
    "",
    async () => {
      await page.getByRole("button", { name: "Ton", exact: true }).click();
      await page.locator("#ringDevice").selectOption("draft-output");
      await page.locator("#ringtone-style").selectOption("short");
      await page.locator("#ringtone-volume").fill("32");
      await page
        .getByRole("button", { name: "Klingelton anhören", exact: true })
        .click();
      await page
        .getByRole("button", { name: "Hörprobe stoppen", exact: true })
        .waitFor();
      const { preview, writes, sound } = await page.evaluate(() => {
        const { commands, snapshot } = window.__settingsTest;
        return {
          preview: commands
            .filter((item) => item.command === "preview_ringtone")
            .at(-1)?.args,
          writes: commands.filter(
            (item) => item.command === "save_sound_settings",
          ),
          sound: snapshot.soundSettings,
        };
      });
      if (
        !preview ||
        preview.device !== "draft-output" ||
        preview.ringtone !== "short" ||
        preview.ringtoneVolume !== 32 ||
        writes.length ||
        sound.ringtone !== "classic" ||
        sound.ringtoneVolume !== 100 ||
        sound.ringDevice !== ""
      )
        throw Error("Ringtone preview must use the draft without saving it");
      const save = page.getByRole("button", {
        name: "Ton speichern",
        exact: true,
      });
      if (!(await save.isDisabled()))
        throw Error("Sound can be saved during the exclusive preview");
      await page
        .getByRole("button", { name: "Hörprobe stoppen", exact: true })
        .click();
      await page
        .getByRole("button", { name: "Klingelton anhören", exact: true })
        .waitFor();
      if (!(await save.isEnabled()))
        throw Error("Ringtone preview incorrectly cleared the draft");
      await page
        .getByRole("button", { name: "Verwerfen", exact: true })
        .click();
      if (
        (await page.locator("#ringtone-style").inputValue()) !== "classic" ||
        (await page.locator("#ringtone-volume").inputValue()) !== "100" ||
        (await page.locator("#ringDevice").inputValue()) !== ""
      )
        throw Error("Discard did not restore the applied ringtone values");
    },
  );
  await check("leaving sound stops the owned preview", "", async () => {
    await page.getByRole("button", { name: "Ton", exact: true }).click();
    await page
      .getByRole("button", { name: "Klingelton anhören", exact: true })
      .click();
    await page
      .getByRole("button", { name: "Hörprobe stoppen", exact: true })
      .waitFor();
    await page.getByRole("button", { name: "Über Ruf", exact: true }).click();
    await page.waitForFunction(
      () =>
        window.__settingsTest.commands.some(
          (item) => item.command === "stop_ringtone_preview",
        ),
      null,
      { timeout: 2000 },
    );
  });
  await check(
    "incoming call freezes and restores the account draft",
    "",
    async () => {
      await page.getByLabel("Name", { exact: true }).fill("Entwurf bleibt");
      await page.evaluate(() => {
        const state = window.__settingsTest;
        state.snapshot.call = "incoming";
        state.snapshot.callId = "synthetic-call";
        state.snapshot.callAccountId = 1;
        state.emit(state.snapshot);
      });
      if (!(await page.getByLabel("Name", { exact: true }).isDisabled()))
        throw Error("Call did not freeze edits");
      await page.evaluate(() => {
        const state = window.__settingsTest;
        state.snapshot.call = "idle";
        state.snapshot.callId = null;
        state.snapshot.callAccountId = null;
        state.emit(state.snapshot);
      });
      await page.waitForFunction(
        () => !document.getElementById("name")!.matches(":disabled"),
      );
      if (
        (await page.getByLabel("Name", { exact: true }).inputValue()) !==
        "Entwurf bleibt"
      )
        throw Error("Call discarded draft");
    },
  );
  await check("failed desktop save preserves draft for retry", "", async () => {
    await page
      .getByRole("button", { name: "Mitteilungen", exact: true })
      .click();
    await page
      .getByLabel("Eingehende Anrufe anzeigen", { exact: true })
      .check();
    await page
      .getByRole("button", { name: "Mitteilungen speichern", exact: true })
      .click();
    await page.getByRole("alert").waitFor();
    if (
      !(await page
        .getByLabel("Eingehende Anrufe anzeigen", { exact: true })
        .isChecked())
    )
      throw Error("Notification preference draft lost");
    await page.evaluate(() => {
      window.__settingsTest.failDesktopSave = false;
    });
    await page
      .getByRole("button", { name: "Mitteilungen speichern", exact: true })
      .click();
    await page
      .getByText("Einstellungen gespeichert.", { exact: true })
      .waitFor();
    if (
      !(await page
        .getByRole("button", { name: "Mitteilungen speichern", exact: true })
        .isDisabled())
    )
      throw Error("Successful save remained dirty");
  });
  await check(
    "audio enumeration retries and missing output can use system default",
    "devices-failure",
    async () => {
      await page.getByRole("button", { name: "Ton", exact: true }).click();
      await page.getByRole("alert").waitFor();
      await page
        .getByRole("button", { name: "Geräte aktualisieren", exact: true })
        .click();
      await page
        .getByText("Gespeichertes Gerät nicht verfügbar.", { exact: false })
        .waitFor();
      await page
        .getByLabel("Klingelton-Ausgabe", { exact: true })
        .selectOption("");
      await page
        .getByRole("button", { name: "Ton speichern", exact: true })
        .click();
      await page
        .getByText("Ton-Einstellungen gespeichert.", { exact: false })
        .waitFor();
      if (
        (await page
          .getByLabel("Klingelton-Ausgabe", { exact: true })
          .inputValue()) !== ""
      )
        throw Error("Missing output could not recover to system default");
    },
  );
  await check(
    "zero search results can be cleared without losing the current form",
    "",
    async () => {
      await page
        .getByLabel("Konten und Bereiche suchen", { exact: true })
        .fill("kein-treffer-xyz");
      await page.getByText("Keine Treffer", { exact: false }).waitFor();
      await page
        .getByRole("button", { name: "Suche leeren", exact: true })
        .click();
      await page
        .getByRole("button", { name: "Allgemein", exact: true })
        .waitFor();
      if (
        (await page.getByLabel("Name", { exact: true }).inputValue()) !== "Büro"
      )
        throw Error("Search altered account form");
    },
  );
  await check(
    "window close waits while a save is in flight",
    "busy-save",
    async () => {
      await page
        .getByLabel("Name", { exact: true })
        .fill("Noch nicht gespeichert");
      await page
        .getByRole("button", { name: "Speichern & verbinden", exact: true })
        .click();
      await page.waitForFunction(
        () => typeof window.__settingsTest.releaseSave === "function",
      );
      await page.evaluate(() =>
        window.__settingsTest.event("tauri://close-requested"),
      );
      await page.getByRole("alert").waitFor();
      if (await page.evaluate(() => window.__settingsTest.closed))
        throw Error("Window closed during pending save");
      await page.evaluate(() => window.__settingsTest.releaseSave!());
      await page
        .getByRole("button", { name: "Speichern & verbinden", exact: true })
        .waitFor();
      await page.evaluate(() =>
        window.__settingsTest.event("tauri://close-requested"),
      );
      await page
        .getByRole("group", { name: "Ungespeicherte Änderungen" })
        .waitFor();
    },
  );
  await check(
    "closing while preview stops prevents late edits",
    "slow-preview-stop",
    async () => {
      await page.getByRole("button", { name: "Ton", exact: true }).click();
      await page
        .getByRole("button", { name: "Klingelton anhören", exact: true })
        .click();
      await page
        .getByRole("button", { name: "Hörprobe stoppen", exact: true })
        .waitFor();
      await page.evaluate(() =>
        window.__settingsTest.event("tauri://close-requested"),
      );
      await page.waitForFunction(
        () => typeof window.__settingsTest.releaseStop === "function",
      );
      const blocked = await page
        .getByLabel("Bei Anrufen klingeln", { exact: true })
        .isDisabled();
      await page.evaluate(() => window.__settingsTest.releaseStop!());
      if (!blocked)
        throw Error("New edits were accepted after close was confirmed");
      await page.waitForFunction(() => window.__settingsTest.closed);
    },
  );
  await check(
    "diagnostic refresh failure labels the last report and keeps copying explicit",
    "diagnostics-refresh-failure",
    async () => {
      await page
        .getByRole("button", { name: "Diagnose & Hilfe", exact: true })
        .click();
      const refresh = page.getByRole("button", {
        name: "Status erneut prüfen",
        exact: true,
      });
      const copy = page.getByRole("button", {
        name: "Diagnosebericht kopieren",
        exact: true,
      });
      await page
        .getByText("1 von 1 Konten haben eine bestätigte Anmeldung.", {
          exact: true,
        })
        .waitFor();
      if (_capture)
        await page.screenshot({
          path: "output/playwright/diagnostics-settings-loaded.png",
        });
      await page.evaluate(() => {
        Object.defineProperty(navigator, "clipboard", {
          configurable: true,
          value: {
            async writeText() {
              throw Error("Synthetic clipboard failure");
            },
          },
        });
      });
      await copy.click();
      await page.waitForFunction(() =>
        window.__settingsTest.commands.some(
          ({ command }) => command === "copy_diagnostics",
        ),
      );
      await refresh.click();
      await page.getByRole("alert").waitFor();
      const last = page.getByRole("heading", {
        name: "Letzter erfolgreicher Stand",
        exact: true,
      });
      await last.waitFor({ timeout: 3_000 });
      const copyLast = page.getByRole("button", {
        name: "Letzten Bericht kopieren",
        exact: true,
      });
      if (!(await page.locator("time[datetime]").isVisible()))
        throw Error("Retained report lost its observation time");
      if (_capture) {
        await last.scrollIntoViewIfNeeded();
        await page.screenshot({
          path: "output/playwright/diagnostics-settings-stale.png",
        });
      }
      await copyLast.click();
      await page
        .getByText("Der letzte Diagnosebericht konnte nicht kopiert werden.", {
          exact: true,
        })
        .waitFor({ timeout: 3_000 });
      if (
        (await page.evaluate(
          () =>
            window.__settingsTest.commands.filter(
              ({ command }) => command === "copy_diagnostics",
            ).length,
        )) !== 1
      )
        throw Error("Copying the last report attempted a fresh native copy");
      await page.evaluate(() => {
        Object.defineProperty(navigator, "clipboard", {
          configurable: true,
          value: { async writeText() {} },
        });
      });
      await copyLast.click();
      await page
        .getByText("Der letzte erfolgreiche Diagnosebericht wurde kopiert.", {
          exact: true,
        })
        .waitFor({ timeout: 3_000 });
      await refresh.click();
      const current = page.getByRole("heading", {
        name: "Aktueller Zustand",
        exact: true,
      });
      await current.waitFor({ timeout: 3_000 });
      await page.getByRole("alert").waitFor({ state: "hidden" });
      if (
        await page
          .getByText("Der letzte erfolgreiche Diagnosebericht wurde kopiert.", {
            exact: true,
          })
          .isVisible()
      )
        throw Error("Copy feedback outlived the report it applied to");
      if (_capture) {
        await current.scrollIntoViewIfNeeded();
        await page.screenshot({
          path: "output/playwright/diagnostics-settings-recovered.png",
        });
      }
    },
  );
  if (failures.length) throw Error(failures.join("\n"));
}
