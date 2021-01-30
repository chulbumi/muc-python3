# -*- coding: utf-8 -*-

from objs.object import Object

from lib.loader import load_script, save_script

class Script(Object):
    
    attr = {}
    
    def __init__(self):
        self.load()
        
    def load(self):
        self.attr = {}
        script = load_script('data/config/script.json')
        self.attr = script['메인설정']
        for attr in self.attr:
            self.attr[attr] = self.attr[attr]

SCRIPT = Script()

        