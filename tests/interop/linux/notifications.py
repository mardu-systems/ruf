"""Isolated freedesktop test daemon; no user data or real desktop notifications."""
import dbus
import dbus.service
import time
from dbus.mainloop.glib import DBusGMainLoop
from gi.repository import GLib

INTERFACE = "org.freedesktop.Notifications"
DBusGMainLoop(set_as_default=True)


class Notifications(dbus.service.Object):
    def __init__(self):
        self.name = dbus.service.BusName(INTERFACE, bus=dbus.SessionBus())
        super().__init__(self.name, "/org/freedesktop/Notifications")
        self.counter = 0
        self.recorded = (0, "", [], False)
        self.identity = ("", "")
        self.stalled = False

    @dbus.service.method(INTERFACE, out_signature="as")
    def GetCapabilities(self):
        if self.stalled:
            time.sleep(10)
        return ["actions", "body", "body-markup"]

    @dbus.service.method(INTERFACE, in_signature="susssasa{sv}i", out_signature="u")
    def Notify(self, app, replaces, icon, title, body, actions, hints, timeout):
        self.counter += 1
        identifier = replaces or self.counter
        self.recorded = (identifier, body, actions, bool(hints.get("suppress-sound")))
        self.identity = (icon, str(hints.get("desktop-entry", "")))
        return identifier

    @dbus.service.method(INTERFACE, in_signature="u")
    def CloseNotification(self, identifier):
        self.NotificationClosed(identifier, 3)

    @dbus.service.signal(INTERFACE, signature="uu")
    def NotificationClosed(self, identifier, reason):
        pass

    @dbus.service.signal(INTERFACE, signature="us")
    def ActionInvoked(self, identifier, action):
        pass

    @dbus.service.method(INTERFACE, in_signature="us")
    def Activate(self, identifier, action):
        self.ActionInvoked(identifier, action)

    @dbus.service.method(INTERFACE, out_signature="usasb")
    def Recorded(self):
        return self.recorded

    @dbus.service.method(INTERFACE, out_signature="ss")
    def Identity(self):
        return self.identity

    @dbus.service.method(INTERFACE)
    def Stall(self):
        self.stalled = True


daemon = Notifications()
GLib.MainLoop().run()
