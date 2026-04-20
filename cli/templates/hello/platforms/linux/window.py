import gi

gi.require_version('Gtk', '4.0')
gi.require_version('Adw', '1')
from gi.repository import Gtk, Adw

from core_wrapper import CoreWrapper

class {{name_pascal}}Window(Adw.ApplicationWindow):
    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        
        self.core = CoreWrapper()
        
        self.set_title("{{greeting}}")
        self.set_default_size(600, 400)
        
        # Create header bar
        header = Adw.HeaderBar()
        
        # Create main content
        main_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        main_box.append(header)
        
        # Center content
        center_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=16)
        center_box.set_vexpand(True)
        center_box.set_valign(Gtk.Align.CENTER)
        center_box.set_halign(Gtk.Align.CENTER)
        
        # Greeting label
        self.greeting_label = Gtk.Label()
        self.greeting_label.add_css_class("title-1")
        center_box.append(self.greeting_label)
        
        # Refresh button
        refresh_button = Gtk.Button(label="Refresh")
        refresh_button.add_css_class("suggested-action")
        refresh_button.connect("clicked", self.on_refresh_clicked)
        center_box.append(refresh_button)
        
        main_box.append(center_box)
        
        self.set_content(main_box)
        
        # Load initial greeting
        self.greeting_label.set_text(self.core.greeting())
    
    def on_refresh_clicked(self, button):
        self.greeting_label.set_text(self.core.greeting())
