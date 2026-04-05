import gi

gi.require_version('Gtk', '4.0')
gi.require_version('Adw', '1')
from gi.repository import Gtk, Adw, GLib

from ffi_wrapper import FfiWrapper

class MainWindow(Adw.ApplicationWindow):
    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        
        self.ffi = FfiWrapper()
        self.items = []
        
        self.set_title("Today")
        self.set_default_size(600, 450)
        
        # Create header bar
        header = Adw.HeaderBar()
        add_button = Gtk.Button(icon_name="list-add-symbolic")
        add_button.connect("clicked", self.on_add_clicked)
        header.pack_end(add_button)
        
        # Create main content
        main_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        main_box.append(header)
        
        # Stats cards
        stats_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        stats_box.set_margin_start(20)
        stats_box.set_margin_end(20)
        stats_box.set_margin_top(20)
        stats_box.set_margin_bottom(20)
        stats_box.set_homogeneous(True)
        
        self.total_label = self.create_stat_card("Total", "0")
        self.active_label = self.create_stat_card("Active", "0")
        self.done_label = self.create_stat_card("Done", "0")
        
        stats_box.append(self.total_label)
        stats_box.append(self.active_label)
        stats_box.append(self.done_label)
        
        main_box.append(stats_box)
        
        # Tasks list
        scrolled = Gtk.ScrolledWindow()
        scrolled.set_vexpand(True)
        
        self.list_box = Gtk.ListBox()
        self.list_box.set_margin_start(20)
        self.list_box.set_margin_end(20)
        self.list_box.set_margin_bottom(20)
        self.list_box.add_css_class("boxed-list")
        
        scrolled.set_child(self.list_box)
        main_box.append(scrolled)
        
        self.set_content(main_box)
        self.refresh_items()
    
    def create_stat_card(self, title, value):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
        box.set_margin_top(16)
        box.set_margin_bottom(16)
        
        value_label = Gtk.Label(label=value)
        value_label.add_css_class("title-1")
        
        title_label = Gtk.Label(label=title)
        title_label.add_css_class("dim-label")
        
        box.append(value_label)
        box.append(title_label)
        
        frame = Gtk.Frame()
        frame.set_child(box)
        
        return box
    
    def refresh_items(self):
        self.items = self.ffi.get_items()
        
        # Update stats
        total = len(self.items)
        active = sum(1 for item in self.items if not item['completed'])
        done = sum(1 for item in self.items if item['completed'])
        
        self.total_label.get_first_child().set_label(str(total))
        self.active_label.get_first_child().set_label(str(active))
        self.done_label.get_first_child().set_label(str(done))
        
        # Clear and rebuild list
        while True:
            row = self.list_box.get_row_at_index(0)
            if row is None:
                break
            self.list_box.remove(row)
        
        for item in self.items:
            row = self.create_task_row(item)
            self.list_box.append(row)
    
    def create_task_row(self, item):
        row = Gtk.ListBoxRow()
        
        box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)
        box.set_margin_start(12)
        box.set_margin_end(12)
        box.set_margin_top(12)
        box.set_margin_bottom(12)
        
        # Checkbox
        check = Gtk.CheckButton()
        check.set_active(item['completed'])
        check.connect("toggled", self.on_toggle_clicked, item['id'])
        
        # Title
        label = Gtk.Label(label=item['title'])
        label.set_hexpand(True)
        label.set_halign(Gtk.Align.START)
        
        if item['completed']:
            label.add_css_class("dim-label")
        
        box.append(check)
        box.append(label)
        
        row.set_child(box)
        return row
    
    def on_add_clicked(self, button):
        dialog = Adw.MessageDialog(
            transient_for=self,
            heading="New Task",
            body="Enter task name:"
        )
        
        entry = Gtk.Entry()
        entry.set_margin_start(12)
        entry.set_margin_end(12)
        entry.set_margin_top(12)
        entry.set_margin_bottom(12)
        
        dialog.set_extra_child(entry)
        dialog.add_response("cancel", "Cancel")
        dialog.add_response("add", "Add")
        dialog.set_response_appearance("add", Adw.ResponseAppearance.SUGGESTED)
        
        def on_response(dialog, response):
            if response == "add":
                title = entry.get_text()
                if title:
                    import uuid
                    self.ffi.add_item(str(uuid.uuid4()), title)
                    self.refresh_items()
        
        dialog.connect("response", on_response)
        dialog.present()
    
    def on_toggle_clicked(self, check, item_id):
        self.ffi.toggle_item(item_id)
        self.refresh_items()
