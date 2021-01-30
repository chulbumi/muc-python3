# -*- coding: utf-8 -*-
from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
            
        if line == '':
            ob.sendLine('몹이름 존이름 추적')
            return
        words = line.split()

        if len(words) < 2 :
            ob.sendLine('몹이름 존이름 추적')
            return

        try:
            zone = Room.Zones[words[1]]
        except KeyError:
            ob.sendLine('그런 존은 없어요!')
            return
        for room in zone:
            r = getRoom(words[1] + ':' + room)
            
            for obj in r.objs:
                if is_mob(obj):
                    if obj['이름'] == words[0]:
                        ob.sendLine(room)
                        return 
        ob.sendLine('못찾았음')
    
    
    

