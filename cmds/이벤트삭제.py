# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 1000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        words = line.split(None, 1)
        if line == '' or len(words) < 2:
            ob.sendLine('☞ 사용법: [대상] [이벤트] 이벤트삭제')
            return
            
        target = ob.env.findObjName(words[0])
        if target == None:
            ob.sendLine('☞ 그런 대상이 없어요!')
            return
        if target.checkEvent(words[1]) == False:
            ob.sendLine('☞ [%s] 이벤트는 설정되어있지 않습니다.'% words[1])
            return
        target.delEvent(words[1])
        
        ob.sendLine('☞ [%s] 이벤트가 삭제되었습니다.' % words[1])
