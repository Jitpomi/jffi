import gi

gi.require_version('Gtk', '4.0')
gi.require_version('Adw', '1')
from gi.repository import Gtk, Adw

from window import {{name_pascal}}Window

class {{name_pascal}}Application(Adw.Application):
    def __init__(self):
        super().__init__(
            application_id='com.example.{{name_package}}',
            flags=Gtk.ApplicationFlags.DEFAULT_FLAGS
        )
        self.window = None

    def do_activate(self):
        if not self.window:
            self.window = {{name_pascal}}Window(application=self)
        self.window.present()