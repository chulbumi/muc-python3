# -*- coding: utf-8 -*-

from objs.cmd import Command
from include.ansi import *

class CmdObj(Command):
    level = 1000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        if line == '':
            ob.sendLine('☞ 운영자 명령: [아이템인덱스] 누가주나')
            return
        c = 0
        
        t = getInt(line)

        for zoneName in Mob.Mobs:
            zone = Mob.Mobs[zoneName]
            for mobName in zone:
                mob = zone[mobName]
                s = mob['아이템'] 
                if s != '':
                    s = s
                    for l in s:
                        if l.split(' ')[0] == line.strip() :
                            ob.sendLine(mob['이름'] + ' : ' + mob.index)
                s = mob['사용아이템'] 
                if s != '':
                    s = s
                    for l in s:
                        if l.split(' ')[0] == line.strip() :
                            ob.sendLine(mob['이름'] + ' : ' + mob.index)

