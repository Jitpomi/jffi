import gi
import importlib

gi.require_version('Gtk', '4.0')
gi.require_version('Adw', '1')
from gi.repository import Gtk, Adw

_window = importlib.import_module('window')
WindowClass = getattr(_window, '{{name_pascal}}Window')


def _init(self):
    Adw.Application.__init__(self, application_id='com.example.{{name_package}}')
    self.window = None


def _do_activate(self):
    if not self.window:
        self.window = WindowClass(application=self)
    self.window.present()


globals()['{{name_pascal}}Application'] = type(
    '{{name_pascal}}Application',
    (Adw.Application,),
    {'__init__': _init, 'do_activate': _do_activate}
)