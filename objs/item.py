# -*- coding: utf-8 -*-

import os
import glob
import time

from objs.object import Object
from objs.oneitem import Oneitem, ONEITEM
from lib.hangul import is_han, han_obj
from lib.loader import load_script, save_script
from lib.func import *

class Item(Object):
    Items = {}
    
    MagicMap = {}
    OptionName = []

    def __init__(self):
        self.timeofdrop = 0
        
    def __del__(self):
        pass
        #print 'Delete!!! ' + self.getName()
        
    def han_obj(self):
        if is_han(self['이름']) == False:
            react = self['반응이름'][0]
            #if type(react) == list:
            #    react = react[0]
            return self.getNameA() + han_obj(react)
        return self.getNameA() + han_obj(self['이름'])

    def create(self, index):
        #print(path)
        self.index = index
        self.path = 'data/item/' + index + '.json'
        scr = load_script(self.path)
        if scr == None:
            return False
        try:
            self.attr = scr['아이템정보']
        except:
            return False
            
        self.inUse = False

        """
        n = self['이름']
        s = stripANSI(n)
        if n != s:
            self['이름'] = s
            self['안시'] = n[:n.find(s)]

            self.save()

        react = self['반응이름']
        if type(react) == str:
            print('react')
            self['반응이름'] = [ react ]
        """
        #print '%s 생성!!!' % str(index)

    def save(self, mode = True):
        o = {}
        o['아이템정보'] = self.attr

        try:
            f = open(self.path, 'w')
        except:
            return False
        save_script(f, o)
        f.close()
        return True

    def view(self, ob):
        ref = '━━━━━━━━━━━━━━━━━━━━━'

        ob.sendLine('━━━━━━━━━━━━━━━━━━━━━')

        out = '◆ 이름 ▷ %s' % self.get('이름')
        out = fillSpace(ref, out)
        ob.sendLine('[0m[44m[1m[37m%s[0m[37m[40m' % out)

        out = '◆ 종류 ▷ %s' % self.get('종류')
        out = fillSpace(ref, out)
        ob.sendLine('[0m[44m[1m[37m%s[0m[37m[40m' % out)
        ob.sendLine('─────────────────────')

        #ob.sendLine(self.get('설명2'))
        desc = self['설명2']

        if type(desc) == list:
            for l in desc:
                if l.find('방어력 - ') == 0:
                    ob.sendLine('방어력 - %d' % self['방어력'])
                else:
                    ob.sendLine(l)
        else:
            ob.sendLine(desc)

        s = self.getOptionStr()
        if s != '':
            ob.sendLine(s)
        ob.sendLine('━━━━━━━━━━━━━━━━━━━━━')
        
    def drop(self, sec = None):
        if sec != None:
            self.timeofdrop = time.time() - sec
        else:
            self.timeofdrop = time.time()
        
    def update(self):
        if self.timeofdrop == 0:
            self.timeofdrop = time.time()
            return False
        curTime = time.time()
        if curTime - self.timeofdrop >= 600:
            self.destroy()
            return True
    
    def destroy(self):
        if self.isOneItem():
            ONEITEM.destroy(self.index)
        if self.env != None:
            self.env.remove(self)
            self.env = None
        del self
        
    def getNameA(self):
        return '[36m' + self.get('이름') + '[37m'
        
    def getDesc1(self):
        return self.get('설명1').replace('$아이템$', '[36m' + self.get('이름') + '[37m')
    
    def getType(self):
        return self.get('종류')

    def getUseScript(self):
        line = self['사용스크립']
        if type(line) == list:
            line = '\r\n'.join(line)
        return line.replace('$아이템$', self.get('이름'))
        
    def isOneItem(self):
        if self.checkAttr('아이템속성', '단일아이템'):
            return True
        return False
        
    def isOneThere(self):
        bRet, owner = ONEITEM.checkOneItemIndex(self.index)
        return bRet

    def delOption(self):
        if self['옵션'] != None:
            del self['아이템속성']
            del self['옵션']

    def getOption(self):
        s = self['옵션']
        if s == '':
            return None
        option = {}
        if type(s) == str:
            s = [ s ]
        for l in s:
            w = l.split()
            if len(w) != 2:
                continue
            option[w[0]] = int(w[1])
        return option
        
    def setOption(self, option):
        s = []
        for d in option:
            s.append(d + ' ' + str(option[d]))
        self['옵션'] = s

    def getOptionStr(self):
        option = self.getOption()
        if option == None:
            return ''
        s = ''
        for d in option:
            s += d + '(' + str(option[d]) + '), '
        return s[:-2]
        #return '[0m[47m[30m%s[0m[37m[40m' % s[:-2]
        

def is_item(obj):
    return isinstance(obj, Item)

def getItem(itemName):

    try:
        item = Item.Items[itemName]
    except KeyError:
        item = Item()
        ret = item.create(itemName)
        if ret == False:
            return None

        Item.Items[itemName] = item

    return item
    

def loadAllItem():
    log('아이템 로딩중... 잠시만 기다려주세요.')
    pwd = os.getcwd()
    c = 0
    os.chdir('data/item')
    files = glob.glob('*.json')
    os.chdir(pwd)
    for file in files:
        item = getItem(file[:-5])
        if item != None:
            c = c + 1
    
    log(str(c) + '개의 아이템이 로딩되었습니다.')

