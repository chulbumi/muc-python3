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
            ob.sendLine('☞ 사용법: [대상] [이벤트] 이벤트설정')
            return
            
        target = ob.env.findObjName(words[0])
        if target == None:
            ob.sendLine('☞ 그런 대상이 없어요!')
            return
        if target.checkEvent(words[1]):
            ob.sendLine('☞ 이미 설정되어 있습니다.')
            return
        target.setEvent(words[1])
        
        ob.sendLine('☞ [%s] 이벤트가 설정되었습니다.' % words[1])
