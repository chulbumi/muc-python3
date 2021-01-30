# -*- coding: utf-8 -*-

from objs.object import Object

from lib.loader import load_script, save_script

class Help(Object):
    
    attr = {}
    
    def __init__(self):
        super().__init__()
        self.load()
        
    def load(self):
        help = load_script('data/config/help.json')
        self.attr = help['도움말']

    def save(self):
        o = {}
        o['도움말'] = self.attr
        try:
            f = open('data/config/help.json', 'w', encoding="utf-8")
        except:
            return False
        save_script(f, o)
        f.close()
        return True
        
HELP = Help()
        