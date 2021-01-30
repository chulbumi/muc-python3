# -*- coding: utf-8 -*-

from objs.object import Object

from lib.loader import load_script, save_script

class Nickname(Object):
    
    attr = {}
    
    def __init__(self):
        self.load()
    
    def save(self):
        o = {}
        o['무림별호'] = self.attr
        try:
            with open('data/config/nickname.cfg.json', 'w', encoding='utf-8') as fp:
                save_script(fp, o)
            return True
        except:
            return False

    def load(self):
        script = load_script('data/config/nickname.json')
        self.attr = script['무림별호']
        
NICKNAME = Nickname()
        