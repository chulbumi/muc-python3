# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 2000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 2000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        if line == '':
            ob.sendLine('☞ 사용법: [방번호] 방제거')
            return
        
        ret = self.delRoom(line)
        if ret == None:
            ob.sendLine('존재하지않는 방입니다.')
            return
        del ret
        ob.sendLine('방이 제거되었습니다.')
        
    def delRoom(self, path):
    
        i = path.find(':')
        if i == -1:
            return None
    
        zoneName = path[:i]
        roomName = path[i+1:]
    
        try:
            zone = Room.Zones[zoneName]
        except KeyError:
            return None
            
        try:
            room = zone[roomName]
        except KeyError:
            return None
        
        zone[roomName] = None
        zone.__delitem__(roomName)
        return room
