from {{name_snake}}_core import Core

class CoreWrapper:
    """Wrapper for Rust Core bindings"""
    
    def __init__(self):
        self.core = Core()
    
    def greeting(self):
        return self.core.greeting()
