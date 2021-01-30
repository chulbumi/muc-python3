# -*- coding: utf-8 -*-

from objs.object import Object

from lib.loader import load_script, save_script

class Oneitem(Object):
    
    attr = {}
    
    def __init__(self):
        script = load_script('data/config/oneitem_index.json')
        self.index = script['단일아이템인덱스']
        
        script = load_script('data/config/oneitem.json')
        attr = script['단일아이템']
        
        for att in self.index:
            self.index[att] = str(self.index[att])
            
        for att in attr:
            words = attr[att].split()
            if len(words) > 1 and words[1] == '버림':
                continue
            self.attr[att] = attr[att]
        
    def save(self):
        o = {}
        o['단일아이템'] = self.attr
        try:
            f = open('data/config/oneitem.json', 'w')
        except:
            return False
        save_script(f, o)
        f.close()
        return True
        
    def checkOneItemName(self, name):
        # name: index
        # index: owner
        if name in self.index:
            index = self.index[name]
        else:
            print('not in index')
            return False, None
        if index in self.attr:
            return True, self[index].split()[0]
        return False, None
        
    def checkOneItemIndex(self, index):
        if index in self.attr:
            return True, self[index].split()[0]
        return False, None
        
    def have(self, index, name):
        self[index] = name
        self.save()
            
    def drop(self, index, name):
        self[index] = name + ' 버림'
        self.save()

    def drop2(self, index, name):
        self[index] = name + ' 떨굼'
        self.save()
            
    def keep(self, index, name):
        self[index] = name + ' 보관'
        self.save()
            
    def destroy(self, index):
        if index in self.attr:
            self.attr.__delitem__(index)
            self.save()
    
    def getName(self, index):
        for name in self.index:
            if self.index[name] == index:
                return name
        return ''
        
    def list(self):
        msg = ''
        for index in self.attr:
            owner = self[index]
            name = self.getName(index)
            msg += '%-20s : %s\r\n' % (name, owner)
        return msg
            
    def clear(self):
        self.attr = {}
        self.save()

ONEITEM = Oneitem()
        
