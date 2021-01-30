# -*- coding: utf-8 -*-

from objs.object import Object
from lib.loader import load_script

class Config(Object):
    
    attr = {}
    
    def __init__(self):
        self.load()
    
    def load(self):
        self.attr = {}
        scr = load_script('data/config/murim.json')
        self.attr = scr['메인설정']
        
MAIN_CONFIG = Config()
