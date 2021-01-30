# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 2000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 2000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        if len(line) == 0:
            ob.sendLine('사용법: [몹 인덱스] 몹삭제')
            return

        ret = self.delMob(line)
        if ret == None:
            ob.sendLine('존재하지않는 몹입니다.')
            return
        del ret
        ob.sendLine('몹이 삭제되었습니다.')
        
    def delMob(self, path):
        i = path.find(':')
        if i == -1:
            return None
    
        zoneName = path[:i]
        mobName = path[i+1:]
    
        try:
            zone = Mob.Mobs[zoneName]
        except KeyError:
            return None
            
        try:
            mob = zone[mobName]
        except KeyError:
            return None
        
        zone[mobName] = None
        zone.__delitem__(mobName)
        return mob
        
