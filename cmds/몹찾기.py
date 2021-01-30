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
            ob.sendLine('☞ 운영자 명령: [몹이름]|[몹종류] 몹찾기\r\n ex) 산적 몹찾기 or 6 몹찾기')
            return
        c = 0
        
        t = getInt(line)

        for zoneName in Mob.Mobs:
            zone = Mob.Mobs[zoneName]
            for mobName in zone:
                mob = zone[mobName]
                if t != 0:
                    if mob['몹종류'] == t:
                        c += 1
                        ob.sendLine('%s(%s) : %s'% (mob.getNameA(), mob.index, mob['위치']))
                else:
                    if mob['이름'].find(line) != -1:
                        c += 1
                        ob.sendLine('%s(%s) : %s'% (mob.getNameA(), mob.index, mob['위치']))
        if c == 0:
            ob.sendLine('☞ 찾으시는 몹이 없습니다.')
        

