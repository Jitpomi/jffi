import gi

gi.require_version('Gtk', '4.0')
gi.require_version('Adw', '1')
from gi.repository import Gtk, Adw

from window import TestlinuxWindow

class TestlinuxApplication(Adw.Application):
    def __init__(self):
        super().__init__(application_id='com.example.testlinux')
        self.window = None
    
    def do_activate(self):
        if not self.window:
            self.window = TestlinuxWindow(application=self)
        self.window.present()
