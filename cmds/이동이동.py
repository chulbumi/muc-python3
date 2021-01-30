# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 1000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        if len(line) == 0:
            ob.sendLine('☞ 사용법: [존이름:맵번호] 이동')
            return
        room = getRoom(line)
        if room == None:
            ob.sendLine('* 이동 실패!!!')
            return
        
        if room == ob.env:
            ob.sendLine('☞ 같은 자리에요. ^^')
            return
            
        #ob.act = ACT_STAND
        ob.clearTarget()
        
        ob.env.remove(ob)
        ob.env = room
        room.insert(ob)
        
        #reactor.callLater(2, ob.do_command, '동')
        #ob.parse_command('동')
