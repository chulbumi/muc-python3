# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 1000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        if line == '':
            ob.sendLine('☞ 사용법: [대상] 이벤트')
            return
            
        target = ob.env.findObjName(line)
        if target == None:
            ob.sendLine('☞ 그런 대상이 없어요!')
            return
        for l in target['이벤트설정리스트']:
            ob.sendLine(l)

