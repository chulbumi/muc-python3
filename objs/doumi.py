# -*- coding: utf-8 -*-
from twisted.internet import reactor

from objs.object import Object

from lib.loader import load_script, save_script
from lib.func import getInt, getNextWords
from lib.hangul import postPosition1
from objs.skill import MUGONG

class Doumi(Object):
    
    attr = {}
    
    def __init__(self):
        super().__init__()
        self.load()
        
    def load(self):
        self.attr = {}
        s = load_script('data/config/doumi.json')
        self.attr = s['도우미메인설정']

class AutoScript:
    def __init__(self):
        self.tick = 0.5
        self.lineNum = 0
        self.player = None
        self.script = None
        self.lastNum = 0
        self.name = ''

    def start(self, script, player):
        self.tick = 0.5
        self.lineNum = 0
        self.player = player
        self.script = script
        self.lastNum = len(self.script)
        self.run()
        self.name = ''
        
    def run(self):
        if self.player is None:
            return
        print_line = False
        while True:
            if self.lineNum >= self.lastNum:
                self.player.save()
                self.player.showNotice()
                return
            line = self.script[self.lineNum]
            if line == '':
                self.player.sendLine('')
            elif line[0] == '$':
                l = line.strip()
                nw = getNextWords(l)
                func = l.split()[0]
                if l == '$출력시작':
                    print_line = True
                elif l == '$출력끝':
                    print_line = False
                    self.lineNum += 1
                    continue
                elif func == '$틱':
                    tick = getInt(nw)
                    if tick != 0:
                        self.tick = tick * 0.1 * 1.5
                    self.lineNum += 1
                    continue
                elif func == '$키입력':
                    key = l.find(':')
                    if key == -1:
                        self.player.input_to(self.player.pressEnter1)
                    else:
                        self.player.input_to(self.player.getKeyInput, l[key + 1:])
                    self.lineNum += 1
                    return
                elif func == '$이름획득':
                    self.lineNum += 1
                    self.player.write('무림존함ː')
                    self.player.input_to(self.player.getNewname)
                    return
                elif func == '$암호획득':
                    self.lineNum += 1
                    self.player.write('존함암호ː')
                    self.player.input_to(self.player.getNewpass)
                    return
                elif func == '$성별획득':
                    self.lineNum += 1
                    self.player.write('성별(남/여)ː')
                    self.player.input_to(self.player.getSex)
                    return
            else:
                msg = line.replace('[공]', self.player['이름'])
                self.player.sendLine(postPosition1(msg))
            self.lineNum += 1
            if print_line:
                continue
            reactor.callLater(self.tick, self.run)
            break

DOUMI = Doumi()

        
