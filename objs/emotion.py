# -*- coding: utf-8 -*-

from objs.object import Object

from lib.loader import load_script, save_script
from lib.hangul import postPosition1

class Emotion(Object):
    
    attr = {}
    
    def __init__(self):
        self.load()
        
    def load(self):
        self.attr = {}
        help = load_script('data/config/emotion.json')
        e = help['감정표현']
        for key in e:
            keys = key.split()
            for k in keys:
                self.attr[k] = e[key]
                
    def replace(self, line, sub):
        if sub == '':
            return line
        s = line.find('"')
        if s == -1:
            return line
        e = line.find('"', s + 1)
        if e == -1:
            return line
        return line[:s+1] + sub + line[e:]

    def makeScript(self, line, I, U, sub):
        line = self.replace(line, sub)
        if U == None:
            buf1 = line.replace('[공]', '당신')
            buf1 = postPosition1(buf1)
            
            buf3 = line.replace('[공]', I)
            buf3 = postPosition1(buf3)
            return buf1, '', buf3
        else:
            
            buf1 = line.replace('[공]', '당신')
            buf1 = buf1.replace('[방]', U)
            buf1 = buf1.replace('[방성]', U)
            buf1 = postPosition1(buf1)
            buf1 = postPosition1(buf1)
            buf1 = postPosition1(buf1)
            
            buf2 = line.replace('[공]', I)
            buf2 = buf2.replace('[방]', '당신')
            buf2 = buf2.replace('[방성]', '당신')
            buf2 = postPosition1(buf2)
            buf2 = postPosition1(buf2)
            buf2 = postPosition1(buf2)
            
            buf3 = line.replace('[공]', I)
            buf3 = buf3.replace('[방]', U)
            buf3 = buf3.replace('[방성]', U)
            buf3 = postPosition1(buf3)
            buf3 = postPosition1(buf3)
            buf3 = postPosition1(buf3)
            return buf1, buf2, buf3

EMOTION = Emotion()
