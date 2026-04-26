#!/usr/bin/env python3
import sys
import gi
import importlib

gi.require_version('Gtk', '4.0')
gi.require_version('Adw', '1')
from gi.repository import Gtk, Adw

_app = importlib.import_module('app')
AppClass = getattr(_app, '{{name_pascal}}Application')

def main():
    Gtk.init()
    app = AppClass()
    return app.run(sys.argv)

if __name__ == '__main__':
    sys.exit(main())
