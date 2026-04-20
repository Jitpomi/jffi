#!/usr/bin/env python3
import sys
import gi

gi.require_version('Gtk', '4.0')
gi.require_version('Adw', '1')
from gi.repository import Gtk, Adw

from app import {{name_pascal}}Application

def main():
    # Initialize GTK
    Gtk.init()
    
    app = {{name_pascal}}Application()
    return app.run(sys.argv)

if __name__ == '__main__':
    sys.exit(main())
