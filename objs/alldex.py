# -*- coding: utf-8 -*-
from twisted.internet import reactor

from objs.object import Object

from lib.loader import load_script, save_script
from lib.func import getInt
from lib.hangul import postPosition1
from objs.skill import MUGONG

class AllDex1000(Object):
    
    attr = {}
    
    def __init__(self):
        self.load()
        
    def load(self):
        self.attr = {}
        s = load_script('data/config/alldex.json')
        self.attr = s['올숙천스크립트']
    
class autoScript():
    def start(self, script, player):
        self.tick = 0.5
        self.lineNum = 0
        self.player = player
        self.script = script
        self.lastNum = len(self.script)
        self.run()
        self.name = ''
        
    def run(self):
        if self.player == None:
            return
        from objs.player import Player
        printLine = False
        while(True):
            if self.lineNum >= self.lastNum:
                self.player.save()
                self.player.showNotice()
                return
            line = self.script[self.lineNum]
            if line == '':
                self.player.sendLine('')
            elif line[0] == '$':
                l = line.strip()
                if  l == '$출력시작':
                    printLine = True
                elif l == '$출력끝':
                    printLine = False
                    self.lineNum += 1
                    continue
                elif l[:3] == '$틱':
                    tick = getInt(l[4:])
                    if tick != 0:
                        self.tick = tick * 0.1 * 1.5
                    self.lineNum += 1
                    continue
                elif l[:7] == '$키입력':
                    key = l.find(':')
                    if key == -1:
                        self.player.input_to(self.player.pressEnter1)
                    else:
                        self.player.input_to(self.player.getKeyInput, l[key + 1:])
                    self.lineNum += 1
                    return
                elif l[:9] == '$이름획득':
                    self.lineNum += 1
                    self.player.write('무림존함ː')
                    self.player.input_to(self.player.getNewname)
                    return
                elif l[:9] == '$암호획득':
                    self.lineNum += 1
                    self.player.write('존함암호ː')
                    self.player.input_to(self.player.getNewpass)
                    return
                elif l[:9] == '$성별획득':
                    self.lineNum += 1
                    self.player.write('성별(남/여)ː')
                    self.player.input_to(self.player.getSex)
                    return
            else:
                msg = line.replace('[공]', self.player['이름'])
                self.player.sendLine(postPosition1(msg))
            self.lineNum += 1
            if printLine == True:
                continue
            reactor.callLater(self.tick, self.run)
            break

DOUMI = Doumi()

        
